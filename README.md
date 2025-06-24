bazel-dep-reduce
================

Dependency Reduction for Bazel

## Static Dependency Analysis via `depreduce`

`depreduce` is a novel tool for static dependency analysis and reduction proposed by ourself.

### Get Dependency Graph from Bazel Query

`depreduce` can parse the dependency graph in the XML format output by Bazel Query:

```sh
bazel query "deps(//...)" --notool_deps --output xml
```

## Dynamic Dependency Analysis via `buildfuzz`

`buildfuzz` is basically the reproduction of the build fuzz testing algorithm proposed by 
[`mkcheck`] (https://github.com/nandor/mkcheck) with a new feature:

1. Use **custom touchers** instead of the `touch` file operation, 
   which will **CHANGE** the file content but not affect the original functionality.
1. **Restore** touched file content after every round.
1. **Rebuild** the project before every round.

You may wonder why we bother this -- changing the source code instead of just touching them. 
The reason is that Bazel has a very powerful dirtiness checking logic, which means, simply touching a file
will not cause Bazel to rebuild.

Do you think custom touchers just add comments into
the source code? If so, you are wrong. We have to make
custom touchers modify the source code that could further
change the object file. Otherwise, we cannot track the 
dependencies between the object files and the linked artifacts
such as the executables. Bazel is too smart to re-link the
object files without real changes of them. 
See [Skyframe - Bazel] for details.

So, what we do with custom touchers is actually adding a dummy
static thing such as static function into the source code.
But it introduces some risks such as unused function warnings, 
which may cause build failures if the project has settings 
to treat warnings as errors. There are also risks like conflicted symbols,
invalid syntaxes in some special contexts (e.g. a header file used as a database) 
and so on.

What's more, even if we added a new function into the source code, there could be 
a chance that the change stop propagating to its dependents 
(e.g. the unused function might be pruned in the object file).
In such cases, we could lose the tracking of dependencies and get inaccurate results.

Anyway, by using custom touchers, we do make it possible to apply the build fuzz 
testing method to Bazel build system.

### How to Run `buildfuzz`

```sh
buildfuzz --input examples/simple-cxx-project \
    --artifact examples/simple-cxx-project/bazel-bin \
    --command buildfuzz/src/test_data/build.sh \
    --output result_deps.log
```

The result is a JSONL file like below.

```
["a.o",["a.h","a.c"]]
["b.o",["b.h","b.c"]]
["main.o",["main.c","a.h","b.h"]]
["main",["main.o","a.o","b.o"]]
```

## Dynamic Dependency Analaysis via `strace`

`strace_parser` is basically the reproduction of [`buildfs`] (https://github.com/theosotr/buildfs) with some improvements:

1. **`stat`/`lstat`/`statfs` syscalls were ignored** because we don't know if the accessed file truly exists, 
   and they are mostly used to detect file changes, i.e., usually not a real sign of file consumption.
1. **Syscalls returning -1** will be ignored, because they failed mostly for inexistent files.
1. **`clone3` syscall was added** for tracing. Otherwise there will be many missing `Newproc` operations.
1. **`--decode-pids=pidns,comm` was added** as the arguments of `strace`, to resolve the pid within a separate namespace, which is the case of Bazel sandboxing. Otherwise, the pids returned by `clone` or `fork` cannot match the pids traced by `strace` in its own namespace, which prevents us from tracing the process relationship correctly.
1. A **virtual filesystem** was implemented to track symlinks. Bazel creates lots of symlinks because of sandboxing.
1. A **`to_link` operation was added** to DSL (IR) to support tracking symlinks.


Though [`buildfs`] claims their approach is applicable to other build systems including Bazel.
The fact is, without our efforts, it is really hard to apply it to Bazel.

See [Sandboxing - Bazel] for details about the sandboxing mechanism in Bazel.

### About Redundant Dependency Detection

[`buildfs`] does not support to detect redundant dependencies. 
This is because when you specified a dependency in build script,
even if it was not used in the source code, the compiler or linker
will still access the dependency file to finish compilation or linking.

For example, suppose `main.cpp` doesn't include `a.h`, 
but we specify `liba` as a dependency of `main` executable.
When we compile `main.cpp` to `main.o`, the dynamic analysis could work
here, because the compiler will only access all headers included in the `main.cpp`
and does not need to access any other manually-specified dependencies.
However, when it comes to linking, i.e. `main.o` to `main`, all specified
dependencies such as `liba` will be passed to the linker command-line, leading to the file
access on those redundant dependncies. Nothing can be done by dynamic analysis to catch them.

It happens to other programming languages too, especially to those languages without header, such as Java.

[`BuildChecker`] uses the same dynamic approach to detect redundant dependencies.
But in fact, it only supports GNU Make, of which the build dependencies
are based on file instead of target and are more fine-grained. 
So, it could be able to find such dependency errors, but still has
a lot of opportunities to miss redundancy errors.


### How to Run `strace`

```sh
bazel clean --expunge
bazel shutdown
strace -s 300 \
    -f \
    -e access,chdir,chmod,chown,clone,clone3,close,dup,dup2,dup3,execve,fchdir,fchmodat,fchownat,fcntl,fork,getxattr,getcwd,lchown,lgetxattr,lremovexattr,lsetxattr,link,linkat,mkdir,mkdirat,mknod,open,openat,readlink,readlinkat,removexattr,rename,renameat,rmdir,symlink,symlinkat,unlink,unlinkat,utime,utimensat,utimes,vfork,write,writev \
    --decode-pids=pidns,comm \
    -o strace.log \
    bash ../build.sh
```

### How to use `strace_parser`

```sh
strace_parser -i examples/simple-java-project/strace.log -c examples/simple-java-project -o result_deps.log
```

The result is a JSONL file like below.

```
["a.o",["a.h","a.c"]]
["b.o",["b.h","b.c"]]
["main.o",["main.c","a.h","b.h"]]
["main",["main.o","a.o","b.o"]]
```


[`buildfs`]: https://dl.acm.org/doi/10.1145/3428212
[`BuildChecker`]: https://ieeexplore.ieee.org/document/10981616
[`mkcheck`]: https://ieeexplore.ieee.org/document/8812082
[Skyframe - Bazel]: https://bazel.build/reference/skyframe
[Sandboxing - Bazel]: https://bazel.build/docs/sandboxing
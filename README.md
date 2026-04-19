# DepReduce

Automated Dependency Optimization for Artifact-Based Build Systems

See https://github.com/xuhongxu96/paper-depreduce for more details.

## Overview

Currently supports the following build systems:

- Bazel
- Buck
- Cargo (for Rust)

See [`depreduce`] for implementation details.

### How to Use [`depreduce`]

```sh
depreduce \
    -w <WORKSPACE_DIR> \
    -c <BUILD_SCRIPT> \
    --config <depreduce.toml> \
    --build-system <bazel|buck|cargo>
```

See scripts in [scripts/experiments](scripts/experiments)
or [scripts/buck](scripts/buck) for more examples.

#### Example Build Script

```bash
set -e
bazel build //...
bazel test --notest_keep_going -- //...
```

#### Example `depreduce.toml`

See [depreduce.toml](depreduce.toml)

---

## Extensions to Related Tools

We also extend two related tools to support Bazel build system and
investigate their effectiveness on redundant dependency detection.
It turns out that neither of them can do it well.

### Extending [`mkcheck`] to [`buildfuzz`]

[`buildfuzz`] is the reproduction of the build fuzzing algorithm proposed by
[`mkcheck`] (https://github.com/nandor/mkcheck) with new features:

1. Use **custom touchers** instead of the `touch` file operation,
   which will **CHANGE** the file content but not affect the original functionality.
1. Use **SHA256** instead of timestamp to detect file changes.
1. **Restore** touched file content after every round.
1. **Rebuild** the project before every round.

#### How to Run [`buildfuzz`]

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

### Extending [`buildfs`] to [`strace_parser`]

[`strace_parser`] is the reproduction of [`buildfs`] (https://github.com/theosotr/buildfs) with some improvements:

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

#### How to Run `strace`

```sh
bazel clean --expunge
bazel shutdown
strace -s 300 \
    -f \
    -e access,chdir,chmod,chown,clone,clone3,close,dup,dup2,dup3,execve,fchdir,fchmodat,fchownat,fcntl,fork,getxattr,getcwd,lchown,lgetxattr,lremovexattr,lsetxattr,link,linkat,mkdir,mkdirat,mknod,open,openat,readlink,readlinkat,removexattr,rename,renameat,rmdir,symlink,symlinkat,unlink,unlinkat,utime,utimensat,utimes,vfork,write,writev \
    --decode-pids=pidns,comm \
    -o strace.log \
    bash ../build_for_strace.sh
```

#### How to use [`strace_parser`]

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
[`buildfuzz`]: buildfuzz
[`strace_parser`]: strace_parser
[`depreduce`]: depreduce
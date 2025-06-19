bazel-dep-reduce
================

Dependency Reduction for Bazel

## How to Run `strace`

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

## How to use `strace_parser`

```sh
strace_parser \
    -i /data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project/strace.log \
    -c /data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project \
    -o result_deps.log
```

Results look like:

```
<id>: <Path>
  -> <id>: <Dependency Path>
  -> <id>: <Dependency Path>
  -> <id>: <Dependency Path>

<id>: <Path>
  -> <id>: <Dependency Path>
  -> <id>: <Dependency Path>

...
```
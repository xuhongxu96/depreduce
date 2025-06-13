bazel-dep-reduce
================

Dependency Reduction for Bazel

## How to Run `strace`

```sh
bazel clean --expunge
bazel shutdown
strace -s 300 -f -e access,chdir,chmod,chown,clone,close,dup,dup2,dup3,execve,fchdir,fchmodat,fchownat,fcntl,fork,getxattr,getcwd,lchown,lgetxattr,lremovexattr,lsetxattr,lstat,link,linkat,mkdir,mkdirat,mknod,open,openat,readlink,readlinkat,removexattr,rename,renameat,rmdir,stat,statfs,symlink,symlinkat,unlink,unlinkat,utime,utimensat,utimes,vfork,write,writev -o strace.log bash ../build.sh
```
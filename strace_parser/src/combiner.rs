use crate::syscall_line::*;
use std::collections::HashMap;

pub struct SyscallLineCombiner<T>
where
    T: Iterator<Item = SyscallLine>,
{
    iter: T,
    buffered_unfinished_lines: HashMap<u64, UnfinishedSyscallDesp>,
}

impl<T> Iterator for SyscallLineCombiner<T>
where
    T: Iterator<Item = SyscallLine>,
{
    type Item = SyscallDesp;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(line) = self.iter.next() {
                match line {
                    SyscallLine::Full(syscall_desp) => return Some(syscall_desp),
                    SyscallLine::Unfinished(unfinished_syscall_desp) => {
                        self.buffered_unfinished_lines
                            .insert(unfinished_syscall_desp.pid, unfinished_syscall_desp);
                        continue;
                    }
                    SyscallLine::Resumed(resumed_syscall_desp) => {
                        if let Some(unfinished) = self
                            .buffered_unfinished_lines
                            .get(&resumed_syscall_desp.pid)
                        {
                            assert_eq!(unfinished.syscall, resumed_syscall_desp.syscall);
                            return Some(SyscallDesp {
                                pid: resumed_syscall_desp.pid,
                                syscall: resumed_syscall_desp.syscall,
                                args: format!(
                                    "{}{}",
                                    unfinished.partial_args, resumed_syscall_desp.partial_args
                                ),
                                ret: resumed_syscall_desp.ret,
                                line_no: unfinished.line_no,
                            });
                        } else {
                            continue;
                        }
                    }
                    SyscallLine::Error(_) => continue,
                }
            } else {
                return None;
            }
        }
    }
}

pub fn combine_syscall_lines(
    lines: impl IntoIterator<Item = SyscallLine>,
) -> SyscallLineCombiner<impl Iterator<Item = SyscallLine>> {
    SyscallLineCombiner {
        iter: lines.into_iter(),
        buffered_unfinished_lines: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::combine_syscall_lines;
    use crate::syscall_line::*;

    #[test]
    fn test_combine() {
        let res: Vec<_> = combine_syscall_lines(
            vec![
                SyscallLine::Full(SyscallDesp {
                    pid: 815823,
                    syscall: "newfstatat".to_string(),
                    args: "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE\", 0xc00017f488, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 1
                }),
                SyscallLine::Full(SyscallDesp {
                    pid: 815823,
                    syscall: "newfstatat".to_string(),
                    args: "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE.bazel\", 0xc00017f558, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 2
                }),
                SyscallLine::Resumed(ResumedSyscallDesp {
                    pid: 815824,
                    syscall: "nanosleep".to_string(),
                    partial_args: "NULL)".to_string(),
                    ret: "0".to_string(),
                    line_no: 3
                }),
                SyscallLine::Unfinished(UnfinishedSyscallDesp {
                    pid: 815823,
                    syscall: "newfstatat".to_string(),
                    partial_args: "(AT_FDCWD, \"/data/h445xu/repo/WORKSPACE\", ".to_string(),
                    line_no: 4
                }),
                SyscallLine::Unfinished(UnfinishedSyscallDesp {
                    pid: 815824,
                    syscall: "nanosleep".to_string(),
                    partial_args: "({tv_sec=0, tv_nsec=20000}, ".to_string(),
                    line_no: 5
                }),
                SyscallLine::Resumed(ResumedSyscallDesp {
                    pid: 815823,
                    syscall: "newfstatat".to_string(),
                    partial_args: "0xc00017f628, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 6
                })
            ]
        ).collect();

        let expected = vec![
            SyscallDesp {
                pid: 815823,
                syscall: "newfstatat".to_string(),
                args:
                    "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE\", 0xc00017f488, 0)"
                        .to_string(),
                ret: "-1 ENOENT (No such file or directory)".to_string(),
                line_no: 1
            },
            SyscallDesp { 
                pid: 815823, 
                syscall: "newfstatat".to_string(), 
                args: "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE.bazel\", 0xc00017f558, 0)".to_string(), 
                ret: "-1 ENOENT (No such file or directory)".to_string(), 
                line_no: 2 
            }, 
            SyscallDesp { 
                pid: 815823,  
                syscall: "newfstatat".to_string(), 
                args: "(AT_FDCWD, \"/data/h445xu/repo/WORKSPACE\", 0xc00017f628, 0)".to_string(), 
                ret: "-1 ENOENT (No such file or directory)".to_string(), 
                line_no: 4 
            },
        ];

        assert_eq!(res, expected);
    }

    #[test]
    fn test_large_file() {
        use crate::{parser::parse_strace_from_path};
        use std::fs::{self};
        use std::io::Write;
        use std::path::Path;

        let data_path = Path::new(file!())
            .parent()
            .unwrap()
            .join("test_data/strace.log");
        let expected_data_path = Path::new(file!())
            .parent()
            .unwrap()
            .join("test_data/strace.combined.expected.out");
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(expected_data_path)
            .unwrap();
        for line in combine_syscall_lines(parse_strace_from_path(
            data_path.to_str().unwrap(),
        )) {
            writeln!(f, "{:?}", line).unwrap();
        }
    }
}

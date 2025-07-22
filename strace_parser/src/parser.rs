use crate::syscall_line::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct StraceParseResult<R> {
    reader: BufReader<R>,

    line: String,
    line_no: u32,
}

mod line_utils {
    use lazy_static::lazy_static;
    use regex::Regex;

    use super::{ResumedSyscallDesp, SyscallDesp, UnfinishedSyscallDesp};

    pub fn parse_unfinished_line(line: &str, line_no: u32) -> Option<UnfinishedSyscallDesp> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"(\d+)[<](.*?)[>] ([a-z0-9_?]+)(.*)[ ][<]unfinished[ ][.][.][.][>]")
                    .unwrap();
        };

        if !line.ends_with("<unfinished ...>") {
            None
        } else if let Some(captures) = RE.captures(line) {
            Some(UnfinishedSyscallDesp {
                pid: captures[1].parse().unwrap(),
                cmd: captures[2].to_string(),
                syscall: captures[3].to_string(),
                partial_args: captures[4].trim_start().to_string(),
                line_no,
            })
        } else {
            None
        }
    }

    pub fn parse_resumed_line(line: &str, line_no: u32) -> Option<ResumedSyscallDesp> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"(\d+)[<](.*?)[>] [<][.][.][.][ ]([a-z0-9_?]+) resumed[>](.*)\s+[=]\s+(.*?)$"
            )
            .unwrap();
        };

        if !line.contains(" resumed>") || !line.contains("<... ") {
            None
        } else if let Some(captures) = RE.captures(line) {
            Some(ResumedSyscallDesp {
                pid: captures[1].parse().unwrap(),
                cmd: captures[2].to_string(),
                syscall: captures[3].to_string(),
                partial_args: captures[4].trim_end().to_string(),
                ret: captures[5].to_string(),
                line_no,
            })
        } else {
            None
        }
    }

    pub fn parse_full_line(line: &str, line_no: u32) -> Option<SyscallDesp> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"(\d+)[<](.*?)[>] ([a-z0-9_?]+)(.*)\s+[=]\s+(.*?)$").unwrap();
        };

        if let Some(captures) = RE.captures(line) {
            Some(SyscallDesp {
                pid: captures[1].parse().unwrap(),
                cmd: captures[2].to_string(),
                resumed_cmd: None,
                syscall: captures[3].to_string(),
                args: captures[4].trim().to_string(),
                ret: captures[5].to_string(),
                line_no,
            })
        } else {
            None
        }
    }
}

impl<R> Iterator for StraceParseResult<R>
where
    R: std::io::Read,
{
    type Item = SyscallLine;

    fn next(&mut self) -> Option<Self::Item> {
        self.line.clear();
        if let Ok(sz) = self.reader.read_line(&mut self.line) {
            if sz == 0 {
                return None;
            }

            self.line_no += 1;

            let line = self.line.trim();
            if line.contains("+++ exited with ") {
                Some(SyscallLine::Error(ErrorSyscallDesp {
                    line_no: self.line_no,
                    line: self.line.clone(),
                    msg: "exit".to_string(),
                }))
            } else if (line.starts_with("+++") && line.ends_with("+++"))
                || (line.starts_with("---") && line.ends_with("---"))
            {
                Some(SyscallLine::Error(ErrorSyscallDesp {
                    line_no: self.line_no,
                    line: self.line.clone(),
                    msg: "others".to_string(),
                }))
            } else if let Some(unfinished) = line_utils::parse_unfinished_line(line, self.line_no) {
                Some(SyscallLine::Unfinished(unfinished))
            } else if let Some(resumed) = line_utils::parse_resumed_line(line, self.line_no) {
                Some(SyscallLine::Resumed(resumed))
            } else if let Some(full) = line_utils::parse_full_line(line, self.line_no) {
                Some(SyscallLine::Full(full))
            } else {
                Some(SyscallLine::Error(ErrorSyscallDesp {
                    line_no: self.line_no,
                    line: self.line.clone(),
                    msg: "Failed to parse".to_string(),
                }))
            }
        } else {
            None
        }
    }
}

pub fn parse_strace_from_path(path: &str) -> StraceParseResult<File> {
    StraceParseResult {
        reader: BufReader::new(File::open(path).unwrap()),
        line: String::new(),
        line_no: 0,
    }
}

pub fn parse_strace_from_content(content: &[u8]) -> StraceParseResult<&[u8]> {
    StraceParseResult {
        reader: BufReader::new(content),
        line: String::new(),
        line_no: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::line_utils::*;
    use super::*;
    use utils::*;

    #[test]
    fn test_parse_unfinished_line() {
        assert_eq!(
            parse_unfinished_line("815827<a> sigaltstack(NULL,  <unfinished ...>", 0),
            Some(UnfinishedSyscallDesp {
                pid: 815827,
                cmd: "a".to_string(),
                syscall: "sigaltstack".to_string(),
                partial_args: "(NULL, ".to_string(),
                line_no: 0,
            })
        );
        assert_eq!(
            parse_unfinished_line("815827<a> sigaltstack <unfinished ...>", 0),
            Some(UnfinishedSyscallDesp {
                pid: 815827,
                cmd: "a".to_string(),
                syscall: "sigaltstack".to_string(),
                partial_args: "".to_string(),
                line_no: 0,
            })
        );
    }

    #[test]
    fn test_parse_resumed_line() {
        assert_eq!(
            parse_resumed_line("815827<a> <... gettid resumed>)            = 815827", 0),
            Some(ResumedSyscallDesp {
                pid: 815827,
                cmd: "a".to_string(),
                syscall: "gettid".to_string(),
                partial_args: ")".to_string(),
                ret: "815827".to_string(),
                line_no: 0,
            })
        );
        assert_eq!(
            parse_resumed_line("815827<a> <... gettid resumed>         = 815827", 0),
            Some(ResumedSyscallDesp {
                pid: 815827,
                cmd: "a".to_string(),
                syscall: "gettid".to_string(),
                partial_args: "".to_string(),
                ret: "815827".to_string(),
                line_no: 0,
            })
        );
    }

    #[test]
    fn test_parse_full_line() {
        assert_eq!(
            parse_full_line(
                "815824<time> nanosleep({tv_sec=0, tv_nsec=20000}, NULL) = 0",
                0
            ),
            Some(SyscallDesp {
                pid: 815824,
                cmd: "time".to_string(),
                resumed_cmd: None,
                syscall: "nanosleep".to_string(),
                args: "({tv_sec=0, tv_nsec=20000}, NULL)".to_string(),
                ret: "0".to_string(),
                line_no: 0,
            })
        );
    }

    #[test]
    fn test_parse_empty() {
        let res: Vec<_> = parse_strace_from_content("".as_bytes()).collect();
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_parse() {
        let data_path = get_test_data_path!("parser/test_strace.in");
        let res: Vec<_> = parse_strace_from_path(data_path.to_str().unwrap()).collect();
        assert_eq!(
            res,
            vec![
                SyscallLine::Full(SyscallDesp {
                    pid: 815823,
                    cmd: "a".to_string(),
                    resumed_cmd: None,
                    syscall: "newfstatat".to_string(),
                    args: "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE\", 0xc00017f488, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 1
                }),
                SyscallLine::Full(SyscallDesp {
                    pid: 815823,
                    cmd: "a".to_string(),
                    resumed_cmd: None,
                    syscall: "newfstatat".to_string(),
                    args: "(AT_FDCWD, \"/data/h445xu/repo/bazel-dep-reduce/WORKSPACE.bazel\", 0xc00017f558, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 2
                }),
                SyscallLine::Resumed(ResumedSyscallDesp {
                    pid: 815824,
                    cmd: "b".to_string(),
                    syscall: "nanosleep".to_string(),
                    partial_args: "NULL)".to_string(),
                    ret: "0".to_string(),
                    line_no: 3
                }),
                SyscallLine::Unfinished(UnfinishedSyscallDesp {
                    pid: 815823,
                    cmd: "a".to_string(),
                    syscall: "newfstatat".to_string(),
                    partial_args: "(AT_FDCWD, \"/data/h445xu/repo/WORKSPACE\", ".to_string(),
                    line_no: 4
                }),
                SyscallLine::Unfinished(UnfinishedSyscallDesp {
                    pid: 815824,
                    cmd: "b".to_string(),
                    syscall: "nanosleep".to_string(),
                    partial_args: "({tv_sec=0, tv_nsec=20000}, ".to_string(),
                    line_no: 5
                }),
                SyscallLine::Resumed(ResumedSyscallDesp {
                    pid: 815823,
                    cmd: "a".to_string(),
                    syscall: "newfstatat".to_string(),
                    partial_args: "0xc00017f628, 0)".to_string(),
                    ret: "-1 ENOENT (No such file or directory)".to_string(),
                    line_no: 6
                })
            ]
        );
    }

    #[test]
    fn test_parse_bug25061301() {
        let line = "2964881<cmd> write(360, \"CC = gcc\\nCPPFLAGS = -g -O3 -Wall -march=native\\n\\nOBJS = main.o iconv.o naive.o\\n\\nutf8to16: ${OBJS}\\n\\tgcc $^ -o $@\\n\\n.PHONY: clean\\nclean:\\n\\trm -f utf8to16 *.o\\n\", 153) = 153";
        let res: Vec<_> = parse_strace_from_content(line.as_bytes()).collect();
        let content = to_json_lines(&res);
        assert_eq!(
            content,
            read_or_create_test_data!("parser/bug25061301.out", &content)
        );
    }

    #[test]
    fn test_large_file() {
        let strace = read_test_data!("strace-cxx.log");

        let res: Vec<_> = parse_strace_from_content(strace.as_bytes()).collect();
        let content = to_json_lines(&res);

        assert_eq!(
            content,
            read_or_create_test_data!("parser/strace.out", &content)
        );
    }

    #[test]
    fn test_large_file_java() {
        let strace = read_test_data!("strace-java.log");

        let res: Vec<_> = parse_strace_from_content(strace.as_bytes()).collect();
        let content = to_json_lines(&res);

        assert_eq!(
            content,
            read_or_create_test_data!("parser/strace-java.out", &content)
        );
    }
}

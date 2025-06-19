use crate::syscall_line::SyscallDesp;
use crate::{syntax, utils};

fn has_dupfd(args: &str) -> bool {
    args.contains("F_DUPFD")
}

fn has_rdonly(args: &str) -> bool {
    args.contains("O_RDONLY")
}

fn has_wronly(args: &str) -> bool {
    args.contains("O_WRONLY")
}

fn has_rdwrd(args: &str) -> bool {
    args.contains("O_RDWR")
}

fn has_trunc(args: &str) -> bool {
    args.contains("O_TRUNC")
}

fn has_creat(args: &str) -> bool {
    args.contains("O_CREAT")
}

fn has_clone_fs(args: &str) -> bool {
    args.contains("CLONE_FS")
}

fn has_clone_files(args: &str) -> bool {
    args.contains("CLONE_FILES")
}

fn to_path_expr(args: &str, p_index: usize) -> Option<syntax::Expr> {
    match utils::extract_pathname(args, p_index) {
        None => None,
        Some(p) => Some(syntax::Expr::P(p)),
    }
}

fn to_fdvar(fd: Option<&str>) -> syntax::FdVar {
    match fd {
        None | Some("AT_FDCWD") => syntax::FdVar::CWD,
        Some(f) => syntax::FdVar::Fd(extract_ret_int(f)),
    }
}

fn to_fdvar_expr(fd: Option<&str>) -> syntax::Expr {
    syntax::Expr::V(to_fdvar(fd))
}

fn to_at_expr(args: &str, fd: Option<&str>, p_index: usize) -> Option<syntax::Expr> {
    match utils::extract_pathname(args, p_index) {
        None => None,
        Some(p) => Some(syntax::Expr::At(to_fdvar(fd), p)),
    }
}

fn is_open_consumed(args: &str) -> bool {
    match (
        has_rdonly(args),
        has_wronly(args),
        has_rdwrd(args),
        has_trunc(args),
        has_creat(args),
    ) {
        (true, _, _, _, _) => true,         // O_RDONLY
        (_, true, _, true, _) => false,     // O_WRONLY|O_TRUNC
        (_, _, true, false, false) => true, // O_RDWR
        (_, _, true, _, true) => false,     // O_RDWR|O_CREAT
        (_, true, _, _, true) => false,     // O_WRONLY|O_CREAT
        (_, true, _, false, false) => true, // O_WRONLY
        _ => true,
    }
}

fn get_fd(args: &str, index: Option<usize>) -> Option<&str> {
    match index {
        None => None,
        Some(i) => Some(utils::extract_arg(args, i)),
    }
}

fn extract_ret_int(ret: &str) -> i64 {
    let mut res = ret;
    if ret.contains(" ") {
        res = &ret[..ret.find(" ").unwrap()];
    }
    if ret.contains("<") {
        res = &ret[..ret.find("<").unwrap()];
    }

    res = res.trim();
    res = res.strip_prefix("(").unwrap_or(res);
    res = res.strip_suffix(")").unwrap_or(res);

    if res.starts_with("0x") {
        i64::from_str_radix(&res[2..], 16).unwrap()
    } else {
        res.parse().unwrap()
    }
}

fn extract_newproc_ret_int(ret: &str) -> i64 {
    // Examples:
    //
    // For `2711930<daemonize>`, we want to extract `2711930`.
    // For `2<linux-sandbox> /* 2716051 in strace's PID NS */`, we want to extract `2716051`.

    if ret.contains("/*") && ret.contains("in strace's PID NS") {
        // Extract the number after `/*` and before `in strace's PID NS`.
        let start = ret.find("/*").unwrap() + 2;
        let end = ret.find("in strace's PID NS").unwrap();
        ret[start..end].trim().parse().unwrap()
    } else if ret.contains('<') {
        // Extract the number before `<`.
        let end = ret.find('<').unwrap();
        ret[..end].trim().parse().unwrap()
    } else {
        // If no special format, just parse it as an integer.
        extract_ret_int(ret)
    }
}

fn to_nop() -> syntax::Statement {
    syntax::Statement::Nop
}

fn to_chdir(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    match to_path_expr(&syscall_desp.args, 0) {
        None => syntax::Statement::Nop,
        Some(e) => syntax::Statement::Let(syntax::FdVar::CWD, e),
    }
}

fn to_newproc(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    syntax::Statement::Newproc(extract_newproc_ret_int(syscall_desp.ret.as_str()))
}

fn to_delfd(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    syntax::Statement::Del(to_fdvar_expr(Some(&syscall_desp.args)))
}

fn to_dupfd_fcntl(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    if has_dupfd(&syscall_desp.args) {
        syntax::Statement::Let(
            to_fdvar(Some(&syscall_desp.ret)),
            to_fdvar_expr(Some(utils::extract_arg(&syscall_desp.args, 0))),
        )
    } else {
        syntax::Statement::Nop
    }
}

fn to_dupfd_dup(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    syntax::Statement::Let(
        to_fdvar(Some(&syscall_desp.ret)),
        to_fdvar_expr(Some(utils::extract_arg(&syscall_desp.args, 0))),
    )
}

fn to_dupfd_dup2(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    syntax::Statement::Let(
        to_fdvar(Some(utils::extract_arg(&syscall_desp.args, 1))),
        to_fdvar_expr(Some(utils::extract_arg(&syscall_desp.args, 0))),
    )
}

fn to_fchdir(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    syntax::Statement::Let(
        syntax::FdVar::CWD,
        to_fdvar_expr(Some(utils::extract_arg(&syscall_desp.args, 0))),
    )
}

fn to_consume(
    syscall_desp: &SyscallDesp,
    d_index: Option<usize>,
    p_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd = get_fd(&syscall_desp.args, d_index);
    match to_at_expr(&syscall_desp.args, fd, p_index) {
        None => syntax::Statement::Nop,
        Some(e) => syntax::Statement::Consume(e),
    }
}

fn to_produce(
    syscall_desp: &SyscallDesp,
    d_index: Option<usize>,
    p_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd = get_fd(&syscall_desp.args, d_index);
    match to_at_expr(&syscall_desp.args, fd, p_index) {
        None => syntax::Statement::Nop,
        Some(e) => syntax::Statement::Produce(e),
    }
}

fn to_link(
    syscall_desp: &SyscallDesp,
    d0_index: Option<usize>,
    p0_index: usize,
    d1_index: Option<usize>,
    p1_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd0 = get_fd(&syscall_desp.args, d0_index);
    let fd1 = get_fd(&syscall_desp.args, d1_index);
    match (
        to_at_expr(&syscall_desp.args, fd0, p0_index),
        to_at_expr(&syscall_desp.args, fd1, p1_index),
    ) {
        (Some(e0), Some(e1)) => syntax::Statement::Link(e0, e1),
        _ => syntax::Statement::Nop,
    }
}

fn to_copy(
    syscall_desp: &SyscallDesp,
    d0_index: Option<usize>,
    p0_index: usize,
    d1_index: Option<usize>,
    p1_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd0 = get_fd(&syscall_desp.args, d0_index);
    let fd1 = get_fd(&syscall_desp.args, d1_index);
    match (
        to_at_expr(&syscall_desp.args, fd0, p0_index),
        to_at_expr(&syscall_desp.args, fd1, p1_index),
    ) {
        (Some(e0), Some(e1)) => syntax::Statement::Copy(e0, e1),
        _ => syntax::Statement::Nop,
    }
}

fn to_del_path(
    syscall_desp: &SyscallDesp,
    d_index: Option<usize>,
    p_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd = get_fd(&syscall_desp.args, d_index);
    match to_at_expr(&syscall_desp.args, fd, p_index) {
        None => syntax::Statement::Nop,
        Some(e) => syntax::Statement::Del(e),
    }
}

fn model_open(
    syscall_desp: &SyscallDesp,
    d_index: Option<usize>,
    p_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd = get_fd(&syscall_desp.args, d_index);
    match to_at_expr(&syscall_desp.args, fd, p_index) {
        None => syntax::Statement::Nop,
        Some(e) => {
            if is_open_consumed(&syscall_desp.args) {
                syntax::Statement::Consume(e)
            } else {
                syntax::Statement::Produce(e)
            }
        }
    }
}

fn to_newfd(
    syscall_desp: &SyscallDesp,
    d_index: Option<usize>,
    p_index: usize,
) -> syntax::Statement {
    if extract_ret_int(&syscall_desp.ret) == -1 {
        return syntax::Statement::Nop;
    }

    let fd = get_fd(&syscall_desp.args, d_index);
    match to_at_expr(&syscall_desp.args, fd, p_index) {
        None => syntax::Statement::Nop,
        Some(e) => syntax::Statement::Let(to_fdvar(Some(&syscall_desp.ret)), e),
    }
}

fn to_begin_task(syscall_desp: &SyscallDesp) -> syntax::Statement {
    if syscall_desp.args.contains("[34mSUBCOMMAND") {
        syntax::Statement::BeginTask(syscall_desp.args.clone())
    } else {
        syntax::Statement::Nop
    }
}

pub fn parse_syscall_desp(syscall_desp: &SyscallDesp) -> Vec<syntax::Statement> {
    match syscall_desp.syscall.as_str() {
        "access" => vec![to_consume(&syscall_desp, None, 0)],
        "chdir" => vec![to_chdir(&syscall_desp)],
        "chmod" => vec![to_consume(&syscall_desp, None, 0)],
        "chown" => vec![to_consume(&syscall_desp, None, 0)],
        "clone" => vec![to_newproc(&syscall_desp)],
        "clone3" => vec![to_newproc(&syscall_desp)],
        "close" => vec![to_delfd(&syscall_desp)],
        "dup" => vec![to_dupfd_dup(&syscall_desp)],
        "dup2" => vec![to_dupfd_dup2(&syscall_desp)],
        "dup3" => vec![to_dupfd_dup2(&syscall_desp)],
        "execve" => vec![to_consume(&syscall_desp, None, 0)],
        "fchdir" => vec![to_fchdir(&syscall_desp)],
        "fchmodat" => vec![to_consume(&syscall_desp, Some(0), 1)],
        "fchownat" => vec![to_consume(&syscall_desp, Some(0), 1)],
        "fcntl" => vec![to_dupfd_fcntl(&syscall_desp)],
        "fork" => vec![to_newproc(&syscall_desp)],
        "getxattr" => vec![to_consume(&syscall_desp, None, 0)],
        "getcwd" => vec![to_chdir(&syscall_desp)],
        "lchown" => vec![to_consume(&syscall_desp, None, 0)],
        "lgetxattr" => vec![to_consume(&syscall_desp, None, 0)],
        "lremovexattr" => vec![to_consume(&syscall_desp, None, 0)],
        "lsetxattr" => vec![to_consume(&syscall_desp, None, 0)],
        "link" => vec![to_link(&syscall_desp, None, 0, None, 1)],
        "linkat" => vec![to_link(&syscall_desp, Some(0), 1, Some(2), 3)],
        "mkdir" => vec![to_produce(&syscall_desp, None, 0)],
        "mkdirat" => vec![to_produce(&syscall_desp, Some(0), 1)],
        "mknod" => vec![to_produce(&syscall_desp, None, 0)],
        "open" => vec![
            to_newfd(&syscall_desp, None, 0),
            model_open(&syscall_desp, None, 0),
        ],
        "openat" => vec![
            to_newfd(&syscall_desp, Some(0), 1),
            model_open(&syscall_desp, Some(0), 1),
        ],
        "pread" => vec![to_nop()],
        "pwrite" => vec![to_nop()],
        "read" => vec![to_nop()],
        "readlink" => vec![to_consume(&syscall_desp, None, 0)],
        "readlinkat" => vec![to_consume(&syscall_desp, Some(0), 1)],
        "removexattr" => vec![to_consume(&syscall_desp, None, 0)],
        "rename" => vec![
            to_copy(&syscall_desp, None, 0, None, 1),
            to_del_path(&syscall_desp, None, 0),
        ],
        "renameat" => vec![
            to_copy(&syscall_desp, Some(0), 1, Some(2), 3),
            to_del_path(&syscall_desp, Some(0), 1),
        ],
        "rmdir" => vec![to_del_path(&syscall_desp, None, 0)],
        "symlink" => vec![to_link(&syscall_desp, None, 0, None, 1)],
        "symlinkat" => vec![to_link(&syscall_desp, None, 0, Some(1), 2)],
        "unlink" => vec![to_del_path(&syscall_desp, None, 0)],
        "unlinkat" => vec![to_del_path(&syscall_desp, Some(0), 1)],
        "utime" => vec![to_consume(&syscall_desp, None, 0)],
        "utimensat" => vec![to_consume(&syscall_desp, Some(0), 1)],
        "utimes" => vec![to_consume(&syscall_desp, None, 0)],
        "vfork" => vec![to_newproc(&syscall_desp)],
        "write" => vec![to_begin_task(&syscall_desp)],
        "writev" => vec![to_nop()],
        _ => {
            println!(
                "Unsupported syscall: {} with args: {}",
                syscall_desp.syscall, syscall_desp.args
            );
            vec![]
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct TraceIR {
    pub statement: syntax::Statement,
    pub syscall: SyscallDesp,
}

pub struct StatementIterator<T>
where
    T: Iterator<Item = SyscallDesp>,
{
    iter: T,
    buffer: Vec<syntax::Statement>,
    syscall: SyscallDesp,
}

impl<T> Iterator for StatementIterator<T>
where
    T: Iterator<Item = SyscallDesp>,
{
    type Item = TraceIR;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            while !self.buffer.is_empty() {
                let stmt = self.buffer.remove(0);
                if stmt != syntax::Statement::Nop {
                    return Some(TraceIR {
                        statement: stmt,
                        syscall: self.syscall.clone(),
                    });
                }
            }
            if let Some(syscall_desp) = self.iter.next() {
                self.buffer = parse_syscall_desp(&syscall_desp);
                self.syscall = syscall_desp;
            } else {
                return None;
            }
        }
    }
}

pub fn parse_syscall_desps(
    syscall_desp: impl IntoIterator<Item = SyscallDesp>,
) -> StatementIterator<impl Iterator<Item = SyscallDesp>> {
    StatementIterator {
        iter: syscall_desp.into_iter(),
        buffer: Vec::new(),
        syscall: SyscallDesp::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{syntax::Statement, syscall_line::SyscallDesp};

    #[test]
    fn test_parse_syscall_desp() {
        let syscall_desp = SyscallDesp {
            pid: 1234,
            cmd: "test_cmd".to_string(),
            resumed_cmd: None,
            syscall: "openat".to_string(),
            args: "AT_FDCWD, \"file.txt\", O_RDONLY".to_string(),
            ret: "3".to_string(),
            line_no: 0,
        };
        let statements = parse_syscall_desp(&syscall_desp);
        assert_eq!(statements.len(), 2);
        assert_eq!(
            statements[0],
            Statement::Let(
                syntax::FdVar::Fd(3),
                syntax::Expr::At(
                    syntax::FdVar::CWD,
                    syntax::Path::Path("file.txt".to_string())
                )
            )
        );
        assert_eq!(
            statements[1],
            Statement::Consume(syntax::Expr::At(
                syntax::FdVar::CWD,
                syntax::Path::Path("file.txt".to_string())
            ))
        );
    }

    #[test]
    fn test_large_file() {
        use crate::{combiner::combine_syscall_lines, parser::parse_strace_from_path};
        use std::fs::{self};
        use std::io::Write;
        use std::path::Path;

        let data_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("test_data/strace.log");
        let expected_data_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("test_data/strace.ir.expected.out");
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(expected_data_path)
            .unwrap();
        for ir in parse_syscall_desps(combine_syscall_lines(parse_strace_from_path(
            data_path.to_str().unwrap(),
        ))) {
            writeln!(
                f,
                "Line {}: {} {} {:?}",
                ir.syscall.line_no, ir.syscall.pid, ir.syscall.syscall, ir.statement
            )
            .unwrap();
        }
    }
}

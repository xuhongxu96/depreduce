use std::collections::{HashMap, HashSet, LinkedList};

use normalize_path::NormalizePath;

use crate::{
    lower::TraceIR,
    syntax::{self},
    syscall_line::{FileDescriptor, ProcessId, SyscallDesp},
    vfs::{INode as VFSINode, Node as VFSNode, VFS},
};

#[derive(PartialEq, Debug, Clone)]
enum FileOperation {
    Consume(Result<(String, VFSINode), String>),
    Produce(Result<(String, VFSINode), String>),
    Delete(Result<(String, VFSNode), String>),
}

#[derive(PartialEq, Debug, Clone, Default)]
struct ProcessState {
    fd_maps: HashMap<FileDescriptor, String>,
    cwd: String,
    parent: Option<ProcessId>,
    operations: Vec<FileOperation>,
}

#[derive(PartialEq, Debug, Clone, Default)]
struct State {
    processes: HashMap<ProcessId, ProcessState>,
    vfs: VFS,
}

impl State {
    fn get_process(&mut self, pid: ProcessId) -> &mut ProcessState {
        self.processes.get_mut(&pid).unwrap()
    }

    fn get_cwd(&self, pid: ProcessId) -> String {
        self.processes
            .get(&pid)
            .map_or_else(|| "/CWD".to_string(), |p| p.cwd.clone())
    }

    fn get_absolute_path(&self, pid: ProcessId, path: &str) -> String {
        if path.starts_with('/') {
            // absolute path
            path.to_string()
        } else {
            let cwd = self.get_cwd(pid);
            std::path::Path::new(&cwd)
                .join(path)
                .normalize()
                .to_string_lossy()
                .to_string()
        }
    }

    fn get_parent_dir(&self, pid: ProcessId, d: &syntax::FdVar) -> Option<String> {
        use syntax::FdVar;
        match d {
            FdVar::CWD => Some(self.get_cwd(pid)),
            FdVar::Fd(0) | FdVar::Fd(1) | FdVar::Fd(2) => None,
            FdVar::Fd(fd) => self.processes.get(&pid)?.fd_maps.get(fd).cloned(),
        }
    }

    fn get_pathname(
        &self,
        pid: ProcessId,
        d: &syntax::FdVar,
        p: &syntax::Path,
    ) -> Option<syntax::Path> {
        use syntax::Path;
        match p {
            Path::Unknown(_) => Some(p.clone()),
            Path::Path(path) => {
                if path.starts_with('/') {
                    // absolute path
                    Some(p.clone())
                } else {
                    match (path.as_str(), self.get_parent_dir(pid, d)) {
                        (_, None) => None,
                        (".", Some(cwd)) => Some(Path::Path(cwd)),
                        ("..", Some(cwd)) => Some(Path::Path(
                            std::path::Path::new(&cwd)
                                .parent()?
                                .to_string_lossy()
                                .to_string(),
                        )),
                        (_, Some(cwd)) => Some(Path::Path(
                            std::path::Path::new(&cwd)
                                .join(path)
                                .normalize()
                                .to_string_lossy()
                                .to_string(),
                        )),
                    }
                }
            }
        }
    }

    fn eval_expr(&self, pid: ProcessId, expr: &syntax::Expr) -> Option<syntax::Path> {
        match expr {
            syntax::Expr::P(p) => Some(p.clone()),
            syntax::Expr::At(fd_var, path) => self.get_pathname(pid, fd_var, path),
            syntax::Expr::V(fd_var) => self
                .get_parent_dir(pid, fd_var)
                .map(|p| syntax::Path::Path(p)),
        }
    }

    fn interpret_let(&mut self, pid: ProcessId, fd_var: &syntax::FdVar, expr: &syntax::Expr) {
        use syntax::{Expr, FdVar, Path};

        match (fd_var, expr) {
            (FdVar::Fd(f1), Expr::V(FdVar::Fd(f2))) => {
                if *f1 == -1 {
                    return;
                }
                let fd_maps = &mut self.get_process(pid).fd_maps;
                if let Some(target_path) = fd_maps.get(f2) {
                    fd_maps.insert(*f1, target_path.clone());
                }
            }
            (FdVar::CWD, Expr::P(Path::Path(path))) => {
                self.get_process(pid).cwd = path.clone();
            }
            _ => match self.eval_expr(pid, expr) {
                Some(Path::Path(path)) => match fd_var {
                    FdVar::CWD => self.get_process(pid).cwd = path.to_string(),
                    FdVar::Fd(fd) => {
                        if *fd == -1 {
                            return;
                        }
                        self.get_process(pid).fd_maps.insert(*fd, path.to_string());
                    }
                },
                _ => {}
            },
        }
    }

    fn interpret_del(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::{Expr, FdVar};

        match expr {
            Expr::V(FdVar::Fd(f)) => {
                self.get_process(pid).fd_maps.remove(f);
            }
            Expr::V(FdVar::CWD) => {}
            _ => match self.eval_expr(pid, expr) {
                Some(syntax::Path::Path(path)) => {
                    let vfs_node = self.remove_inode(pid, &path);

                    self.get_process(pid)
                        .operations
                        .push(FileOperation::Delete(vfs_node));
                }
                _ => {}
            },
        }
    }

    fn get_or_create_inode(
        &mut self,
        pid: ProcessId,
        path: &str,
    ) -> Result<(String, VFSINode), String> {
        let path = self.get_absolute_path(pid, path);
        match self.vfs.get_inode_by_path(&path) {
            Some(inode) => Ok((path, inode)),
            None => {
                // TODO: allowlist for path within the project
                Err(format!("File not found: {}", path))
            }
        }
    }

    fn create_inode(&mut self, pid: ProcessId, path: &str) -> (String, VFSINode) {
        (
            path.to_string(),
            self.vfs
                .create_node_recursively(&self.get_absolute_path(pid, path)),
        )
    }

    fn remove_inode(&mut self, pid: ProcessId, path: &str) -> Result<(String, VFSNode), String> {
        self.vfs
            .remove_node_recursively(&self.get_absolute_path(pid, path))
            .map(|inode| (path.to_string(), inode))
    }

    fn create_symlink_inode(
        &mut self,
        pid: ProcessId,
        path: &str,
        target: &str,
    ) -> Result<(String, VFSINode), String> {
        self.vfs
            .create_symlink(
                &self.get_absolute_path(pid, path),
                &self.get_absolute_path(pid, target),
            )
            .map(|inode| (path.to_string(), inode))
    }

    fn interpret_consume(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                let inode = self.get_or_create_inode(pid, &path);
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Consume(inode));
            }
            _ => {}
        }
    }

    fn interpret_produce(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                let inode = self.create_inode(pid, &path);
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Produce(Ok(inode)));
            }
            _ => {}
        }
    }

    fn interpret_link(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(target_path)), Some(Path::Path(link))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let target_inode = self.get_or_create_inode(pid, &target_path);
            let link_inode = self.create_symlink_inode(pid, &link, &target_path);

            self.get_process(pid)
                .operations
                .push(FileOperation::Consume(target_inode));
            self.get_process(pid)
                .operations
                .push(FileOperation::Produce(link_inode));
        }
    }

    fn interpret_copy(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(path)), Some(Path::Path(path1))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let inode = self.get_or_create_inode(pid, &path);
            let inode1 = self.create_inode(pid, &path1);

            self.get_process(pid)
                .operations
                .push(FileOperation::Consume(inode));
            self.get_process(pid)
                .operations
                .push(FileOperation::Produce(Ok(inode1)));
        }
    }

    fn interpret_newproc(&mut self, syscall: &SyscallDesp, new_pid: ProcessId) {
        let parent_state = self.processes.get(&syscall.pid).unwrap();
        self.processes.insert(
            new_pid,
            ProcessState {
                fd_maps: parent_state.fd_maps.clone(),
                cwd: parent_state.cwd.clone(),
                parent: Some(syscall.pid),
                operations: Vec::new(),
            },
        );
    }

    fn analyze_ir(&mut self, ir: TraceIR) {
        use syntax::Statement;

        match ir.statement {
            Statement::Let(fd_var, expr) => self.interpret_let(ir.syscall.pid, &fd_var, &expr),
            Statement::Del(expr) => self.interpret_del(ir.syscall.pid, &expr),
            Statement::Link(expr, expr1) => self.interpret_link(ir.syscall.pid, &expr, &expr1),
            Statement::Copy(expr, expr1) => self.interpret_copy(ir.syscall.pid, &expr, &expr1),
            Statement::Consume(expr) => self.interpret_consume(ir.syscall.pid, &expr),
            Statement::Produce(expr) => self.interpret_produce(ir.syscall.pid, &expr),
            Statement::Newproc(new_pid) => {
                self.interpret_newproc(&ir.syscall, new_pid.try_into().unwrap())
            }
            Statement::BeginTask(_) => {}
            Statement::Nop => {}
        }
    }
}

fn analyze(irs: impl IntoIterator<Item = TraceIR>, cwd: &str) -> State {
    // FIXME: can we avoid collecting into a Vec?
    let mut irs: Vec<_> = irs.into_iter().collect();
    irs.sort_by(|a, b| a.syscall.line_no.cmp(&b.syscall.line_no));

    let cwd = std::path::Path::new(cwd)
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut state = State::default();

    for ir in irs {
        if state.processes.is_empty() {
            state.processes.insert(
                ir.syscall.pid,
                ProcessState {
                    fd_maps: HashMap::new(),
                    cwd: cwd.to_string(),
                    parent: None,
                    operations: Vec::new(),
                },
            );
        }

        state.analyze_ir(ir);
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::parse_syscall_desps;
    use crate::syntax::{self, Expr, FdVar, Path, Statement};

    #[test]
    fn test_analyze() {
        use crate::{combiner::combine_syscall_lines, parser::parse_strace_from_path};
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
            .join("test_data/strace.paths.expected.out");
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(expected_data_path)
            .unwrap();
        let state = analyze(
            parse_syscall_desps(combine_syscall_lines(parse_strace_from_path(
                data_path.to_str().unwrap(),
            ))),
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
        );

        let mut processes = state.processes.iter().collect::<Vec<_>>();
        processes.sort_by(|(a, _), (b, _)| a.cmp(b));
        processes.iter().for_each(|(pid, props)| {
            writeln!(
                f,
                "Process {} (Parent: {:?}) CWD: {:?}",
                pid, props.parent, props.cwd
            )
            .unwrap();

            for op in &props.operations {
                match op {
                    FileOperation::Consume(res) => {
                        writeln!(f, "  Consume: {:?}", res).unwrap();
                    }
                    FileOperation::Produce(res) => {
                        writeln!(f, "  Produce: {:?}", res).unwrap();
                    }
                    FileOperation::Delete(res) => {
                        writeln!(f, "  Delete: {:?}", res).unwrap();
                    }
                }
            }
        });

        let inode = state
            .vfs
            .get_inode_by_path(
                "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/bazel-bin",
            )
            .unwrap();
        assert_eq!(
            &state.vfs.resolve_link_path(inode),
            "/home/hongxu/.cache/bazel/_bazel_hongxu/6df96e832ca223696660a141f132846f/execroot/_main/bazel-out/k8-fastbuild/bin"
        );
    }
}

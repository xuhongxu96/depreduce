use std::collections::HashMap;

use normalize_path::NormalizePath;

use crate::{
    lower::TraceIR,
    syntax::{self},
    syscall_line::{FileDescriptor, ProcessId, SyscallDesp},
    vfs::{Node as VFSNode, NodeIndex as VFSNodeIndex, VFS},
};

#[derive(PartialEq, Debug, Clone)]
pub enum FileOperation {
    Consume(Result<(String, VFSNodeIndex), String>),
    Produce(Result<(String, VFSNodeIndex), String>),
    Delete(Result<(String, VFSNodeIndex), String>),
    LetFd((FileDescriptor, String)),
    CloseFd(FileDescriptor),
    BeginTask(String),
}

#[derive(PartialEq, Debug, Clone, Default)]
pub struct ProcessState {
    pub fd_maps: HashMap<FileDescriptor, String>,
    pub cwd: String,
    pub parent: Option<ProcessId>,
    pub operations: Vec<FileOperation>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct State {
    pub processes: HashMap<ProcessId, ProcessState>,
    pub vfs: VFS,
    pub cwd: String,
}

impl State {
    fn get_index(&mut self, pid: ProcessId, path: &str) -> Result<(String, VFSNodeIndex), String> {
        let path = self.get_absolute_path(pid, path);
        match self.vfs.get_index_by_path(&path) {
            Some(index) => Ok((self.vfs.resolve_link_path(index).unwrap(), index)),
            None => {
                if path.starts_with(&self.cwd) {
                    let index = self.vfs.create_node_recursively(&path);
                    Ok((path, index))
                } else {
                    Err(format!("File not found: {}", path))
                }
            }
        }
    }

    fn create_index(&mut self, pid: ProcessId, path: &str) -> (String, VFSNodeIndex) {
        let abs_path = self.get_absolute_path(pid, path);
        let index = self.vfs.create_node_recursively(&abs_path);
        (abs_path, index)
    }

    fn remove_index(
        &mut self,
        pid: ProcessId,
        path: &str,
    ) -> Result<(String, VFSNodeIndex), String> {
        let abs_path = self.get_absolute_path(pid, path);
        if let Some(index) = self.vfs.get_index_by_path(&abs_path) {
            self.vfs.remove_node_recursively(&abs_path)?;
            Ok((abs_path, index))
        } else {
            Err(format!("File not found: {}", abs_path))
        }
    }

    fn create_symlink_index(
        &mut self,
        pid: ProcessId,
        path: &str,
        target: &str,
    ) -> Result<(String, VFSNodeIndex), String> {
        self.vfs
            .create_symlink(
                &self.get_absolute_path(pid, path),
                &self.get_absolute_path(pid, target),
            )
            .map(|index| (path.to_string(), index))
    }
}

impl State {
    fn new(cwd: String) -> Self {
        Self {
            processes: HashMap::new(),
            vfs: VFS::new(),
            cwd: cwd,
        }
    }

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
                if let Some(target_path) = fd_maps.get(f2).cloned() {
                    fd_maps.insert(*f1, target_path.clone());
                    self.get_process(pid)
                        .operations
                        .push(FileOperation::LetFd((*f1, target_path)));
                }
            }
            (FdVar::CWD, Expr::P(Path::Path(path))) => {
                self.get_process(pid).cwd = path.clone();
            }
            _ => match self.eval_expr(pid, expr) {
                Some(Path::Path(path)) => match fd_var {
                    FdVar::CWD => self.get_process(pid).cwd = path,
                    FdVar::Fd(fd) => {
                        if *fd == -1 {
                            return;
                        }
                        self.get_process(pid).fd_maps.insert(*fd, path.to_string());
                        self.get_process(pid)
                            .operations
                            .push(FileOperation::LetFd((*fd, path)));
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

                self.get_process(pid)
                    .operations
                    .push(FileOperation::CloseFd(*f));
            }
            Expr::V(FdVar::CWD) => {}
            _ => match self.eval_expr(pid, expr) {
                Some(syntax::Path::Path(path)) => {
                    let vfs_node = self.remove_index(pid, &path);

                    self.get_process(pid)
                        .operations
                        .push(FileOperation::Delete(vfs_node));
                }
                _ => {}
            },
        }
    }

    fn interpret_consume(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                let index = self.get_index(pid, &path);
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Consume(index));
            }
            _ => {}
        }
    }

    fn interpret_produce(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                let index = self.create_index(pid, &path);
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Produce(Ok(index)));
            }
            _ => {}
        }
    }

    fn interpret_link(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(target_path)), Some(Path::Path(link))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let target_index = self.get_index(pid, &target_path);
            let link_index = self.create_symlink_index(pid, &link, &target_path);

            // FIXME: Shall we record the link operation?
            //
            // self.get_process(pid)
            //     .operations
            //     .push(FileOperation::Consume(target_index));
            // self.get_process(pid)
            //     .operations
            //     .push(FileOperation::Produce(link_index));
        }
    }

    fn interpret_copy(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(path)), Some(Path::Path(path1))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let index = self.get_index(pid, &path);
            let index1 = self.create_index(pid, &path1);

            self.get_process(pid)
                .operations
                .push(FileOperation::Consume(index));
            self.get_process(pid)
                .operations
                .push(FileOperation::Produce(Ok(index1)));
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

    fn interpret_begin_task(&mut self, pid: ProcessId, task_name: &str) {
        self.get_process(pid)
            .operations
            .push(FileOperation::BeginTask(task_name.to_string()));
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
            Statement::BeginTask(task_name) => {
                self.interpret_begin_task(ir.syscall.pid, &task_name);
            }
            Statement::Nop => {}
        }
    }
}

pub fn analyze(irs: impl IntoIterator<Item = TraceIR>, cwd: &str) -> State {
    // FIXME: can we avoid collecting into a Vec? Maybe better to change the combiner to finish unfinished syscall first.
    let mut irs: Vec<_> = irs.into_iter().collect();
    irs.sort_by(|a, b| a.syscall.line_no.cmp(&b.syscall.line_no));

    let cwd = std::path::Path::new(cwd)
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut state = State::new(cwd.clone());
    if irs.is_empty() {
        return state;
    }

    state.processes.insert(
        irs[0].syscall.pid,
        ProcessState {
            fd_maps: HashMap::new(),
            cwd: cwd,
            parent: None,
            operations: Vec::new(),
        },
    );

    for ir in irs {
        state.analyze_ir(ir);
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::parse_syscall_desps;
    use crate::syntax::{self, Expr, FdVar, Path, Statement};

    fn dump_analysis(input_strace_path: &str, output_path: &str, cwd: &str) -> State {
        use crate::{combiner::combine_syscall_lines, parser::parse_strace_from_path};
        use std::fs::{self};
        use std::io::Write;
        use std::path::Path;

        let data_path = Path::new(file!()).parent().unwrap().join(input_strace_path);
        let expected_data_path = Path::new(file!()).parent().unwrap().join(output_path);
        let state = analyze(
            parse_syscall_desps(combine_syscall_lines(parse_strace_from_path(
                data_path.to_str().unwrap(),
            ))),
            cwd,
        );

        let mut processes = state.processes.iter().collect::<Vec<_>>();
        processes.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(expected_data_path)
            .unwrap();
        processes.iter().for_each(|(pid, props)| {
            writeln!(
                f,
                "Process {} (Parent: {:?}) CWD: {:?}",
                pid, props.parent, props.cwd
            )
            .unwrap();

            let write_realpath =
                |f: &mut dyn Write, res: &Result<(String, VFSNodeIndex), String>| {
                    if let Ok((path, index)) = res {
                        if let Ok(realpath) = state.vfs.resolve_link_path(*index) {
                            if realpath != *path {
                                writeln!(f, "    RealPath: {}", realpath).unwrap();
                            }
                        }
                    }
                };

            for op in &props.operations {
                match op {
                    FileOperation::Consume(res) => {
                        writeln!(f, "  Consume: {:?}", res).unwrap();
                        write_realpath(&mut f, res);
                    }
                    FileOperation::Produce(res) => {
                        writeln!(f, "  Produce: {:?}", res).unwrap();
                        write_realpath(&mut f, res);
                    }
                    FileOperation::Delete(res) => {
                        writeln!(f, "  Delete: {:?}", res).unwrap();
                        write_realpath(&mut f, res);
                    }
                    FileOperation::LetFd((fd, path)) => {
                        writeln!(f, "  Let FD: {} -> {}", fd, path).unwrap();
                    }
                    FileOperation::CloseFd(fd) => {
                        writeln!(f, "  Close FD: {}", fd).unwrap();
                    }
                    FileOperation::BeginTask(task_name) => {
                        writeln!(f, "  Begin Task: {}", task_name).unwrap();
                    }
                }
            }
        });

        state
    }

    #[test]
    fn test_analyze_cxx() {
        let state = dump_analysis(
            "test_data/strace.log",
            "test_data/strace.paths.expected.out",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
        );

        let index = state
            .vfs
            .get_index_by_path(
                "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/bazel-bin",
            )
            .unwrap();
        assert_eq!(
            &state.vfs.resolve_link_path(index).unwrap(),
            "/home/hongxu/.cache/bazel/_bazel_hongxu/6df96e832ca223696660a141f132846f/execroot/_main/bazel-out/k8-fastbuild/bin",
        );

        let index = state
            .vfs
            .get_index_by_path(
                "/home/hongxu/.cache/bazel/_bazel_hongxu/6df96e832ca223696660a141f132846f/execroot/_main/main/main.cpp",
            )
            .unwrap();
        assert_eq!(
            &state.vfs.resolve_link_path(index).unwrap(),
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/main/main.cpp",
        );
    }

    #[test]
    fn test_analyze_java() {
        let state = dump_analysis(
            "test_data/strace-java.log",
            "test_data/strace-java.paths.expected.out",
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-java-project",
        );
    }
}

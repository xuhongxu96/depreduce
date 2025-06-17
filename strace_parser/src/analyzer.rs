use std::collections::{HashMap, HashSet, LinkedList};

use normalize_path::NormalizePath;

use crate::{
    lower::TraceIR,
    syntax::{self},
    syscall_line::{FileDescriptor, ProcessId, SyscallDesp},
};

#[derive(PartialEq, Debug, Clone)]
enum FileOperation {
    Consume(String),
    Produce(String),
    Delete(String),
}

#[derive(PartialEq, Debug, Clone, Default)]
struct ProcessState {
    fd_maps: HashMap<FileDescriptor, String>,
    cwd: String,
    parent: Option<ProcessId>,
    operations: Vec<FileOperation>,
}

#[derive(PartialEq, Debug, Clone, Default)]
struct PathProps {
    path: String,
    links: HashSet<String>,
    copied_from: String,
    deleted: bool,
}

#[derive(PartialEq, Debug, Clone, Default)]
struct State {
    processes: HashMap<ProcessId, ProcessState>,
    paths: HashMap<String, PathProps>,
    recent_sandboxed_newprocs: LinkedList<SyscallDesp>,
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
                    self.get_process(pid)
                        .operations
                        .push(FileOperation::Delete(path.clone()));

                    if let Some(path_props) = self.paths.get_mut(&path) {
                        path_props.links.clear();
                        path_props.deleted = true;
                    }
                }
                _ => {}
            },
        }
    }

    fn interpret_consume(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Consume(path.clone()));
            }
            _ => {}
        }
    }

    fn interpret_produce(&mut self, pid: ProcessId, expr: &syntax::Expr) {
        use syntax::Path;

        match self.eval_expr(pid, expr) {
            Some(Path::Path(path)) => {
                self.paths
                    .entry(path.clone())
                    .or_insert_with(|| PathProps {
                        path: path.clone(),
                        links: HashSet::new(),
                        copied_from: String::new(),
                        deleted: false,
                    })
                    .deleted = false;
                self.get_process(pid)
                    .operations
                    .push(FileOperation::Produce(path.clone()));
            }
            _ => {}
        }
    }

    fn interpret_link(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(path)), Some(Path::Path(link))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let link_prop = self.paths.entry(link.clone()).or_insert_with(|| PathProps {
                path: link.clone(),
                links: HashSet::new(),
                copied_from: String::new(),
                deleted: false,
            });
            link_prop.links.insert(path.clone());

            // self.paths.entry(path.clone()).or_insert_with(|| PathProps {
            //     path: path.clone(),
            //     links: HashSet::new(),
            //     copied_from: String::new(),
            //     deleted: false,
            // });

            self.get_process(pid)
                .operations
                .push(FileOperation::Consume(path.clone()));
            self.get_process(pid)
                .operations
                .push(FileOperation::Produce(link.clone()));
        }
    }

    fn interpret_copy(&mut self, pid: ProcessId, expr: &syntax::Expr, expr1: &syntax::Expr) {
        use syntax::Path;

        if let (Some(Path::Path(path)), Some(Path::Path(path1))) =
            (self.eval_expr(pid, expr), self.eval_expr(pid, expr1))
        {
            let path_props = self
                .paths
                .entry(path1.clone())
                .or_insert_with(|| PathProps {
                    path: path1.clone(),
                    links: HashSet::new(),
                    copied_from: String::new(),
                    deleted: false,
                });
            path_props.copied_from = path.clone();
            path_props.deleted = false;

            self.get_process(pid)
                .operations
                .push(FileOperation::Consume(path.clone()));
            self.get_process(pid)
                .operations
                .push(FileOperation::Produce(path1.clone()));
        }
    }

    fn interpert_newproc(&mut self, syscall: &SyscallDesp, new_pid: ProcessId) {
        if new_pid < 100 {
            self.recent_sandboxed_newprocs.push_back(syscall.clone());
        } else {
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
                self.interpert_newproc(&ir.syscall, new_pid.try_into().unwrap())
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
        if !state.processes.contains_key(&ir.syscall.pid) {
            if state.recent_sandboxed_newprocs.is_empty() {
                state.processes.insert(
                    ir.syscall.pid,
                    ProcessState {
                        fd_maps: HashMap::new(),
                        cwd: cwd.to_string(),
                        parent: None,
                        operations: Vec::new(),
                    },
                );
            } else {
                let first_sandboxed = state.recent_sandboxed_newprocs.pop_front().unwrap();
                let parent_state = state.get_process(first_sandboxed.pid);
                let fd_maps = parent_state.fd_maps.clone();
                let cwd = parent_state.cwd.clone();
                state.processes.insert(
                    ir.syscall.pid,
                    ProcessState {
                        fd_maps: fd_maps,
                        cwd: cwd,
                        parent: Some(first_sandboxed.pid),
                        operations: Vec::new(),
                    },
                );
            }
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

        let mut paths = state.paths.iter().collect::<Vec<_>>();
        paths.sort_by(|(a, _), (b, _)| a.cmp(b));
        paths.iter().for_each(|(_, props)| {
            writeln!(f, "{:?}", props).unwrap();
        });
    }
}

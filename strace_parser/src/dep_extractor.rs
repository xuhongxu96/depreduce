use std::collections::{HashMap, HashSet};

use crate::{
    analyzer::{FileOperation, State},
    syscall_line::FileDescriptor,
    vfs::NodeIndex,
};

pub struct DependencyExtractor {
    paths: Vec<String>,
    path_index_map: HashMap<String, usize>,
    deps: HashMap<usize, HashSet<usize>>,

    final_dep_caches: HashMap<usize, HashSet<usize>>,
    attentioned_paths: HashSet<usize>,
}

pub struct DependencyGraph {
    pub deps: HashMap<String, HashSet<String>>,
}

impl DependencyExtractor {
    fn new() -> Self {
        DependencyExtractor {
            paths: Vec::new(),
            path_index_map: HashMap::new(),
            deps: HashMap::new(),
            final_dep_caches: HashMap::new(),
            attentioned_paths: HashSet::new(),
        }
    }

    pub fn get_path(&self, index: usize) -> Option<&String> {
        self.paths.get(index)
    }

    pub fn get_path_index(&self, path: &str) -> Option<usize> {
        self.path_index_map.get(path).copied()
    }

    fn add_path(&mut self, path: String) -> usize {
        if let Some(&index) = self.path_index_map.get(&path) {
            index
        } else {
            let index = self.paths.len();
            self.paths.push(path.clone());
            self.path_index_map.insert(path, index);
            index
        }
    }

    fn add_dependency(&mut self, from: usize, to: usize) {
        self.deps.entry(from).or_default().insert(to);
    }

    fn cache_final_dependencies(&mut self, index: usize, attentioned_paths: &HashSet<usize>) {
        if let Some(_) = self.final_dep_caches.get(&index) {
            return;
        }

        if self.deps.get(&index).unwrap_or(&HashSet::new()).is_empty() {
            let mut itself = HashSet::new();
            itself.insert(index);
            self.final_dep_caches.insert(index, itself);
            return;
        }

        self.final_dep_caches.insert(index, HashSet::new());

        let mut cache = HashSet::new();
        for dep in self.deps.get(&index).unwrap().clone() {
            if attentioned_paths.contains(&dep) {
                cache.insert(dep);
            } else {
                self.cache_final_dependencies(dep, attentioned_paths);
                if let Some(final_deps) = self.final_dep_caches.get(&dep) {
                    for final_dep in final_deps {
                        if attentioned_paths.contains(final_dep) {
                            cache.insert(*final_dep);
                        }
                    }
                }
            }
        }

        self.final_dep_caches.insert(index, cache);
    }

    fn simplify(&mut self, paths: impl IntoIterator<Item = usize>) {
        let paths: HashSet<usize> = paths.into_iter().collect();
        for index in &paths {
            self.cache_final_dependencies(*index, &paths);
        }
        self.attentioned_paths = paths;
        self.final_dep_caches
            .retain(|k, v| v.len() > 1 && self.attentioned_paths.contains(k));
    }

    pub fn get_dependencies(&self) -> DependencyGraph {
        let mut deps = HashMap::new();
        for (i, paths) in &self.final_dep_caches {
            let src = self.get_path(*i).unwrap();

            deps.insert(
                src.clone(),
                paths
                    .iter()
                    .filter_map(|&dep| self.get_path(dep))
                    .cloned()
                    .collect(),
            );
        }

        DependencyGraph { deps }
    }
}

pub fn extract_dependencies(state: &State) -> DependencyExtractor {
    let mut res = DependencyExtractor::new();

    let mut consumed_files: HashSet<usize> = HashSet::new();
    let get_realpath = |path: &str, index: NodeIndex| {
        if let Ok(realpath) = state.vfs.resolve_link_path(index) {
            if realpath != path {
                realpath
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        }
    };

    let mut processes: Vec<_> = state.processes.iter().collect();
    processes.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (_, process) in processes {
        consumed_files.clear();

        let mut path2fd: HashMap<&str, (FileDescriptor, Option<NodeIndex>)> = HashMap::new();
        let mut fd2path: HashMap<FileDescriptor, &str> = HashMap::new();

        for op in &process.operations {
            match op {
                FileOperation::Consume(Ok((path, index))) => {
                    path2fd.remove(path.as_str()).map(|(fd, _)| {
                        fd2path.remove(&fd);
                    });

                    let realpath = get_realpath(path, *index);
                    let index = res.add_path(realpath);
                    consumed_files.insert(index);
                }
                FileOperation::Produce(Ok((path, index))) => match path2fd.get_mut(path.as_str()) {
                    Some((_, node)) => {
                        *node = Some(*index);
                    }
                    None => {
                        let realpath = get_realpath(path, *index);
                        let index = res.add_path(realpath);
                        for consumed in &consumed_files {
                            res.add_dependency(index, *consumed);
                        }
                    }
                },
                FileOperation::Delete(Ok((path, index))) => {
                    let realpath = get_realpath(path, *index);
                    if let Some(index) = res.get_path_index(&realpath) {
                        consumed_files.remove(&index);
                    }
                }
                FileOperation::LetFd((fd, path)) => {
                    path2fd.insert(path, (*fd, None));
                    fd2path.insert(*fd, path);
                }
                FileOperation::CloseFd(fd) => {
                    fd2path.remove(fd).map(|path| {
                        path2fd.remove(path).map(|(_, index)| {
                            index.map(|index| {
                                let realpath = get_realpath(path, index);
                                let index = res.add_path(realpath);
                                for consumed in &consumed_files {
                                    res.add_dependency(index, *consumed);
                                }
                            });
                        });
                    });
                }
                FileOperation::BeginTask(_) => {
                    consumed_files.clear();
                }
                _ => {}
            }
        }
    }

    let mut attentioned_paths: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();

    // <cwd>/bazel-<the last component of cwd>/external
    let external_path = state
        .cwd
        .split('/')
        .last()
        .map(|last| format!("{}/bazel-{}/external", state.cwd, last))
        .map(|p| state.vfs.get_index_by_path(&p))
        .flatten()
        .map(|i| state.vfs.resolve_link_path(i).ok())
        .flatten();
    let out_path = Some(format!("{}/bazel-out", state.cwd))
        .map(|p| state.vfs.get_index_by_path(&p))
        .flatten()
        .map(|i| state.vfs.resolve_link_path(i).ok())
        .flatten();

    stack.push(res.add_path(state.cwd.clone()));
    while let Some(path_index_in_graph) = stack.pop() {
        if attentioned_paths.contains(&path_index_in_graph) {
            continue;
        }
        let path = res.get_path(path_index_in_graph).unwrap();
        if path.as_str() == external_path.as_deref().unwrap_or("")
            || path.as_str() == out_path.as_deref().unwrap_or("")
        {
            continue; // skip bazel external paths
        }

        attentioned_paths.insert(path_index_in_graph);

        let path_index_in_vfs = state.vfs.get_index_by_path(path).unwrap();
        let children = state.vfs.get_children(path_index_in_vfs).unwrap();
        for (_, child_index) in children {
            if let Ok(child_path) = state.vfs.resolve_link_path(*child_index) {
                stack.push(res.add_path(child_path));
            }
        }
    }

    res.simplify(attentioned_paths);
    res
}

impl DependencyGraph {
    pub fn to_sorted_vec(&self) -> Vec<(String, Vec<String>)> {
        let mut deps: Vec<_> = self
            .deps
            .iter()
            .map(|(k, v)| {
                let mut paths: Vec<_> = v.iter().cloned().collect();
                paths.sort();
                (k.clone(), paths)
            })
            .collect();

        deps.sort_by(|(a, _), (b, _)| a.cmp(b));
        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::*;

    fn dump_deps(input_strace_path: &str, output_path: &str) {
        let state_content = read_test_data!(input_strace_path);
        let state = State::from_json_lines(&state_content);
        let dependency_extractor = extract_dependencies(&state);
        let dep_graph = dependency_extractor.get_dependencies();

        let content = to_json_lines(&dep_graph.to_sorted_vec());
        assert_eq!(content, read_or_create_test_data!(output_path, &content));
    }

    #[test]
    fn test_dep_extractor() {
        dump_deps(
            "analyzer/strace-state-cxx.out",
            "dep_extractor/deps-cxx.out",
        );
    }

    #[test]
    fn test_dep_extractor_java() {
        dump_deps(
            "analyzer/strace-state-java.out",
            "dep_extractor/deps-java.out",
        );
    }
}

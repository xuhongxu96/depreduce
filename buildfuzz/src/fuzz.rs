use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    process::Command,
    time::SystemTime,
};

use normalize_path::NormalizePath;
use utils::DependencyGraph;

use crate::touchers::Toucher;

struct BuildArtifacts {
    inputs: HashSet<String>,
    outputs: HashSet<String>,
    command: String,
    cwd: String,

    touchers: Vec<Box<dyn Toucher>>,
}

fn read_timestamp<P: AsRef<std::path::Path>>(file: P) -> u128 {
    std::fs::metadata(file)
        .map(|metadata| {
            metadata
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        })
        .unwrap()
}

impl BuildArtifacts {
    fn norm_path(&self, path: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.cwd).join(path).normalize()
    }

    fn read_timestamps(&self) -> HashMap<&String, u128> {
        self.outputs
            .iter()
            .map(|file| (file, read_timestamp(self.norm_path(file))))
            .collect()
    }

    // Returns a closure that can restore the original content of the file
    fn touch(&self, path: &str) -> impl Fn() -> () {
        let abs_path = self.norm_path(path).to_string_lossy().to_string();
        let original_content = std::fs::read_to_string(&abs_path).unwrap();

        for toucher in &self.touchers {
            if toucher.should_touch(&abs_path) {
                toucher.touch(&abs_path);
            }
        }

        move || {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&abs_path)
                .unwrap();
            file.write_all(original_content.as_bytes()).unwrap();
        }
    }

    fn build(&self) -> Result<(), std::io::Error> {
        let abs_path = self.norm_path(&self.command);

        Command::new("/bin/bash")
            .arg(abs_path)
            .current_dir(&self.cwd)
            .spawn()?
            .wait()?;

        Ok(())
    }

    pub fn fuzz(&self) -> Result<DependencyGraph, String> {
        let mut res = HashMap::new();

        for file in &self.inputs {
            if self.build().is_err() {
                return Err("Failed to build the original project".to_string());
            }

            let t0 = self.read_timestamps();
            let restore_fn = self.touch(file);

            if self.build().is_err() {
                restore_fn();
                return Err(format!("Failed to build after touching {}", file));
            }

            let t1 = self.read_timestamps();

            self.outputs
                .iter()
                .filter(|output| t0[output] < t1[output])
                .for_each(|output| {
                    res.entry(output.clone())
                        .or_insert_with(|| HashSet::new())
                        .insert(file.clone());
                });

            restore_fn();
            println!("Restored file {}", file);
        }

        Ok(DependencyGraph { deps: res })
    }
}

#[cfg(test)]
mod tests {
    use utils::*;

    use super::*;

    #[test]
    fn test_fuzz() {
        let artifacts = BuildArtifacts {
            inputs: vec![
                "liba/a.h".to_string(),
                "liba/a.cpp".to_string(),
                "libb/b.h".to_string(),
                "libb/b.cpp".to_string(),
                "main/main.cpp".to_string(),
            ]
            .into_iter()
            .collect(),
            outputs: vec![
                "bazel-bin/liba/libliba.a".to_string(),
                "bazel-bin/liba/libliba.so".to_string(),
                "bazel-bin/liba/_objs/liba/a.pic.o".to_string(),
                "bazel-bin/libb/liblibb.a".to_string(),
                "bazel-bin/libb/liblibb.so".to_string(),
                "bazel-bin/libb/_objs/libb/b.pic.o".to_string(),
                "bazel-bin/main/main".to_string(),
                "bazel-bin/main/_objs/main/main.pic.o".to_string(),
            ]
            .into_iter()
            .collect(),
            command: get_test_data_path!("build.sh")
                .to_string_lossy()
                .to_string(),
            cwd: get_test_data_path!("../../../examples/simple-cxx-project")
                .to_string_lossy()
                .to_string(),
            touchers: vec![Box::new(crate::touchers::c_toucher::CToucher {})],
        };

        let dep_graph = artifacts.fuzz().unwrap();
        let content = to_json_lines(&dep_graph.to_sorted_vec());
        assert_eq!(content, read_or_create_test_data!("fuzz/cxx.out", &content));
    }
}

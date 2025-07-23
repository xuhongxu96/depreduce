use std::collections::{BTreeMap, HashMap, HashSet};
use std::{
    io::{BufRead, BufReader},
    process::Command,
};

use serde::{Serialize, Serializer};

pub fn ordered_map<S, K: Ord + Serialize, V: Serialize>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub struct DependencyMap {
    pub deps: HashMap<String, HashSet<String>>,
}

impl DependencyMap {
    pub fn new() -> Self {
        Self {
            deps: HashMap::new(),
        }
    }

    pub fn to_sorted_vec(&self) -> Vec<(String, Vec<String>)> {
        let mut deps: Vec<_> = self
            .deps
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| {
                let mut paths: Vec<_> = v.iter().cloned().collect();
                paths.sort();
                (k.clone(), paths)
            })
            .collect();

        deps.sort_by(|(a, _), (b, _)| a.cmp(b));
        deps
    }

    pub fn from_json_lines(json_lines: &str) -> Self {
        Self {
            deps: from_json_lines::<(String, Vec<String>)>(json_lines).fold(
                HashMap::new(),
                |mut acc, (k, v)| {
                    acc.entry(k).or_insert_with(HashSet::new).extend(v);
                    acc
                },
            ),
        }
    }
}

#[macro_export]
macro_rules! get_test_data_path {
    ($path:expr) => {{
        use std::path::Path;
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/test_data")
            .join($path)
    }};
}

#[macro_export]
macro_rules! read_test_data {
    ($path:expr) => {{
        use std::fs;
        use std::path::Path;

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/test_data")
            .join($path);

        fs::read_to_string(&path).expect("Failed to read test data")
    }};
}

#[macro_export]
macro_rules! read_or_create_test_data {
    ($path:expr, $content:expr) => {{
        use std::fs::{self, File};
        use std::io::Write;
        use std::path::Path;

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/test_data")
            .join($path);

        fs::create_dir_all(path.parent().unwrap()).unwrap_or_default();

        match fs::read_to_string(&path) {
            Ok(existing_content) => existing_content,
            Err(_) => {
                let mut file = File::create(path).expect("Failed to create file");
                file.write_all($content.as_bytes())
                    .expect("Failed to write to file");
                $content.to_string()
            }
        }
    }};
}

pub fn to_json_lines<T>(items: &[T]) -> String
where
    T: serde::Serialize,
{
    items
        .iter()
        .map(|item| serde_json::to_string(item).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn from_json_lines<T>(json_lines: &str) -> impl Iterator<Item = T>
where
    T: serde::de::DeserializeOwned,
{
    json_lines
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
}

pub fn indent_all_lines(text: &str, indent_size: usize) -> String {
    let indent = " ".repeat(indent_size);
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn remove_lines_with_indent(text: &str, indent_size: usize) -> String {
    let indent = " ".repeat(indent_size);
    text.lines()
        .filter(|line| !line.starts_with(&indent))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn get_bazel_query(workspace: &str) -> String {
    let mut p = Command::new("bazel")
        .arg("query")
        .arg("deps(//...)")
        .arg("--keep_going")
        .arg("--notool_deps")
        .arg("--noimplicit_deps")
        .arg("--output")
        .arg("xml")
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run bazel query");

    let mut xml_str = String::new();
    let stdout = p.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for (i, line) in stdout_lines.enumerate() {
        let line = line.expect("Failed to read line from bazel query output");

        xml_str.push_str(&line);
        if i % 1000 == 0 {
            eprintln!("Read {} lines from bazel query output...", i);
        }
    }

    p.wait().expect("Bazel query did not finish successfully");

    xml_str
}

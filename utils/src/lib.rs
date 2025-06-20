use std::collections::{HashMap, HashSet};

pub struct DependencyGraph {
    pub deps: HashMap<String, HashSet<String>>,
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

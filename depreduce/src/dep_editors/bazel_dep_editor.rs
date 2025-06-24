use std::collections::HashMap;
use std::path::Path;

use rustpython_parser::Parse;
use rustpython_parser::ast::Ranged;
use serde::{Deserialize, Serialize};

use crate::dep_editors::{ChangedFiles, DepEditor};
use crate::dep_graph::bazel_xml_parser::{Query, SkyValue};

pub struct BazelDepEditor {
    label2location: HashMap<String, String>,
    workspace_root: String,
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
struct BazelLabel {
    name: String,
    package: String,
    repo: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
struct Interval {
    start: usize,
    end: usize,
}

impl Interval {
    fn size(&self) -> usize {
        self.end - self.start
    }
}

impl BazelLabel {
    fn parse(label: &str) -> BazelLabel {
        let mut res = BazelLabel {
            name: String::new(),
            package: String::new(),
            repo: String::new(),
        };

        if label.starts_with("@") {
            let i = label.find('/').unwrap();
            let mut res = BazelLabel::parse(&format!("/{}", &label[i + 1..]));
            res.repo = label[..i].to_string();
            return res;
        }

        if !label.starts_with("//") {
            res.name = label.strip_prefix(":").unwrap_or(label).to_string();
        } else if label.contains(':') {
            let i = label.find(':').unwrap();
            res.package = label[..i].to_string();
            res.name = label[i + 1..].to_string();
        } else {
            res.package = label.to_string();
            res.name = res.package.split('/').last().unwrap().to_string();
        }

        res
    }
}

impl ToString for BazelLabel {
    fn to_string(&self) -> String {
        format!("{}{}:{}", self.repo, self.package, self.name)
    }
}

impl std::fmt::Debug for BazelLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}:{}", self.repo, self.package, self.name)
    }
}

fn extract_list_items(expr: &rustpython_parser::ast::Expr) -> Vec<(String, Interval)> {
    use rustpython_parser::ast;

    let mut res = vec![];

    match expr {
        ast::Expr::Call(e) => {
            e.keywords.iter().for_each(|kw| {
                res.extend(extract_list_items(&kw.value));
            });
        }
        ast::Expr::List(e) => {
            for (i, item) in e.elts.iter().enumerate() {
                if i > 0 {
                    res.last_mut().unwrap().1.end = item.range().start().to_usize();
                }

                let mut label = String::new();
                if let ast::Expr::Constant(c) = item {
                    label = c.value.as_str().cloned().unwrap_or("".to_string());
                }

                res.push((
                    label,
                    Interval {
                        start: item.range().start().to_usize(),
                        end: item.range().end().to_usize(),
                    },
                ));
            }

            if let Some(last) = res.last_mut() {
                last.1.end = e.range().end().to_usize();
            }
        }
        _ => {}
    }

    res
}

impl BazelDepEditor {
    pub fn new(query: &Query, workspace_root: String) -> Self {
        let mut label2location = HashMap::new();
        for value in &query.values {
            match value {
                SkyValue::SourceFile(source_file) => {
                    label2location.insert(source_file.name.clone(), source_file.location.clone());
                }
                SkyValue::Rule(rule) => {
                    label2location.insert(rule.name.clone(), rule.location.clone());
                }
                SkyValue::GeneratedFile(generated_file) => {
                    label2location
                        .insert(generated_file.name.clone(), generated_file.location.clone());
                }
                SkyValue::PackageGroup(_package_group) => {}
            }
        }
        Self {
            label2location,
            workspace_root,
        }
    }

    fn normalize_label(&self, label: &BazelLabel, build_file_path: &str) -> BazelLabel {
        let dir = Path::new(build_file_path).parent().unwrap();
        let dir = dir.strip_prefix(&self.workspace_root).unwrap_or(dir);

        let mut pkg = if label.package.is_empty() {
            format!("//{}", dir.to_str().unwrap())
        } else {
            label.package.clone()
        };

        if pkg == "//." {
            pkg = "//".to_string();
        }

        return BazelLabel {
            name: label.name.clone(),
            package: pkg,
            repo: label.repo.clone(),
        };
    }

    fn extract_all_labels(&self, location: &str) -> Vec<(BazelLabel, Interval)> {
        use rustpython_parser::ast;

        // split location as 3 parts: path, start_line, end_line
        let parts: Vec<&str> = location.split(':').collect();
        assert_eq!(parts.len(), 3, "Invalid location format: {}", location);

        let path = parts[0];
        let start_line = parts[1].parse::<usize>().unwrap_or(0);
        let _end_col = parts[2].parse::<usize>().unwrap_or(0);

        let build = std::fs::read_to_string(path).unwrap();

        // convert start_line and end_line to char offsets
        let start_offset = build
            .lines()
            .take(start_line - 1)
            .map(|s| s.len() + 1)
            .sum::<usize>();

        let mut res = vec![];
        let ast = rustpython_parser::ast::Suite::parse(&build, path).unwrap();
        for stmt in ast {
            if stmt.range().start().to_usize() < start_offset {
                continue;
            }
            match stmt {
                ast::Stmt::Expr(e) => res.extend(extract_list_items(e.value.as_ref())),
                _ => {}
            }
            break;
        }

        res.iter()
            .map(|(label, interval)| {
                let bazel_label = BazelLabel::parse(label);
                let normalized_label = self.normalize_label(&bazel_label, path);
                (normalized_label, interval.clone())
            })
            .collect()
    }
}

impl DepEditor for BazelDepEditor {
    fn add(&mut self, label: &str, dep_label: &str) -> Result<ChangedFiles, String> {
        if let Some(location) = self.label2location.get(label) {
            let changed_files = vec![location.clone()];
            Ok(changed_files)
        } else {
            Err(format!("Label '{}' not found", label))
        }
    }

    fn remove(&mut self, label: &str, dep_label: &str) -> Result<ChangedFiles, String> {
        if let Some(location) = self.label2location.get(label) {
            let changed_files = vec![location.clone()];
            Ok(changed_files)
        } else {
            Err(format!("Label '{}' not found", label))
        }
    }

    fn save(&mut self) -> super::CheckpointId {
        todo!()
    }

    fn restore(&mut self, checkpoint_id: super::CheckpointId) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use utils::{get_test_data_path, read_or_create_test_data, read_test_data};

    use crate::dep_graph::bazel_xml_parser::parse_bazel_xml;

    use super::*;

    #[fixture]
    #[once]
    fn fake_query() -> Query {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><query version=\"2\"><source-file location=\"\" name=\"\"></source-file></query>";
        parse_bazel_xml(xml).unwrap()
    }

    #[fixture]
    #[once]
    fn cxx_query() -> Query {
        let xml = read_test_data!("cxx-deps.xml");
        parse_bazel_xml(&xml).unwrap()
    }

    #[test]
    fn test_label_with_repo() {
        let label = "@maven//example/package:target_name";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "target_name");
        assert_eq!(parsed.package, "//example/package");
        assert_eq!(parsed.repo, "@maven");
    }

    #[test]
    fn test_label_without_repo() {
        let label = "//example/package:target_name";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "target_name");
        assert_eq!(parsed.package, "//example/package");
        assert_eq!(parsed.repo, "");
    }

    #[test]
    fn test_label_omitting_name() {
        let label = "//antlropt/src/org/perses/antlr/ast";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "ast");
        assert_eq!(parsed.package, "//antlropt/src/org/perses/antlr/ast");
        assert_eq!(parsed.repo, "");
    }

    #[test]
    fn test_label_in_root_package() {
        let label = "//:target_name";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "target_name");
        assert_eq!(parsed.package, "//");
        assert_eq!(parsed.repo, "");
    }

    #[test]
    fn test_label_without_package() {
        let label = ":target_name";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "target_name");
        assert_eq!(parsed.package, "");
        assert_eq!(parsed.repo, "");
    }

    #[test]
    fn test_label_without_package_or_colon() {
        let label = "target_name";
        let parsed = BazelLabel::parse(label);
        assert_eq!(parsed.name, "target_name");
        assert_eq!(parsed.package, "");
        assert_eq!(parsed.repo, "");
    }

    #[rstest]
    fn test_normalize_label(fake_query: &Query) {
        let editor = BazelDepEditor::new(
            fake_query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project".to_string(),
        );

        let label = BazelLabel::parse("//main");
        let normalized = editor.normalize_label(
            &label,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/main/BUILD",
        );
        assert_eq!(normalized.to_string(), "//main:main");

        let label = BazelLabel::parse("liba");
        let normalized = editor.normalize_label(
            &label,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/liba/BUILD",
        );
        assert_eq!(normalized.to_string(), "//liba:liba");
    }

    #[rstest]
    fn test_extract_all_labels(cxx_query: &Query) {
        let editor = BazelDepEditor::new(
            cxx_query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project".to_string(),
        );

        let labels = editor.extract_all_labels(
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/main/BUILD:3:10",
        );
        let res = format!("{:#?}", labels);
        assert_eq!(
            res,
            read_or_create_test_data!("dep_editors/bazel_dep_editor/extract_all_labels.out", res)
        );
    }

    #[rstest]
    fn test_extract_all_labels_2(fake_query: &Query) {
        let editor = BazelDepEditor::new(
            fake_query,
            get_test_data_path!("").to_string_lossy().to_string(),
        );
        let labels = editor.extract_all_labels(&format!(
            "{}:3:12",
            get_test_data_path!("test.BUILD").to_str().unwrap()
        ));
        let res = format!("{:#?}", labels);
        assert_eq!(
            res,
            read_or_create_test_data!("dep_editors/bazel_dep_editor/extract_all_labels_2.out", res)
        );
    }

    #[rstest]
    fn test_bazel_dep_editor(cxx_query: &Query) {
        let mut editor = BazelDepEditor::new(
            cxx_query,
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project".to_string(),
        );
        let changed_files = editor.remove("//main:main", "//liba:liba").unwrap();
        assert_eq!(changed_files.len(), 1);
        assert_eq!(
            changed_files[0],
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project/main/BUILD:3:10"
        );
    }
}

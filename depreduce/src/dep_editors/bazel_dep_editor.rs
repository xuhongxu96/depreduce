use std::collections::HashMap;
use std::path::Path;

use crate::dep_editors::{ChangedFiles, DepEditor};
use crate::dep_graph::bazel_xml_parser::{Query, SkyValue};

pub struct BazelDepEditor {
    label2location: HashMap<String, String>,
    workspace_root: String,
}

struct BazelLabel {
    name: String,
    package: String,
    repo: String,
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
            res.repo = label[..i].to_string();
            return BazelLabel::parse(&format!("/{}", &label[i + 1..]));
        }

        if !label.starts_with("//") {
            res.name = label.strip_prefix(":").unwrap().to_string();
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
    use utils::read_test_data;

    use crate::dep_graph::bazel_xml_parser::parse_bazel_xml;

    use super::*;

    #[test]
    fn test_bazel_dep_editor() {
        let query = parse_bazel_xml(&read_test_data!("cxx-deps.xml")).unwrap();
        let mut editor = BazelDepEditor::new(
            &query,
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

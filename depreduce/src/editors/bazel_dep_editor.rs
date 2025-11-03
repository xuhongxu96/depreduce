// Also supports Buck

use std::collections::{HashMap, HashSet};
use std::path::Path;

use normalize_path::NormalizePath;
use rustpython_parser::Parse;
use rustpython_parser::ast::Ranged;
use serde::{Deserialize, Serialize};

use crate::editors::{DepEditor, FileEdit};
use crate::graph::bazel_xml_parser::{BazelQuery, SkyValue};
use crate::graph::buck_json_parser::BuckQuery;

pub struct BazelDepEditor {
    label2location: HashMap<String, String>,
    workspace_root: String,
    keywords_for_deps_removal: HashSet<String>,
    keywords_for_deps_insertion: HashSet<String>,
    buck_mode: bool,
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct BazelLabel {
    pub name: String,
    pub package: String,
    pub repo: String,
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

    fn to_range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

impl BazelLabel {
    pub fn parse(label: &str) -> BazelLabel {
        let mut res = BazelLabel {
            name: String::new(),
            package: String::new(),
            repo: String::new(),
        };

        if label.starts_with("@") {
            if let Some(i) = label.find('/') {
                let mut res = BazelLabel::parse(&format!("/{}", &label[i + 1..]));
                res.repo = label[..i].to_string();
                return res;
            } else {
                res.repo = label.to_string();
                res.name = label.strip_prefix("@").unwrap_or(label).to_string();
                return res;
            }
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

fn buck_target_to_bazel_label(target: &str) -> BazelLabel {
    let mut repo = "root".to_string();

    let remaining;
    if !target.contains("//") {
        assert!(target.starts_with(':'));
        return BazelLabel {
            name: target.strip_prefix(':').unwrap().to_string(),
            package: "".to_string(),
            repo: "root".to_string(),
        };
    } else if !target.starts_with("//") {
        let mut parts = target.split("//");
        repo = parts.next().unwrap().to_string();
        remaining = parts
            .next()
            .expect(&format!("Failed to get remaining path from {}", target));
    } else {
        remaining = &target[2..];
    }

    let mut parts = remaining.split(":");
    let package = parts.next().unwrap().to_string();
    let name = parts.next().unwrap().to_string();

    BazelLabel {
        name,
        package: format!("//{}", package),
        repo,
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

pub fn split_location(location: &str) -> (String, usize, usize) {
    let parts: Vec<&str> = location.split(':').collect();
    assert_eq!(parts.len(), 3, "Invalid location format: {}", location);

    let path = parts[0];
    let start_line = parts[1].parse::<usize>().unwrap_or(0);
    let end_col = parts[2].parse::<usize>().unwrap_or(0);

    (path.to_string(), start_line, end_col)
}

fn get_list_insert_pos(
    expr: &rustpython_parser::ast::Expr,
    keywords: &HashSet<String>,
) -> Option<usize> {
    use rustpython_parser::ast;

    match expr {
        ast::Expr::Call(e) => {
            let dep_kw = e
                .keywords
                .iter()
                .filter(|kw| {
                    if let Some(ident) = &kw.arg {
                        keywords.contains(ident.as_str())
                    } else {
                        false
                    }
                })
                .next();
            if let Some(kw) = dep_kw {
                return get_list_insert_pos(&kw.value, keywords);
            }
        }
        ast::Expr::List(e) => {
            return Some(e.range.start().to_usize());
        }
        _ => {}
    }
    None
}

pub fn get_fn_name_and_rule_name(expr: &rustpython_parser::ast::Expr) -> Option<(String, String)> {
    use rustpython_parser::ast;

    let mut name = None;
    let mut rule = None;

    match expr {
        ast::Expr::Call(e) => {
            if let Some(n) = e.func.as_name_expr().map(|n| n.id.as_str()) {
                rule = Some(n.to_string());
            }

            name = e.keywords.iter().find_map(|kw| {
                if let Some(ident) = &kw.arg {
                    if ident == "name" {
                        if let ast::Expr::Constant(c) = &kw.value {
                            return Some(c.value.as_str().cloned().unwrap_or_default());
                        }
                    }
                }
                None
            });
        }
        _ => {}
    };

    if let Some(name) = name {
        if let Some(rule) = rule {
            return Some((name, rule));
        }
    }

    None
}

fn extract_list_items(
    expr: &rustpython_parser::ast::Expr,
    keywords: &HashSet<String>,
) -> Vec<(String, Interval)> {
    use rustpython_parser::ast;

    let mut res = vec![];

    match expr {
        ast::Expr::Call(e) => {
            e.keywords
                .iter()
                .filter(|kw| {
                    kw.arg
                        .as_ref()
                        .map(|ident| keywords.contains(ident.as_str()))
                        .unwrap_or(false)
                })
                .for_each(|kw| {
                    res.extend(extract_list_items(&kw.value, keywords));
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
                last.1.end = e.range().end().to_usize() - 1;
            }
        }
        _ => {}
    }

    res
}

impl BazelDepEditor {
    pub fn new(
        label2location: HashMap<String, String>,
        workspace_root: &str,
        keywords_for_deps_insertion: HashSet<String>,
        keywords_for_deps_removal: HashSet<String>,
    ) -> Self {
        Self::new_with_buck_mode(
            label2location,
            workspace_root,
            keywords_for_deps_insertion,
            keywords_for_deps_removal,
            false,
        )
    }

    pub fn new_with_buck_mode(
        label2location: HashMap<String, String>,
        workspace_root: &str,
        keywords_for_deps_insertion: HashSet<String>,
        keywords_for_deps_removal: HashSet<String>,
        buck_mode: bool,
    ) -> Self {
        Self {
            label2location,
            workspace_root: Path::new(workspace_root)
                .normalize()
                .to_str()
                .unwrap()
                .to_string(),
            keywords_for_deps_insertion,
            keywords_for_deps_removal,
            buck_mode,
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

    fn get_insertion_pos(
        &self,
        label: &str,
        path: &str,
        build_content: &str,
        start_line: usize,
        keywords: &HashSet<String>,
    ) -> Option<usize> {
        use rustpython_parser::ast;

        // convert start_line and end_line to char offsets
        let start_offset = if self.buck_mode {
            0
        } else {
            build_content
                .lines()
                .take(start_line - 1)
                .map(|s| s.len() + 1)
                .sum::<usize>()
        };

        let mut res = None;
        let ast = rustpython_parser::ast::Suite::parse(&build_content, path).unwrap();
        for stmt in ast {
            if stmt.range().start().to_usize() < start_offset {
                continue;
            }
            match stmt {
                ast::Stmt::Expr(e) => {
                    if let Some((label_name, _)) = get_fn_name_and_rule_name(&e.value) {
                        let label = if self.buck_mode {
                            buck_target_to_bazel_label(label)
                        } else {
                            BazelLabel::parse(label)
                        };
                        if label.name == label_name {
                            res = get_list_insert_pos(&e.value, keywords);
                        }
                    }
                }
                _ => {}
            }
            if self.buck_mode && res.is_none() {
                // as there is no start_line for buck mode,
                // we need to check all statements to find the target
            } else {
                break;
            }
        }

        if let Some(pos) = res {
            let offset = &build_content[pos..].find('[').unwrap() + 1;
            Some(pos + offset)
        } else {
            None
        }
    }

    fn extract_all_labels(
        &self,
        label: &str,
        path: &str,
        build_content: &str,
        start_line: usize,
        keywords: &HashSet<String>,
    ) -> Vec<(BazelLabel, Interval)> {
        use rustpython_parser::ast;

        // convert start_line and end_line to char offsets
        let start_offset = if self.buck_mode {
            0
        } else {
            build_content
                .lines()
                .take(start_line - 1)
                .map(|s| s.len() + 1)
                .sum::<usize>()
        };

        let mut res: Option<Vec<(String, Interval)>> = None;
        let ast = rustpython_parser::ast::Suite::parse(&build_content, path).unwrap();
        for stmt in ast {
            if stmt.range().start().to_usize() < start_offset {
                continue;
            }

            match stmt {
                ast::Stmt::Expr(e) => {
                    if let Some((label_name, _)) = get_fn_name_and_rule_name(&e.value) {
                        let label = if self.buck_mode {
                            buck_target_to_bazel_label(label)
                        } else {
                            BazelLabel::parse(label)
                        };
                        if label.name == label_name {
                            res = Some(extract_list_items(&e.value, keywords));
                        }
                    }
                }
                _ => {}
            }

            if self.buck_mode && res.is_none() {
                // as there is no start_line for buck mode,
                // we need to check all statements to find the target
            } else {
                break;
            }
        }

        res.unwrap_or(vec![])
            .iter()
            .map(|(label, interval)| {
                let label = if self.buck_mode {
                    buck_target_to_bazel_label(label)
                } else {
                    BazelLabel::parse(label)
                };
                let normalized_label = self.normalize_label(&label, path);
                (normalized_label, interval.clone())
            })
            .collect()
    }

    fn simplify_label(&self, dep_label: &str, path: &str) -> Option<String> {
        let mut simplified_label = None;
        if let Some(dep_location) = self.label2location.get(dep_label) {
            let (dep_path, _dep_start_line, _dep_end_col) = if self.buck_mode {
                (dep_location.clone(), 0, 0)
            } else {
                split_location(dep_location)
            };
            if dep_path == path {
                let dep_label = if self.buck_mode {
                    buck_target_to_bazel_label(dep_label)
                } else {
                    BazelLabel::parse(dep_label)
                };
                simplified_label = Some(format!(":{}", dep_label.name));
            }
        }

        let dep_label = if self.buck_mode {
            buck_target_to_bazel_label(dep_label)
        } else {
            BazelLabel::parse(dep_label)
        };
        if dep_label.package.split('/').last().unwrap() == dep_label.name {
            simplified_label = Some(format!("{}{}", dep_label.repo, dep_label.package));
        }

        simplified_label
    }
}

impl DepEditor for BazelDepEditor {
    fn add(
        &self,
        label: &str,
        dep_label: &str,
        _original_dependent_label: &str,
    ) -> Result<FileEdit, String> {
        if let Some(location) = self.label2location.get(label) {
            let (path, start_line, _end_col) = if self.buck_mode {
                (location.clone(), 0, 0)
            } else {
                split_location(location)
            };
            if !Path::new(&path)
                .normalize()
                .starts_with(Path::new(&self.workspace_root))
            {
                return Err(format!(
                    "Path '{}' is not within the workspace root '{}'",
                    path, self.workspace_root
                ));
            }

            let simplified_label = if self.buck_mode {
                None
            } else {
                self.simplify_label(dep_label, &path)
            };

            let build = std::fs::read_to_string(&path).unwrap();
            if let Some(pos) = self.get_insertion_pos(
                label,
                &path,
                &build,
                start_line,
                &self.keywords_for_deps_insertion,
            ) {
                Ok(FileEdit {
                    path: path,
                    content: format!(
                        "{}\"{}\",{}",
                        &build[..pos],
                        simplified_label
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or(dep_label),
                        &build[pos..]
                    ),
                    desp: format!("Add dependency '{}' to label '{}'", dep_label, label),
                })
            } else {
                Err(format!("Label '{}' does not have 'deps' field", label))
            }
        } else {
            Err(format!("Label '{}' not found", label))
        }
    }

    fn remove(&self, label: &str, dep_label: &str) -> Result<FileEdit, String> {
        if let Some(location) = self.label2location.get(label) {
            let (path, start_line, _end_col) = if self.buck_mode {
                (location.clone(), 0, 0)
            } else {
                split_location(location)
            };
            if !Path::new(&path)
                .normalize()
                .starts_with(Path::new(&self.workspace_root))
            {
                return Err(format!(
                    "Path '{}' is not within the workspace root '{}'",
                    path, self.workspace_root
                ));
            }
            let mut build = std::fs::read_to_string(&path).unwrap();
            let candidate_labels = self.extract_all_labels(
                &label,
                &path,
                &build,
                start_line,
                &self.keywords_for_deps_removal,
            );

            if let Some((_label, interval)) = candidate_labels
                .iter()
                .find(|(l, _)| l.to_string() == dep_label)
            {
                // replace interval of build as spaces with the same length of the interval, but keep \n as \n
                let mut replacement = String::new();
                for c in build[interval.to_range()].chars() {
                    if c.is_whitespace() {
                        replacement.push(c);
                    }
                }
                build.replace_range(interval.to_range(), &replacement);
                Ok(FileEdit {
                    path,
                    content: build,
                    desp: format!("Remove dependency '{}' from label '{}'", dep_label, label),
                })
            } else {
                Err(format!("Dependency Label '{}' not found", dep_label))
            }
        } else {
            Err(format!("Label '{}' not found", label))
        }
    }
}

pub fn generate_label2location_for_bazel(query: &BazelQuery) -> HashMap<String, String> {
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
                label2location.insert(generated_file.name.clone(), generated_file.location.clone());
            }
            SkyValue::PackageGroup(_package_group) => {}
        }
    }
    label2location
}

pub fn generate_label2location_for_buck(
    query: &BuckQuery,
    workspace: &str,
) -> HashMap<String, String> {
    let mut label2location = HashMap::new();
    for (name, target) in &query.query {
        label2location.insert(
            name.clone(),
            format!("{}/{}", workspace, target.to_buck_path().unwrap()),
        );
    }
    label2location
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use utils::{get_test_data_path, read_or_create_test_data, read_test_data};

    use crate::graph::bazel_xml_parser::parse_bazel_xml_query;

    use super::*;

    fn get_test_workspace_root() -> String {
        get_test_data_path!("../../../examples/simple-cxx-project")
            .normalize()
            .to_str()
            .unwrap()
            .to_string()
    }

    #[fixture]
    #[once]
    fn fake_query() -> BazelQuery {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><query version=\"2\"><source-file location=\"\" name=\"\"></source-file></query>";
        parse_bazel_xml_query(xml).unwrap()
    }

    #[fixture]
    #[once]
    fn cxx_query() -> BazelQuery {
        let mut xml = read_test_data!("cxx-deps.xml");
        xml = xml.replace(
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
            &get_test_workspace_root(),
        );
        parse_bazel_xml_query(&xml).unwrap()
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
    fn test_normalize_label(fake_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(fake_query),
            "/data/h445xu/repo/bazel-dep-reduce/examples/simple-cxx-project",
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
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
    fn test_extract_all_labels(cxx_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(cxx_query),
            &get_test_workspace_root(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        );

        let path = format!("{}/main/BUILD", get_test_workspace_root());
        let labels = editor.extract_all_labels(
            "main",
            &path,
            &std::fs::read_to_string(&path).unwrap(),
            3,
            &editor.keywords_for_deps_removal,
        );
        let res = format!("{:#?}", labels);
        assert_eq!(
            res,
            read_or_create_test_data!("dep_editors/bazel_dep_editor/extract_all_labels.out", res)
        );
    }

    #[rstest]
    fn test_extract_all_labels_2(fake_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(fake_query),
            get_test_data_path!("").to_str().unwrap(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        );
        let labels = editor.extract_all_labels(
            "main",
            get_test_data_path!("test.BUILD").to_str().unwrap(),
            &read_test_data!("test.BUILD"),
            3,
            &editor.keywords_for_deps_removal,
        );
        let res = format!("{:#?}", labels);
        assert_eq!(
            res,
            read_or_create_test_data!("dep_editors/bazel_dep_editor/extract_all_labels_2.out", res)
        );
    }

    #[rstest]
    fn test_get_insertion_pos(fake_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(fake_query),
            get_test_data_path!("").to_str().unwrap(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        );
        let pos = editor.get_insertion_pos(
            "main",
            get_test_data_path!("test.BUILD").to_str().unwrap(),
            &read_test_data!("test.BUILD"),
            3,
            &editor.keywords_for_deps_removal,
        );
        assert_eq!(pos, Some(119));
    }

    #[rstest]
    fn test_bazel_dep_editor_remove(cxx_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(cxx_query),
            &get_test_workspace_root(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        );
        let edit = editor.remove("//main:main", "//liba:liba").unwrap();
        assert_eq!(
            edit.path,
            format!("{}/main/BUILD", get_test_workspace_root())
        );
        assert_eq!(
            edit.content,
            read_or_create_test_data!(
                "dep_editors/bazel_dep_editor/remove_main_liba.BUILD",
                edit.content
            )
        );
    }

    #[rstest]
    fn test_bazel_dep_editor_add(cxx_query: &BazelQuery) {
        let editor = BazelDepEditor::new(
            generate_label2location_for_bazel(cxx_query),
            &get_test_workspace_root(),
            HashSet::from(["deps".to_string()]),
            HashSet::from(["deps".to_string()]),
        );
        let edit = editor.add("//main:main", "//libc:libc", "unused").unwrap();
        assert_eq!(
            edit.path,
            format!("{}/main/BUILD", get_test_workspace_root())
        );
        assert_eq!(
            edit.content,
            read_or_create_test_data!(
                "dep_editors/bazel_dep_editor/add_main_libc.BUILD",
                edit.content
            )
        );
    }

    #[test]
    fn test_buck_target_to_bazel_label() {
        let cases = vec![
            (
                "root//liba:liba",
                BazelLabel {
                    name: "liba".to_string(),
                    package: "//liba".to_string(),
                    repo: "root".to_string(),
                },
            ),
            (
                "//main:main",
                BazelLabel {
                    name: "main".to_string(),
                    package: "//main".to_string(),
                    repo: "root".to_string(),
                },
            ),
            (
                "prelude//pkg/pkg:target",
                BazelLabel {
                    name: "target".to_string(),
                    package: "//pkg/pkg".to_string(),
                    repo: "prelude".to_string(),
                },
            ),
        ];
        for (input, expected) in cases {
            let result = buck_target_to_bazel_label(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}

use std::{process::Command, rc::Rc};

use cargo_metadata::PackageId;

use crate::editors::{DepEditor, FileEdit};

pub struct CargoDepEditor {
    metadata: Rc<cargo_metadata::Metadata>,
}

impl CargoDepEditor {
    pub fn new(metadata: Rc<cargo_metadata::Metadata>) -> Self {
        Self { metadata }
    }
}

impl DepEditor for CargoDepEditor {
    fn add(
        &self,
        label: &str,
        dep_label: &str,
        original_dependent_label: &str,
    ) -> Result<FileEdit, String> {
        let src_pkg_id = PackageId {
            repr: label.to_string(),
        };
        let dep_pkg_id = PackageId {
            repr: dep_label.to_string(),
        };
        let original_dep_pkg_id = PackageId {
            repr: original_dependent_label.to_string(),
        };

        let src_pkg = &self.metadata[&src_pkg_id];
        let dep_pkg = &self.metadata[&dep_pkg_id];
        let original_dep_pkg = &self.metadata[&original_dep_pkg_id];

        let possible_deps = original_dep_pkg
            .dependencies
            .iter()
            .filter(|d| &d.name == dep_pkg.name.as_str() && d.req.matches(&dep_pkg.version))
            .collect::<Vec<_>>();

        if possible_deps.is_empty() {
            return Err(format!(
                "Dependency {}@{} not found in original dependent package {}",
                dep_pkg.name, dep_pkg.version, original_dep_pkg.name
            ));
        } else if possible_deps.len() > 1 {
            return Err(format!(
                "Multiple dependencies {:?} found in original dependent package {} match {}@{}. Ambiguous which one to add.",
                possible_deps
                    .iter()
                    .map(|d| format!("{}@{}", &d.name, &d.req)),
                original_dep_pkg.name,
                dep_pkg.name,
                dep_pkg.version
            ));
        }

        let dep_info = possible_deps.first().unwrap();

        let features = dep_info.features.join(",");

        let mut cmd = Command::new("cargo");

        let version_req = dep_info.req.to_string();
        let dep_name = if version_req.is_empty() {
            dep_pkg.name.to_string()
        } else {
            format!("{}@{}", dep_pkg.name, version_req)
        };
        cmd.arg("add")
            .arg(dep_name.as_str())
            .arg("--manifest-path")
            .arg(src_pkg.manifest_path.as_str());

        if dep_info.optional {
            cmd.arg("--optional");
        }

        if let Some(rename) = &dep_info.rename {
            cmd.arg("--rename").arg(rename.as_str());
        }

        if !dep_info.uses_default_features {
            cmd.arg("--features").arg(features.as_str());
        }

        if let Some(path) = &dep_info.path {
            cmd.arg("--path").arg(path.as_str());
        }

        if let Some(registry) = &dep_info.registry {
            cmd.arg("--registry").arg(registry.as_str());
        }

        match dep_info.kind {
            cargo_metadata::DependencyKind::Development => {
                cmd.arg("--dev");
            }
            cargo_metadata::DependencyKind::Build => {
                cmd.arg("--build");
            }
            _ => {}
        }

        if let Some(platform) = &dep_info.target {
            match platform {
                cargo_metadata::cargo_platform::Platform::Name(name) => {
                    cmd.arg("--target").arg(name.as_str());
                }
                cargo_metadata::cargo_platform::Platform::Cfg(cfg_expr) => {
                    let cfg_str = format!("{}", cfg_expr);
                    cmd.arg("--cfg").arg(cfg_str.as_str());
                }
            }
        }

        cmd.output()
            .map_err(|e| format!("Failed to execute cargo add: {}", e))?;

        Ok(FileEdit {
            path: src_pkg.manifest_path.to_string(),
            content: String::new(),
            desp: format!(
                "Added dependency {}@{} to {}@{}",
                dep_pkg.name, dep_pkg.version, src_pkg.name, src_pkg.version
            ),
        })
    }

    fn remove(&self, label: &str, dep_label: &str) -> Result<FileEdit, String> {
        let src_pkg_id = PackageId {
            repr: label.to_string(),
        };
        let dep_pkg_id = PackageId {
            repr: dep_label.to_string(),
        };

        let src_pkg = &self.metadata[&src_pkg_id];
        let dep_pkg = &self.metadata[&dep_pkg_id];

        Command::new("cargo")
            .arg("remove")
            .arg(dep_pkg.name.as_str())
            .arg("--manifest-path")
            .arg(src_pkg.manifest_path.as_str())
            .output()
            .map_err(|e| format!("Failed to execute cargo remove: {}", e))?;

        Ok(FileEdit {
            path: src_pkg.manifest_path.to_string(),
            content: String::new(),
            desp: format!(
                "Removed dependency {}@{} from {}@{}",
                dep_pkg.name, dep_pkg.version, src_pkg.name, src_pkg.version
            ),
        })
    }
}

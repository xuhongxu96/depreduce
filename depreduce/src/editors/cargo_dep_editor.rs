use std::{collections::HashMap, fs::read_to_string, path::Path, rc::Rc};

use cargo_metadata::PackageId;
use normalize_path::NormalizePath;
use toml_edit::DocumentMut;

use crate::editors::{DepEditor, FileEdit};

pub struct CargoDepEditor {
    workspace: String,
    metadata: Rc<cargo_metadata::Metadata>,
    target_dep_items: HashMap<String, toml_edit::Item>,
    target_dev_dep_items: HashMap<String, toml_edit::Item>,
    reduce_dev_deps: bool,
}

impl CargoDepEditor {
    pub fn new(
        workspace: String,
        metadata: Rc<cargo_metadata::Metadata>,
        reduce_dev_deps: bool,
    ) -> Self {
        let mut target_dep_items = HashMap::new();
        let mut target_dev_dep_items = HashMap::new();

        for pkg in &metadata.packages {
            let id = pkg.id.repr.clone();
            let cargo_toml = read_to_string(pkg.manifest_path.as_str()).unwrap();
            let doc = cargo_toml.parse::<DocumentMut>().unwrap();

            if let Some(deps_table) = doc.get("dependencies").and_then(|d| d.as_table()) {
                target_dep_items.insert(id.clone(), deps_table.clone().into());
            }

            if reduce_dev_deps {
                if let Some(dev_deps_table) = doc.get("dev-dependencies").and_then(|d| d.as_table())
                {
                    target_dev_dep_items.insert(id, dev_deps_table.clone().into());
                }
            }
        }

        Self {
            workspace,
            metadata,
            target_dep_items,
            target_dev_dep_items,
            reduce_dev_deps: reduce_dev_deps,
        }
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
        let src_pkg = &self.metadata[&src_pkg_id];

        if !src_pkg
            .manifest_path
            .as_std_path()
            .normalize()
            .starts_with(Path::new(&self.workspace))
        {
            return Err(format!(
                "Path '{}' is not within the workspace root '{}'",
                src_pkg.manifest_path.as_str(),
                self.workspace
            ));
        }

        let dep_pkg_id = PackageId {
            repr: dep_label.to_string(),
        };
        let original_dep_pkg_id = PackageId {
            repr: original_dependent_label.to_string(),
        };

        let dep_pkg = &self.metadata[&dep_pkg_id];
        let original_dep_pkg = &self.metadata[&original_dep_pkg_id];
        let original_content = read_to_string(src_pkg.manifest_path.as_str()).unwrap();

        let mut doc = original_content.parse::<DocumentMut>().map_err(|e| {
            format!(
                "Failed to parse TOML document {}: {:?}",
                src_pkg.manifest_path.as_str(),
                e
            )
        })?;

        if let Some(item) = self.target_dep_items.get(&original_dep_pkg.id.repr) {
            doc["dependencies"][dep_pkg.name.as_str()] = item.clone();
        } else if let Some(item) = self.target_dev_dep_items.get(&original_dep_pkg.id.repr) {
            if !self.reduce_dev_deps {
                return Err(format!(
                    "Dependency {} is dev deps in target package {}, but reduce_dev_deps is false",
                    original_dep_pkg.name, original_dependent_label
                ));
            }
            doc["dev-dependencies"][dep_pkg.name.as_str()] = item.clone();
        } else {
            return Err(format!(
                "Dependency {} not found in target package {}",
                original_dep_pkg.name, original_dependent_label
            ));
        };

        Ok(FileEdit {
            path: src_pkg.manifest_path.to_string(),
            content: doc.to_string(),
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
        let src_pkg = &self.metadata[&src_pkg_id];

        if !src_pkg
            .manifest_path
            .as_std_path()
            .normalize()
            .starts_with(Path::new(&self.workspace))
        {
            return Err(format!(
                "Path '{}' is not within the workspace root '{}'",
                src_pkg.manifest_path.as_str(),
                self.workspace
            ));
        }

        let dep_pkg_id = PackageId {
            repr: dep_label.to_string(),
        };

        let dep_pkg = &self.metadata[&dep_pkg_id];
        let original_content = read_to_string(src_pkg.manifest_path.as_str()).unwrap();

        let mut doc = original_content.parse::<DocumentMut>().map_err(|e| {
            format!(
                "Failed to parse TOML document {}: {:?}",
                src_pkg.manifest_path.as_str(),
                e
            )
        })?;

        let deps = doc["dependencies"]
            .as_table_mut()
            .and_then(|table| table.remove(dep_pkg.name.as_str()));

        let dev_deps = if self.reduce_dev_deps {
            doc["dev-dependencies"]
                .as_table_mut()
                .and_then(|table| table.remove(dep_pkg.name.as_str()))
        } else {
            None
        };

        if deps.is_none() && dev_deps.is_none() {
            return Err(format!(
                "Dependency {} not found in {}",
                dep_pkg.name,
                src_pkg.manifest_path.as_str()
            ));
        }

        Ok(FileEdit {
            path: src_pkg.manifest_path.to_string(),
            content: doc.to_string(),
            desp: format!(
                "Removed dependency {}@{} from {}@{}",
                dep_pkg.name, dep_pkg.version, src_pkg.name, src_pkg.version
            ),
        })
    }
}

use serde::{Deserialize, Serialize};

type CheckpointId = usize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,

    #[serde(skip)]
    pub content: String,

    pub desp: String,
}

pub trait DepEditor {
    fn add(&self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
    fn remove(&self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
}

mod bazel_dep_editor;

pub use bazel_dep_editor::*;

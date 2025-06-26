type CheckpointId = usize;

pub struct FileEdit {
    pub path: String,
    pub content: String,
}

pub trait DepEditor {
    fn add(&self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
    fn remove(&self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
}

mod bazel_dep_editor;

pub use bazel_dep_editor::BazelDepEditor;

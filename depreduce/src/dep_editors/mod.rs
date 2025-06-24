type CheckpointId = usize;

struct FileEdit {
    path: String,
    content: String,
}

trait DepEditor {
    fn add(&mut self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
    fn remove(&mut self, label: &str, dep_label: &str) -> Result<FileEdit, String>;
}

mod bazel_dep_editor;

pub use bazel_dep_editor::BazelDepEditor;

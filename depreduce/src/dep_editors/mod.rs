type CheckpointId = usize;

type ChangedFiles = Vec<String>;

trait DepEditor {
    fn add(&mut self, label: &str, dep_label: &str) -> Result<ChangedFiles, String>;
    fn remove(&mut self, label: &str, dep_label: &str) -> Result<ChangedFiles, String>;
    fn save(&mut self) -> CheckpointId;
    fn restore(&mut self, checkpoint_id: CheckpointId);
}

mod bazel_dep_editor;

pub use bazel_dep_editor::BazelDepEditor;

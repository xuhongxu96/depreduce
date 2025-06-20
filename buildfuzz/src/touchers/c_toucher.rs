use crate::touchers::{TouchConditionByExtension, ToucherByAppend};

pub struct CToucher {}

impl ToucherByAppend for CToucher {
    fn content_to_append(&self) -> String {
        "\n\nstatic int TOUCHER_RANDOM(){static int i = 0; i += 1; return i;}".to_string()
    }
}

impl TouchConditionByExtension for CToucher {
    fn should_touch_by_extension(&self, ext: &str) -> bool {
        ext == "c" || ext == "h" || ext == "cpp" || ext == "cxx" || ext == "cc" || ext == "hpp"
    }
}

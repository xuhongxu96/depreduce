use crate::touchers::{TouchConditionByExtension, ToucherByAppend};

pub struct KotlinToucher {}

impl ToucherByAppend for KotlinToucher {
    fn content_to_append(&self) -> String {
        "\n\nclass DummyClassForTouch {}".to_string()
    }
}

impl TouchConditionByExtension for KotlinToucher {
    fn should_touch_by_extension(&self, ext: &str) -> bool {
        ext == "kt"
    }
}

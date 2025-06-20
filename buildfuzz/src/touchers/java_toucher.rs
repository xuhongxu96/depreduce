use crate::touchers::{TouchConditionByExtension, ToucherByAppend};

pub struct JavaToucher {}

impl ToucherByAppend for JavaToucher {
    fn content_to_append(&self) -> String {
        "\n\nfinal class DummyClassForTouch {}".to_string()
    }
}

impl TouchConditionByExtension for JavaToucher {
    fn should_touch_by_extension(&self, ext: &str) -> bool {
        ext == "java"
    }
}

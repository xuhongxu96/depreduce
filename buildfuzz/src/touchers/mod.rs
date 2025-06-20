use std::path::Path;

pub trait TouchCondition {
    fn should_touch(&self, file: &str) -> bool;
}

pub trait TouchConditionByExtension: TouchCondition {
    fn should_touch_by_extension(&self, ext: &str) -> bool;
}

impl<T: TouchConditionByExtension> TouchCondition for T {
    fn should_touch(&self, file: &str) -> bool {
        Path::new(file)
            .extension()
            .and_then(|ext| ext.to_str())
            .map_or(false, |ext| self.should_touch_by_extension(ext))
    }
}

pub trait Toucher: TouchCondition {
    fn touch(&self, file: &str);
}

pub mod c_toucher;

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

pub trait ToucherByAppend: Toucher {
    fn content_to_append(&self) -> String;
}

impl<T: ToucherByAppend> Toucher for T {
    fn touch(&self, file: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .append(true)
            .open(file)
            .expect("Failed to open file for touching");

        writeln!(file, "{}", self.content_to_append()).expect("Failed to write to file");
    }
}

pub mod c_toucher;
pub mod java_toucher;

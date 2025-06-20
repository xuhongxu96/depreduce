use crate::touchers::{TouchConditionByExtension, Toucher};

pub struct CToucher {}

impl Toucher for CToucher {
    fn touch(&self, file: &str) {
        // add a comment line to the file
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .append(true)
            .open(file)
            .expect("Failed to open file for touching");
        let code = "\n\nstatic int TOUCHER_RANDOM(){static int i = 0; i += 1; return i;}";
        writeln!(file, "{}", code).expect("Failed to write to file");
    }
}

impl TouchConditionByExtension for CToucher {
    fn should_touch_by_extension(&self, ext: &str) -> bool {
        ext == "c" || ext == "h" || ext == "cpp" || ext == "cxx" || ext == "cc" || ext == "hpp"
    }
}

use normalize_path::NormalizePath;

use crate::syntax;

fn is_address(x: &str) -> bool {
    x.starts_with("0x")
}

fn is_null(x: &str) -> bool {
    x == "NULL"
}

fn ignore_pathname(pathname: &str) -> bool {
    is_null(pathname)
}

pub fn extract_arg(args: &str, index: usize) -> &str {
    args.strip_prefix("(")
        .unwrap_or(args)
        .strip_suffix(")")
        .unwrap_or(args)
        .split(", ")
        .nth(index)
        .unwrap()
}

pub fn extract_pathname(args: &str, index: usize) -> Option<syntax::Path> {
    let pathname_str = extract_arg(args, index);
    if ignore_pathname(pathname_str) {
        return None;
    }

    if is_address(pathname_str) {
        return Some(syntax::Path::Unknown("/UNKNOWN".to_string()));
    }

    let path = pathname_str
        .strip_prefix("\"")
        .unwrap_or(pathname_str)
        .strip_suffix("\"")
        .unwrap_or(pathname_str)
        .replace("//", "/");

    Some(syntax::Path::Path(
        std::path::Path::new(&path)
            .normalize()
            .into_os_string()
            .into_string()
            .unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::{strace_utils::extract_pathname, syntax};

    #[test]
    fn test_extract_pathname() {
        assert_eq!(
            extract_pathname("(\"/\", \"\", 3)", 0),
            Some(syntax::Path::Path("/".to_string()))
        );
        assert_eq!(
            extract_pathname("(\"/a/\", \"\", 3)", 0),
            Some(syntax::Path::Path("/a".to_string()))
        );
    }
}

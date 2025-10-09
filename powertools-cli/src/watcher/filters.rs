use std::path::Path;
use crate::core::Language;

/// Check if a path should be ignored by the watcher
pub fn should_ignore(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Ignore patterns
    let ignore_patterns = [
        ".git/",
        "target/",
        "node_modules/",
        ".scip",
        "dist/",
        "build/",
        ".next/",
        "__pycache__/",
        ".pytest_cache/",
        ".mypy_cache/",
        "venv/",
        ".venv/",
        ".idea/",
        ".vscode/",
        ".DS_Store",
    ];

    for pattern in &ignore_patterns {
        if path_str.contains(pattern) {
            return true;
        }
    }

    false
}

/// Detect language from file path
pub fn detect_language_from_path(path: &Path) -> Option<Language> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "py" | "pyi" => Some(Language::Python),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" => Some(Language::Cpp),
            "c" => Some(Language::C),
            _ => None,
        })
}

/// Check if a file extension is relevant for watching
pub fn is_relevant_file(path: &Path) -> bool {
    !should_ignore(path) && detect_language_from_path(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_should_ignore() {
        assert!(should_ignore(Path::new("target/debug/foo")));
        assert!(should_ignore(Path::new("node_modules/foo/bar.js")));
        assert!(should_ignore(Path::new(".git/HEAD")));
        assert!(should_ignore(Path::new("index.scip")));
        assert!(!should_ignore(Path::new("src/main.rs")));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(
            detect_language_from_path(Path::new("src/main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            detect_language_from_path(Path::new("src/app.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            detect_language_from_path(Path::new("test.py")),
            Some(Language::Python)
        );
        assert_eq!(detect_language_from_path(Path::new("README.md")), None);
    }

    #[test]
    fn test_is_relevant_file() {
        assert!(is_relevant_file(Path::new("src/main.rs")));
        assert!(is_relevant_file(Path::new("app.ts")));
        assert!(!is_relevant_file(Path::new("README.md")));
        assert!(!is_relevant_file(Path::new("target/debug/main.rs")));
    }
}

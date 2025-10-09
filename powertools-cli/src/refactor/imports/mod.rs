mod typescript;
mod python;
mod rust_lang;
mod cpp;

pub use typescript::TypeScriptImportAnalyzer;
pub use python::PythonImportAnalyzer;
pub use rust_lang::RustImportAnalyzer;
pub use cpp::CppImportAnalyzer;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a single import statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportStatement {
    /// Source module/file being imported (e.g., "react", "./utils", "std::collections")
    pub source: String,

    /// Symbols being imported (e.g., ["useState", "useEffect"])
    /// Empty for wildcard imports or C++ includes
    pub symbols: Vec<String>,

    /// Location in the file
    pub location: ImportLocation,

    /// Type of import
    pub kind: ImportKind,

    /// Optional alias for the import (e.g., "as np" in Python)
    pub alias: Option<String>,
}

/// Location of an import in a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportLocation {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Kind of import statement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportKind {
    /// Named imports (e.g., `import { foo, bar } from 'module'`)
    Named,

    /// Default import (e.g., `import React from 'react'`)
    Default,

    /// Namespace import (e.g., `import * as React from 'react'`)
    Namespace,

    /// Side-effect import (e.g., `import 'polyfills'`)
    SideEffect,

    /// CommonJS require (e.g., `const foo = require('module')`)
    Require,

    /// Python from import (e.g., `from module import foo`)
    FromImport,

    /// Python simple import (e.g., `import module`)
    SimpleImport,

    /// Rust use statement (e.g., `use std::collections::HashMap`)
    Use,

    /// C++ include (e.g., `#include <vector>` or `#include "header.h"`)
    Include,
}

/// Change to an import statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportChange {
    pub kind: ImportChangeKind,
    pub statement: ImportStatement,
}

/// Type of change to an import
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportChangeKind {
    Add,
    Remove,
    Modify,
}

/// Trait for language-specific import analyzers
pub trait ImportAnalyzer {
    /// Find all imports in a file
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>>;

    /// Add an import to a file, returning the new file content
    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String>;

    /// Remove an import from a file by symbol name, returning the new file content
    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String>;

    /// Update the source path of an import, returning the new file content
    fn update_import_path(&self, file: &Path, old_path: &str, new_path: &str) -> Result<String>;
}

/// Get the appropriate import analyzer for a file based on its extension
pub fn get_analyzer_for_file(file: &Path) -> Option<Box<dyn ImportAnalyzer>> {
    let ext = file.extension()?.to_str()?;

    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" => {
            Some(Box::new(TypeScriptImportAnalyzer::new()))
        }
        "py" | "pyi" => {
            Some(Box::new(PythonImportAnalyzer::new()))
        }
        "rs" => {
            Some(Box::new(RustImportAnalyzer::new()))
        }
        "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" | "hxx" => {
            Some(Box::new(CppImportAnalyzer::new()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_analyzer_for_typescript() {
        let file = PathBuf::from("test.ts");
        assert!(get_analyzer_for_file(&file).is_some());
    }

    #[test]
    fn test_get_analyzer_for_python() {
        let file = PathBuf::from("test.py");
        assert!(get_analyzer_for_file(&file).is_some());
    }

    #[test]
    fn test_get_analyzer_for_rust() {
        let file = PathBuf::from("test.rs");
        assert!(get_analyzer_for_file(&file).is_some());
    }

    #[test]
    fn test_get_analyzer_for_cpp() {
        let file = PathBuf::from("test.cpp");
        assert!(get_analyzer_for_file(&file).is_some());
    }

    #[test]
    fn test_get_analyzer_for_unknown() {
        let file = PathBuf::from("test.xyz");
        assert!(get_analyzer_for_file(&file).is_none());
    }
}

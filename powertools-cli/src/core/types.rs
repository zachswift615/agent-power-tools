use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<String>,
    pub signature: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    Trait,
    Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub location: Location,
    pub kind: ReferenceKind,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Definition,
    Reference,
    Read,
    Write,
    Call,
    Import,
    TypeReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub location: Location,
    pub matched_text: String,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub language: Language,
    pub node_type: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "ts" => Language::TypeScript,
            "tsx" => Language::TypeScript,
            "js" => Language::JavaScript,
            "jsx" => Language::JavaScript,
            "py" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "cpp" | "cxx" | "cc" => Language::Cpp,
            "c" | "h" => Language::C,
            _ => Language::Unknown,
        }
    }

    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Language::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
            Language::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Language::Java => Some(tree_sitter_java::LANGUAGE.into()),
            Language::Cpp | Language::C => Some(tree_sitter_cpp::LANGUAGE.into()),
            Language::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_symbols: usize,
    pub languages: Vec<(Language, usize)>,
    pub index_time_ms: u64,
    pub index_size_bytes: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub source: PathBuf,
    pub dependencies: Vec<PathBuf>,
    pub dependents: Vec<PathBuf>,
}
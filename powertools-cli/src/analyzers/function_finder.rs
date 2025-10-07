use anyhow::Result;
use std::path::Path;
use crate::core::{Symbol, SymbolKind, Location};
use crate::analyzers::{TreeSitterAnalyzer, FunctionInfo};

pub struct FunctionFinder {
    analyzer: TreeSitterAnalyzer,
}

impl FunctionFinder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyzer: TreeSitterAnalyzer::new()?,
        })
    }

    pub fn find_in_file(&mut self, file_path: &Path, include_private: bool) -> Result<Vec<Symbol>> {
        let functions = self.analyzer.find_functions(file_path)?;

        Ok(functions
            .into_iter()
            .filter(|f| include_private || f.is_public)
            .map(|f| self.function_to_symbol(f))
            .collect())
    }

    pub fn find_by_name(&mut self, file_path: &Path, name: &str) -> Result<Option<Symbol>> {
        let functions = self.analyzer.find_functions(file_path)?;

        Ok(functions
            .into_iter()
            .find(|f| f.name == name)
            .map(|f| self.function_to_symbol(f)))
    }

    fn function_to_symbol(&self, func: FunctionInfo) -> Symbol {
        let signature = self.build_signature(&func);

        Symbol {
            name: func.name.clone(),
            kind: SymbolKind::Function,
            location: func.location,
            container: None, // Could be enhanced to include module/class
            signature: Some(signature),
            documentation: None, // Could be extracted from comments
        }
    }

    fn build_signature(&self, func: &FunctionInfo) -> String {
        let params = func.parameters.join(", ");
        let return_type = func.return_type.as_ref()
            .map(|t| format!(" -> {}", t))
            .unwrap_or_default();

        format!("{}({}){}", func.name, params, return_type)
    }
}
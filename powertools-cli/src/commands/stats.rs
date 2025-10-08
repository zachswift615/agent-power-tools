use anyhow::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use ignore::WalkBuilder;
use crate::core::{output::OutputWriter, Language, IndexStats};

pub async fn run(
    path: Option<PathBuf>,
    _detailed: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));
    let output = OutputWriter::new(format);

    let mut total_files = 0;
    let mut language_counts: HashMap<Language, usize> = HashMap::new();

    let walker = WalkBuilder::new(&search_path)
        .standard_filters(true)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let lang = Language::from_extension(ext);
                if lang.tree_sitter_language().is_some() {
                    total_files += 1;
                    *language_counts.entry(lang).or_insert(0) += 1;
                }
            }
        }
    }

    let mut languages: Vec<(Language, usize)> = language_counts.into_iter().collect();
    languages.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    let stats = IndexStats {
        total_files,
        total_symbols: 0, // Would be populated from actual index
        languages,
        index_time_ms: 0,
        index_size_bytes: 0,
    };

    output.write_stats(&stats)?;

    Ok(())
}
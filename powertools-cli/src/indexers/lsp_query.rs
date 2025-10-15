use anyhow::{Context, Result};
use lsp_types::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use crate::core::{Location, Reference, ReferenceKind};
use crate::indexers::lsp_client::LspClient;

/// LSP-based query implementation for languages without SCIP indexers
///
/// This provides the same interface as ScipQuery but uses LSP protocol
/// for semantic navigation. Designed for Swift and other languages
/// where SCIP indexers are unavailable.
pub struct LspQuery {
    client: LspClient,
    project_root: PathBuf,
}

impl LspQuery {
    /// Create a new LSP query instance by starting an LSP server
    ///
    /// # Arguments
    /// * `command` - LSP server command (e.g., "sourcekit-lsp" for Swift)
    /// * `args` - Arguments to pass to the server
    /// * `project_root` - Project root directory
    ///
    /// # Returns
    /// An initialized LSP query instance ready to answer queries
    pub fn start(command: &str, args: Vec<String>, project_root: PathBuf) -> Result<Self> {
        let root_uri_str = format!("file://{}", project_root.display());
        let client = LspClient::start(command, &args, &root_uri_str)
            .with_context(|| format!("Failed to start LSP server: {}", command))?;

        Ok(Self {
            client,
            project_root,
        })
    }

    /// Find the definition of a symbol at a given location
    ///
    /// This matches the ScipQuery interface for compatibility.
    ///
    /// # Arguments
    /// * `file_path` - Absolute or relative path to the file
    /// * `line` - Line number (1-indexed, user convention)
    /// * `column` - Column number (1-indexed, user convention)
    ///
    /// # Returns
    /// Location of the definition, or None if not found
    pub fn find_definition(&mut self, file_path: &Path, line: usize, column: usize) -> Result<Option<Location>> {
        // Ensure file_path is absolute
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_root.join(file_path)
        };

        // Read the file to send to LSP server (required for textDocument/didOpen)
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("Failed to read file: {}", abs_path.display()))?;

        // Create URI for the file
        let uri_str = format!("file://{}", abs_path.display());
        let uri = Uri::from_str(&uri_str)
            .map_err(|e| anyhow::anyhow!("Invalid URI '{}': {}", uri_str, e))?;

        // Determine language ID from file extension
        let language_id = abs_path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "swift" => Some("swift"),
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                _ => None,
            })
            .unwrap_or("plaintext");

        // Notify LSP server about the file
        self.client.did_open(&uri, language_id, content)?;

        // LSP uses 0-indexed positions, convert from 1-indexed
        let lsp_line = (line.saturating_sub(1)) as u32;
        let lsp_char = (column.saturating_sub(1)) as u32;

        // Query for definition
        let locations = self.client.goto_definition(&uri, lsp_line, lsp_char)?;

        // Convert LSP Location to our Location type
        if let Some(lsp_location) = locations.first() {
            Ok(Some(self.lsp_location_to_location(lsp_location)?))
        } else {
            Ok(None)
        }
    }

    /// Find all references to a symbol
    ///
    /// Note: LSP requires a position to find references, unlike SCIP which can
    /// search by symbol name. This implementation requires you to provide a
    /// known location of the symbol first.
    ///
    /// # Arguments
    /// * `file_path` - File containing a usage of the symbol
    /// * `line` - Line number of symbol usage (1-indexed)
    /// * `column` - Column of symbol usage (1-indexed)
    /// * `include_declarations` - Whether to include the declaration
    ///
    /// # Returns
    /// List of all references to the symbol
    pub fn find_references_at_position(
        &mut self,
        file_path: &Path,
        line: usize,
        column: usize,
        include_declarations: bool,
    ) -> Result<Vec<Reference>> {
        // Ensure file_path is absolute
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_root.join(file_path)
        };

        // Read the file
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("Failed to read file: {}", abs_path.display()))?;

        // Create URI
        let uri_str = format!("file://{}", abs_path.display());
        let uri = Uri::from_str(&uri_str)
            .map_err(|e| anyhow::anyhow!("Invalid URI '{}': {}", uri_str, e))?;

        // Determine language ID
        let language_id = abs_path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "swift" => Some("swift"),
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                _ => None,
            })
            .unwrap_or("plaintext");

        // Notify LSP server
        self.client.did_open(&uri, language_id, content)?;

        // Convert to LSP coordinates (0-indexed)
        let lsp_line = (line.saturating_sub(1)) as u32;
        let lsp_char = (column.saturating_sub(1)) as u32;

        // Query for references
        let locations = self.client.find_references(&uri, lsp_line, lsp_char, include_declarations)?;

        // Convert LSP Locations to our Reference type
        locations.iter()
            .map(|lsp_loc| {
                let location = self.lsp_location_to_location(lsp_loc)?;
                Ok(Reference {
                    location,
                    kind: ReferenceKind::Reference, // LSP doesn't distinguish types
                    context: None,
                })
            })
            .collect()
    }

    /// Prepare to rename a symbol - validates that rename is possible
    ///
    /// # Arguments
    /// * `file_path` - File containing the symbol
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    ///
    /// # Returns
    /// true if rename is possible, false otherwise
    pub fn can_rename_symbol(
        &mut self,
        file_path: &Path,
        line: usize,
        column: usize,
    ) -> Result<bool> {
        // Ensure file_path is absolute
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_root.join(file_path)
        };

        // Read the file
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("Failed to read file: {}", abs_path.display()))?;

        // Create URI
        let uri_str = format!("file://{}", abs_path.display());
        let uri = Uri::from_str(&uri_str)
            .map_err(|e| anyhow::anyhow!("Invalid URI '{}': {}", uri_str, e))?;

        // Determine language ID
        let language_id = abs_path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "swift" => Some("swift"),
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                _ => None,
            })
            .unwrap_or("plaintext");

        // Notify LSP server
        self.client.did_open(&uri, language_id, content)?;

        // Convert to LSP coordinates (0-indexed)
        let lsp_line = (line.saturating_sub(1)) as u32;
        let lsp_char = (column.saturating_sub(1)) as u32;

        // Check if rename is possible
        let can_rename = self.client.prepare_rename(&uri, lsp_line, lsp_char)?;
        Ok(can_rename.is_some())
    }

    /// Rename a symbol at a given location
    ///
    /// This uses the LSP textDocument/rename to find all references and
    /// return the edits needed to perform the rename.
    ///
    /// # Arguments
    /// * `file_path` - File containing the symbol
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    /// * `new_name` - New name for the symbol
    ///
    /// # Returns
    /// WorkspaceEdit containing all file changes
    pub fn rename_symbol(
        &mut self,
        file_path: &Path,
        line: usize,
        column: usize,
        new_name: String,
    ) -> Result<WorkspaceEdit> {
        // Ensure file_path is absolute
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_root.join(file_path)
        };

        // Read the file
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("Failed to read file: {}", abs_path.display()))?;

        // Create URI
        let uri_str = format!("file://{}", abs_path.display());
        let uri = Uri::from_str(&uri_str)
            .map_err(|e| anyhow::anyhow!("Invalid URI '{}': {}", uri_str, e))?;

        // Determine language ID
        let language_id = abs_path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "swift" => Some("swift"),
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                _ => None,
            })
            .unwrap_or("plaintext");

        // Notify LSP server
        self.client.did_open(&uri, language_id, content)?;

        // Convert to LSP coordinates (0-indexed)
        let lsp_line = (line.saturating_sub(1)) as u32;
        let lsp_char = (column.saturating_sub(1)) as u32;

        // Send rename request
        self.client.rename(&uri, lsp_line, lsp_char, new_name)
    }

    /// Convert LSP Location to our Location type
    fn lsp_location_to_location(&self, lsp_location: &lsp_types::Location) -> Result<Location> {
        // Parse URI to get file path
        let uri_str = lsp_location.uri.as_str();
        let file_path = if uri_str.starts_with("file://") {
            PathBuf::from(&uri_str[7..]) // Strip "file://"
        } else {
            return Err(anyhow::anyhow!("Non-file URI not supported: {}", uri_str));
        };

        // LSP uses 0-indexed positions, convert to 1-indexed
        let line = (lsp_location.range.start.line + 1) as usize;
        let column = (lsp_location.range.start.character + 1) as usize;
        let end_line = Some((lsp_location.range.end.line + 1) as usize);
        let end_column = Some((lsp_location.range.end.character + 1) as usize);

        Ok(Location {
            file_path,
            line,
            column,
            end_line,
            end_column,
        })
    }

    /// Get available code actions at a position
    ///
    /// # Arguments
    /// * `file_path` - File to get code actions for
    /// * `start_line` - Start line (1-indexed)
    /// * `start_column` - Start column (1-indexed)
    /// * `end_line` - End line (1-indexed)
    /// * `end_column` - End column (1-indexed)
    /// * `only_kinds` - Optional filter for kinds (e.g., ["refactor.inline"])
    ///
    /// # Returns
    /// List of available code actions
    pub fn get_code_actions(
        &mut self,
        file_path: &Path,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        only_kinds: Option<Vec<lsp_types::CodeActionKind>>,
    ) -> Result<Vec<lsp_types::CodeActionOrCommand>> {
        // Ensure file_path is absolute
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_root.join(file_path)
        };

        // Read the file
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("Failed to read file: {}", abs_path.display()))?;

        // Create URI
        let uri_str = format!("file://{}", abs_path.display());
        let uri = Uri::from_str(&uri_str)
            .map_err(|e| anyhow::anyhow!("Invalid URI '{}': {}", uri_str, e))?;

        // Determine language ID
        let language_id = abs_path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "swift" => Some("swift"),
                "rs" => Some("rust"),
                "ts" | "tsx" => Some("typescript"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                _ => None,
            })
            .unwrap_or("plaintext");

        // Notify LSP server
        self.client.did_open(&uri, language_id, content)?;

        // Convert to LSP coordinates (0-indexed)
        let range = lsp_types::Range {
            start: lsp_types::Position {
                line: (start_line.saturating_sub(1)) as u32,
                character: (start_column.saturating_sub(1)) as u32,
            },
            end: lsp_types::Position {
                line: (end_line.saturating_sub(1)) as u32,
                character: (end_column.saturating_sub(1)) as u32,
            },
        };

        // Get code actions
        self.client.code_actions(&uri, range, only_kinds)
    }

    /// Gracefully shutdown the LSP server
    #[allow(dead_code)]
    pub fn shutdown(mut self) -> Result<()> {
        self.client.shutdown()
    }
}

impl Drop for LspQuery {
    fn drop(&mut self) {
        // Client will shutdown in its own Drop impl
    }
}

/// Apply a WorkspaceEdit to the filesystem
///
/// This function takes a WorkspaceEdit returned by LSP operations (like rename)
/// and applies all the text edits to their respective files.
///
/// # Arguments
/// * `workspace_edit` - The WorkspaceEdit to apply
///
/// # Returns
/// A tuple of (files_modified, total_edits_applied)
pub fn apply_workspace_edit(workspace_edit: &WorkspaceEdit) -> Result<(Vec<PathBuf>, usize)> {
    let mut files_modified = Vec::new();
    let mut total_edits = 0;

    // Handle simple changes (URI -> Vec<TextEdit>)
    if let Some(changes) = &workspace_edit.changes {
        for (uri, edits) in changes {
            let file_path = uri_to_path(uri)?;
            apply_text_edits(&file_path, edits)?;
            files_modified.push(file_path);
            total_edits += edits.len();
        }
    }

    // Handle document_changes (more complex, supports versioning and file operations)
    if let Some(document_changes) = &workspace_edit.document_changes {
        match document_changes {
            DocumentChanges::Edits(edits) => {
                for edit in edits {
                    let file_path = uri_to_path(&edit.text_document.uri)?;
                    // Convert OneOf<TextEdit, AnnotatedTextEdit> to TextEdit
                    let text_edits: Vec<TextEdit> = edit
                        .edits
                        .iter()
                        .map(|e| match e {
                            lsp_types::OneOf::Left(text_edit) => text_edit.clone(),
                            lsp_types::OneOf::Right(annotated) => annotated.text_edit.clone(),
                        })
                        .collect();
                    apply_text_edits(&file_path, &text_edits)?;
                    files_modified.push(file_path);
                    total_edits += text_edits.len();
                }
            }
            DocumentChanges::Operations(ops) => {
                // Handle complex operations like create/rename/delete files
                // For now, we'll just handle TextDocumentEdit operations
                for op in ops {
                    if let DocumentChangeOperation::Edit(edit) = op {
                        let file_path = uri_to_path(&edit.text_document.uri)?;
                        // Convert OneOf<TextEdit, AnnotatedTextEdit> to TextEdit
                        let text_edits: Vec<TextEdit> = edit
                            .edits
                            .iter()
                            .map(|e| match e {
                                lsp_types::OneOf::Left(text_edit) => text_edit.clone(),
                                lsp_types::OneOf::Right(annotated) => annotated.text_edit.clone(),
                            })
                            .collect();
                        apply_text_edits(&file_path, &text_edits)?;
                        files_modified.push(file_path);
                        total_edits += text_edits.len();
                    }
                    // TODO: Handle CreateFile, RenameFile, DeleteFile if needed
                }
            }
        }
    }

    Ok((files_modified, total_edits))
}

/// Apply text edits to a file
///
/// Edits are applied in reverse order (from end to start) to maintain
/// correct positions as we modify the file.
fn apply_text_edits(file_path: &Path, edits: &[TextEdit]) -> Result<()> {
    // Read current file content
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Split into lines for easier editing (owned strings)
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Sort edits by position (reverse order - bottom to top)
    // This ensures earlier edits don't invalidate later positions
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|a, b| {
        b.range.start.line.cmp(&a.range.start.line)
            .then(b.range.start.character.cmp(&a.range.start.character))
    });

    // Apply each edit
    for edit in sorted_edits {
        let start_line = edit.range.start.line as usize;
        let start_char = edit.range.start.character as usize;
        let end_line = edit.range.end.line as usize;
        let end_char = edit.range.end.character as usize;

        // Handle single-line edits
        if start_line == end_line {
            if start_line >= lines.len() {
                return Err(anyhow::anyhow!(
                    "Edit position out of bounds: line {} (file has {} lines)",
                    start_line,
                    lines.len()
                ));
            }

            let line = &lines[start_line];
            let new_line = format!(
                "{}{}{}",
                &line[..start_char.min(line.len())],
                edit.new_text,
                &line[end_char.min(line.len())..]
            );
            lines[start_line] = new_line;
        } else {
            // Multi-line edit: remove lines between start and end
            // and replace with new content
            let start_line_content = &lines[start_line];
            let end_line_content = &lines[end_line];

            let new_text = format!(
                "{}{}{}",
                &start_line_content[..start_char.min(start_line_content.len())],
                edit.new_text,
                &end_line_content[end_char.min(end_line_content.len())..]
            );

            // Replace the range with new content
            let new_lines: Vec<String> = new_text.lines().map(|s| s.to_string()).collect();

            // Remove old lines and insert new ones
            lines.splice(start_line..=end_line, new_lines);
        }
    }

    // Write back to file
    let new_content = lines.join("\n");
    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

    Ok(())
}

/// Convert LSP URI to filesystem path
fn uri_to_path(uri: &Uri) -> Result<PathBuf> {
    let uri_str = uri.as_str();
    if uri_str.starts_with("file://") {
        Ok(PathBuf::from(&uri_str[7..]))
    } else {
        Err(anyhow::anyhow!("Non-file URI not supported: {}", uri_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires LSP server
    fn test_lsp_query_lifecycle() {
        // This would test starting and querying an LSP server
        // Requires an actual LSP server binary in PATH
    }
}

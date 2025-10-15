use anyhow::{Context, Result};
use lsp_types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

/// Errors that can occur when communicating with an LSP server
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP server not running")]
    ServerNotRunning,

    #[error("LSP server initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Server returned error: code={code}, message={message}")]
    ServerError { code: i32, message: String },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// JSON-RPC notification (no response expected)
#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

/// LSP client that communicates with a language server via JSON-RPC over stdio
pub struct LspClient {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
    pub server_capabilities: Option<ServerCapabilities>,
    root_uri: Uri,
}

impl LspClient {
    /// Start an LSP server process and initialize it
    ///
    /// # Arguments
    /// * `command` - Command to execute (e.g., "sourcekit-lsp")
    /// * `args` - Arguments to pass to the command
    /// * `root_uri` - Project root URI (e.g., "file:///path/to/project")
    ///
    /// # Returns
    /// An initialized LSP client ready to serve requests
    pub fn start(command: &str, args: &[String], root_uri: &str) -> Result<Self> {
        // 1. Spawn LSP server process
        let mut process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Ignore stderr (or could pipe for debugging)
            .spawn()
            .with_context(|| format!("Failed to spawn LSP server: {}", command))?;

        // 2. Get stdin/stdout handles
        let stdin = BufWriter::new(
            process
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin"))?,
        );

        let stdout = BufReader::new(
            process
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?,
        );

        let root_url = Uri::from_str(root_uri)
            .map_err(|e| anyhow::anyhow!("Invalid root URI '{}': {}", root_uri, e))?;

        let mut client = Self {
            process,
            stdin,
            stdout,
            request_id: AtomicU64::new(1),
            server_capabilities: None,
            root_uri: root_url.clone(),
        };

        // 3. Initialize the server
        client.initialize(&root_url)?;

        Ok(client)
    }

    /// Initialize the LSP server with project information
    fn initialize(&mut self, root_uri: &Uri) -> Result<()> {
        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri.clone()),
            root_path: None, // Deprecated, use root_uri
            initialization_options: None,
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(ReferenceClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    rename: Some(RenameClientCapabilities {
                        dynamic_registration: Some(false),
                        prepare_support: Some(false),
                        prepare_support_default_behavior: None,
                        honors_change_annotations: Some(false),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: Some(ClientInfo {
                name: "powertools".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            locale: None,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        // Send initialize request
        let response = self
            .send_request("initialize", serde_json::to_value(init_params)?)
            .with_context(|| "Failed to initialize LSP server")?;

        // Parse server capabilities
        let init_result: InitializeResult = serde_json::from_value(response)
            .context("Failed to parse initialize response")?;

        self.server_capabilities = Some(init_result.capabilities);

        // Send initialized notification
        self.send_notification(
            "initialized",
            serde_json::to_value(InitializedParams {})?,
        )?;

        Ok(())
    }

    /// Send a request to the LSP server and wait for response
    ///
    /// # Arguments
    /// * `method` - LSP method name (e.g., "textDocument/definition")
    /// * `params` - JSON-serialized parameters
    ///
    /// # Returns
    /// The result field from the JSON-RPC response
    fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&request)?;

        // Write with LSP headers (Content-Length)
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
        self.stdin.flush()?;

        // Read response
        self.read_response(id)
    }

    /// Send a notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&notification)?;

        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
        self.stdin.flush()?;

        Ok(())
    }

    /// Read a JSON-RPC response from the server
    fn read_response(&mut self, expected_id: u64) -> Result<Value> {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let timeout = Duration::from_secs(30); // 30 second timeout
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

        loop {
            // Check timeout and max attempts
            if start.elapsed() > timeout {
                return Err(LspError::Timeout(30000).into());
            }
            if attempts >= MAX_ATTEMPTS {
                return Err(LspError::ProtocolViolation(
                    format!("Max attempts ({}) exceeded waiting for response ID {}", MAX_ATTEMPTS, expected_id)
                ).into());
            }
            attempts += 1;

            // Read headers
            let mut headers = Vec::new();
            let mut line = String::new();

            loop {
                line.clear();
                self.stdout.read_line(&mut line)?;

                if line == "\r\n" {
                    break; // End of headers
                }

                if !line.trim().is_empty() {
                    headers.push(line.trim().to_string());
                }
            }

            // Parse Content-Length header
            let content_length = headers
                .iter()
                .find(|h| h.starts_with("Content-Length:"))
                .and_then(|h| h.split(':').nth(1))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .ok_or_else(|| {
                    LspError::ProtocolViolation("Missing or invalid Content-Length".to_string())
                })?;

            // Read content
            let mut buffer = vec![0u8; content_length];
            self.stdout.read_exact(&mut buffer)?;

            let content = String::from_utf8(buffer)
                .map_err(|e| LspError::InvalidResponse(format!("Invalid UTF-8: {}", e)))?;

            // Try to parse as response
            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&content) {
                if response.id == expected_id {
                    if let Some(error) = response.error {
                        return Err(LspError::ServerError {
                            code: error.code,
                            message: error.message,
                        }
                        .into());
                    }

                    return response
                        .result
                        .ok_or_else(|| LspError::InvalidResponse("No result field".to_string()).into());
                }
            }

            // Might be a notification or other message, ignore and keep reading
            // (In production, we'd handle notifications properly)
        }
    }

    /// Get next request ID
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Notify server that a document was opened
    pub fn did_open(&mut self, uri: &Uri, language_id: &str, text: String) -> Result<()> {
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: language_id.to_string(),
                version: 1,
                text,
            },
        };

        self.send_notification("textDocument/didOpen", serde_json::to_value(params)?)?;

        Ok(())
    }

    /// Find where a symbol is defined
    ///
    /// # Arguments
    /// * `uri` - Document URI (e.g., "file:///path/to/file.swift")
    /// * `line` - Line number (0-indexed, LSP convention)
    /// * `character` - Character offset (0-indexed, LSP convention)
    ///
    /// # Returns
    /// List of locations where the symbol is defined (usually 1 element)
    pub fn goto_definition(
        &mut self,
        uri: &Uri,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line,
                    character,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request("textDocument/definition", serde_json::to_value(params)?)?;

        // Response can be Location, Location[], or LocationLink[]
        // We'll handle the simple cases for now
        if response.is_null() {
            return Ok(vec![]);
        }

        // Try to parse as single location
        if let Ok(location) = serde_json::from_value::<Location>(response.clone()) {
            return Ok(vec![location]);
        }

        // Try to parse as array of locations
        if let Ok(locations) = serde_json::from_value::<Vec<Location>>(response.clone()) {
            return Ok(locations);
        }

        // Could also be LocationLink[] but we'll skip that for now
        Ok(vec![])
    }

    /// Find all references to a symbol
    ///
    /// # Arguments
    /// * `uri` - Document URI
    /// * `line` - Line number (0-indexed)
    /// * `character` - Character offset (0-indexed)
    /// * `include_declaration` - Whether to include the declaration itself
    ///
    /// # Returns
    /// List of locations where the symbol is referenced
    pub fn find_references(
        &mut self,
        uri: &Uri,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line,
                    character,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let response = self.send_request("textDocument/references", serde_json::to_value(params)?)?;

        if response.is_null() {
            return Ok(vec![]);
        }

        // Should be an array of locations
        let locations = serde_json::from_value::<Vec<Location>>(response)
            .context("Failed to parse references response")?;

        Ok(locations)
    }

    /// Prepare to rename a symbol - validates that rename is possible
    ///
    /// This should be called before `rename()` to validate that the symbol
    /// at the given position can be renamed, and to get the exact range
    /// that will be renamed.
    ///
    /// # Arguments
    /// * `uri` - Document URI where the symbol is located
    /// * `line` - Line number (0-indexed)
    /// * `character` - Character offset (0-indexed)
    ///
    /// # Returns
    /// Option containing the range that can be renamed, or None if rename is not possible
    pub fn prepare_rename(
        &mut self,
        uri: &Uri,
        line: u32,
        character: u32,
    ) -> Result<Option<Range>> {
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line,
                character,
            },
        };

        let response = self.send_request("textDocument/prepareRename", serde_json::to_value(params)?)?;

        if response.is_null() {
            // Rename not possible at this location
            return Ok(None);
        }

        // Response can be Range or { range: Range, placeholder: string }
        // Try to parse as Range first
        if let Ok(range) = serde_json::from_value::<Range>(response.clone()) {
            return Ok(Some(range));
        }

        // Try to parse as PrepareRenameResponse
        #[derive(serde::Deserialize)]
        struct PrepareRenameResponse {
            range: Range,
            #[allow(dead_code)]
            placeholder: String,
        }

        if let Ok(prep) = serde_json::from_value::<PrepareRenameResponse>(response) {
            return Ok(Some(prep.range));
        }

        // Couldn't parse response
        Ok(None)
    }

    /// Rename a symbol across the workspace
    ///
    /// # Arguments
    /// * `uri` - Document URI where the symbol is located
    /// * `line` - Line number (0-indexed)
    /// * `character` - Character offset (0-indexed)
    /// * `new_name` - New name for the symbol
    ///
    /// # Returns
    /// WorkspaceEdit containing all file changes needed to complete the rename
    pub fn rename(
        &mut self,
        uri: &Uri,
        line: u32,
        character: u32,
        new_name: String,
    ) -> Result<WorkspaceEdit> {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line,
                    character,
                },
            },
            new_name,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request("textDocument/rename", serde_json::to_value(params)?)?;

        if response.is_null() {
            // No changes needed (shouldn't normally happen)
            return Ok(WorkspaceEdit {
                changes: None,
                document_changes: None,
                change_annotations: None,
            });
        }

        // Parse the WorkspaceEdit response
        let workspace_edit = serde_json::from_value::<WorkspaceEdit>(response)
            .context("Failed to parse rename response")?;

        Ok(workspace_edit)
    }

    /// Get available code actions at a position or range
    ///
    /// Code actions include refactorings like extract function, inline variable,
    /// quick fixes, and other code transformations.
    ///
    /// # Arguments
    /// * `uri` - Document URI
    /// * `range` - Range to get code actions for
    /// * `only_kinds` - Optional filter for specific kinds (e.g., ["refactor.extract"])
    ///
    /// # Returns
    /// List of available code actions
    pub fn code_actions(
        &mut self,
        uri: &Uri,
        range: Range,
        only_kinds: Option<Vec<CodeActionKind>>,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range,
            context: CodeActionContext {
                diagnostics: vec![], // Could include diagnostics if we tracked them
                only: only_kinds,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request("textDocument/codeAction", serde_json::to_value(params)?)?;

        if response.is_null() {
            return Ok(vec![]);
        }

        // Parse response as array of CodeActionOrCommand
        let actions = serde_json::from_value::<Vec<CodeActionOrCommand>>(response)
            .context("Failed to parse code actions response")?;

        Ok(actions)
    }

    /// Execute a code action's command
    ///
    /// Some code actions return a Command that needs to be executed.
    /// This method sends the workspace/executeCommand request.
    ///
    /// # Arguments
    /// * `lsp_command` - LSP Command to execute
    ///
    /// # Returns
    /// Result value from command execution
    pub fn execute_command(&mut self, lsp_command: &lsp_types::Command) -> Result<Value> {
        let params = ExecuteCommandParams {
            command: lsp_command.command.clone(),
            arguments: lsp_command.arguments.clone().unwrap_or_default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request("workspace/executeCommand", serde_json::to_value(params)?)?;
        Ok(response)
    }

    /// Gracefully shut down the LSP server
    pub fn shutdown(&mut self) -> Result<()> {
        // Send shutdown request
        let _ = self.send_request("shutdown", Value::Null);

        // Send exit notification
        let _ = self.send_notification("exit", Value::Null);

        // Wait for process to exit (with timeout)
        // In production, we'd use a timeout here
        let _ = self.process.wait();

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Try graceful shutdown with timeout
        let _ = self.send_request("shutdown", Value::Null);
        let _ = self.send_notification("exit", Value::Null);

        // Give process 1 second to exit cleanly
        use std::time::Duration;
        use std::thread;

        // Try to wait for up to 1 second
        for _ in 0..10 {
            match self.process.try_wait() {
                Ok(Some(_)) => return, // Process exited cleanly
                Ok(None) => thread::sleep(Duration::from_millis(100)),
                Err(_) => break,
            }
        }

        // Force kill if still running
        let _ = self.process.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require an actual LSP server to be available
    // They're more like integration tests than unit tests

    #[test]
    #[ignore] // Ignore by default (requires LSP server)
    fn test_lsp_client_lifecycle() {
        // This would test starting, initializing, and shutting down a server
        // Requires an LSP server binary in PATH
    }

    #[test]
    fn test_json_rpc_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test\""));
    }
}

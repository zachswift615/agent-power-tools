use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const MAX_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlEntry {
    pub timestamp: DateTime<Utc>,
    pub request: RequestLog,
    pub response: ResponseLog,
    pub token_usage: TokenUsageLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub model: String,
    pub messages: Vec<MessageLog>,
    pub system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageLog {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseLog {
    pub content: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageLog {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub struct JsonlLogger {
    #[allow(dead_code)]
    project_name: String,
    log_dir: PathBuf,
    current_log_file: Option<PathBuf>,
    max_file_size: u64,
}

impl JsonlLogger {
    /// Create a new JSONL logger for a project
    pub fn new(project_name: &str) -> std::io::Result<Self> {
        let log_dir = Self::get_projects_log_dir().join(project_name);
        fs::create_dir_all(&log_dir)?;

        Ok(Self {
            project_name: project_name.to_string(),
            log_dir,
            current_log_file: None,
            max_file_size: MAX_LOG_FILE_SIZE,
        })
    }

    /// Get the base log directory for all projects
    pub fn get_projects_log_dir() -> PathBuf {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home).join(".synthia").join("projects")
    }

    /// Get the project name
    #[allow(dead_code)]
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// List all JSONL log files for this project
    #[allow(dead_code)]
    pub fn list_log_files(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !self.log_dir.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(&self.log_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }

        files.sort();
        Ok(files)
    }

    /// Log a turn (request/response pair)
    pub fn log_turn(&mut self, entry: JsonlEntry) -> std::io::Result<()> {
        let log_file = self.get_or_create_log_file()?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(&entry)?;
        writeln!(writer, "{}", json)?;
        writer.flush()?;

        Ok(())
    }

    /// Get or create the current log file, handling rotation
    fn get_or_create_log_file(&mut self) -> std::io::Result<PathBuf> {
        // Check if we need to rotate (file too large or doesn't exist)
        let needs_rotation = if let Some(ref current_file) = self.current_log_file {
            if current_file.exists() {
                let metadata = fs::metadata(current_file)?;
                metadata.len() >= self.max_file_size
            } else {
                true
            }
        } else {
            true
        };

        if needs_rotation {
            // Use nanoseconds to ensure unique filenames even when rotating quickly
            let now = Utc::now();
            let timestamp = now.format("%Y%m%d_%H%M%S");
            let nanos = now.timestamp_subsec_nanos();
            let filename = format!("{}_{:09}.jsonl", timestamp, nanos);
            let new_file = self.log_dir.join(filename);
            self.current_log_file = Some(new_file.clone());
        }

        Ok(self.current_log_file.clone().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonl_entry_serialization() {
        let entry = JsonlEntry {
            timestamp: Utc::now(),
            request: RequestLog {
                model: "claude-3-5-sonnet-20241022".to_string(),
                messages: vec![
                    MessageLog {
                        role: "user".to_string(),
                        content: "Hello, Claude!".to_string(),
                    }
                ],
                system: Some("You are a helpful assistant.".to_string()),
            },
            response: ResponseLog {
                content: "Hello! How can I help you today?".to_string(),
                stop_reason: Some("end_turn".to_string()),
            },
            token_usage: TokenUsageLog {
                input_tokens: 20,
                output_tokens: 15,
            },
        };

        // Test serialization
        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        assert!(json.contains("claude-3-5-sonnet-20241022"));
        assert!(json.contains("Hello, Claude!"));
        assert!(json.contains("Hello! How can I help you today?"));

        // Test deserialization
        let deserialized: JsonlEntry = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(deserialized.request.messages[0].content, "Hello, Claude!");
        assert_eq!(deserialized.response.content, "Hello! How can I help you today?");
        assert_eq!(deserialized.token_usage.input_tokens, 20);
        assert_eq!(deserialized.token_usage.output_tokens, 15);
    }

    #[test]
    fn test_logger_creation() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

        let temp_dir = std::env::temp_dir().join(format!("synthia_test_logger_{}", std::process::id()));
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", temp_dir.to_str().unwrap());

        let logger = JsonlLogger::new("test_project").expect("Failed to create logger");
        assert_eq!(logger.project_name(), "test_project");

        let expected_dir = temp_dir.join(".synthia").join("projects").join("test_project");
        assert!(expected_dir.exists());

        // Cleanup
        fs::remove_dir_all(temp_dir.join(".synthia")).ok();
        if let Some(home) = old_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_log_turn() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

        let temp_dir = std::env::temp_dir().join(format!("synthia_test_log_turn_{}", std::process::id()));
        let old_home = std::env::var("HOME").ok();

        // Clean up any existing test directory
        fs::remove_dir_all(&temp_dir).ok();
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        std::env::set_var("HOME", temp_dir.to_str().unwrap());

        let mut logger = JsonlLogger::new("test_project").expect("Failed to create logger");

        let entry = JsonlEntry {
            timestamp: Utc::now(),
            request: RequestLog {
                model: "claude-3-5-sonnet-20241022".to_string(),
                messages: vec![
                    MessageLog {
                        role: "user".to_string(),
                        content: "Test message".to_string(),
                    }
                ],
                system: None,
            },
            response: ResponseLog {
                content: "Test response".to_string(),
                stop_reason: Some("end_turn".to_string()),
            },
            token_usage: TokenUsageLog {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        logger.log_turn(entry).expect("Failed to log turn");

        let files = logger.list_log_files().expect("Failed to list files");
        assert_eq!(files.len(), 1, "Expected 1 log file, found {}", files.len());
        assert!(files[0].extension().and_then(|s| s.to_str()) == Some("jsonl"));

        // Verify file content
        let content = fs::read_to_string(&files[0]).expect("Failed to read log file");
        assert!(content.contains("Test message"));
        assert!(content.contains("Test response"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
        if let Some(home) = old_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_file_rotation() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();

        let temp_dir = std::env::temp_dir().join(format!("synthia_test_rotation_{}", std::process::id()));
        let old_home = std::env::var("HOME").ok();

        // Clean up any existing test directory
        fs::remove_dir_all(&temp_dir).ok();
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        std::env::set_var("HOME", temp_dir.to_str().unwrap());

        let mut logger = JsonlLogger::new("test_project").expect("Failed to create logger");
        // Set a very small max file size to trigger rotation
        logger.max_file_size = 100;

        let create_entry = || JsonlEntry {
            timestamp: Utc::now(),
            request: RequestLog {
                model: "claude-3-5-sonnet-20241022".to_string(),
                messages: vec![
                    MessageLog {
                        role: "user".to_string(),
                        content: "This is a test message that will help us exceed the file size limit".to_string(),
                    }
                ],
                system: None,
            },
            response: ResponseLog {
                content: "This is a test response that will help us exceed the file size limit".to_string(),
                stop_reason: Some("end_turn".to_string()),
            },
            token_usage: TokenUsageLog {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        // Log multiple entries to trigger rotation
        // Each entry is ~300 bytes, so we need at least 2 entries to exceed 100 bytes
        for i in 0..10 {
            logger.log_turn(create_entry()).expect("Failed to log turn");
            // Small delay to ensure different timestamps for rotation
            if i % 2 == 0 {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }

        let files = logger.list_log_files().expect("Failed to list files");
        assert!(files.len() > 1, "Expected multiple files due to rotation, got {}", files.len());

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
        if let Some(home) = old_home {
            std::env::set_var("HOME", home);
        }
    }
}

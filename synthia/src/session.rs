use crate::types::Message;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,  // Optional friendly name
    pub created_at: i64,
    pub last_modified: i64,
    pub model: String,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new(model: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: generate_session_id(),
            name: None,
            created_at: now,
            last_modified: now,
            model,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.last_modified = chrono::Utc::now().timestamp_millis();
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
        self.last_modified = chrono::Utc::now().timestamp_millis();
    }

    pub fn save(&self) -> Result<()> {
        let path = get_session_path(&self.id)?;
        let json = serde_json::to_string_pretty(&self)
            .context("Failed to serialize session")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create session directory")?;
        }

        fs::write(&path, json)
            .with_context(|| format!("Failed to write session to {:?}", path))?;

        tracing::debug!("Session saved to {:?}", path);
        Ok(())
    }

    pub fn load(session_id: &str) -> Result<Self> {
        let path = get_session_path(session_id)?;
        let json = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session from {:?}", path))?;

        let session: Session = serde_json::from_str(&json)
            .context("Failed to deserialize session")?;

        tracing::debug!("Session loaded from {:?}", path);
        Ok(session)
    }

    #[allow(dead_code)]
    pub fn delete(session_id: &str) -> Result<()> {
        let path = get_session_path(session_id)?;
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete session at {:?}", path))?;

        tracing::debug!("Session deleted: {:?}", path);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: Option<String>,
    pub created_at: i64,
    pub last_modified: i64,
    pub model: String,
    pub message_count: usize,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            name: session.name.clone(),
            created_at: session.created_at,
            last_modified: session.last_modified,
            model: session.model.clone(),
            message_count: session.messages.len(),
        }
    }
}

pub fn list_sessions() -> Result<Vec<SessionInfo>> {
    let sessions_dir = get_sessions_dir()?;

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in fs::read_dir(&sessions_dir)
        .with_context(|| format!("Failed to read sessions directory: {:?}", sessions_dir))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match Session::load(
                &path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
            ) {
                Ok(session) => sessions.push(SessionInfo::from(&session)),
                Err(e) => {
                    tracing::warn!("Failed to load session from {:?}: {}", path, e);
                }
            }
        }
    }

    // Sort by last modified (most recent first)
    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(sessions)
}

#[allow(dead_code)]
pub fn get_most_recent_session() -> Result<Option<Session>> {
    let sessions = list_sessions()?;

    if let Some(info) = sessions.first() {
        Ok(Some(Session::load(&info.id)?))
    } else {
        Ok(None)
    }
}

fn get_sessions_dir() -> Result<PathBuf> {
    // Check for XDG_DATA_HOME override (for testing)
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg_data_home).join("synthia").join("sessions"));
    }

    let data_dir = dirs::data_local_dir()
        .context("Failed to get local data directory")?;
    Ok(data_dir.join("synthia").join("sessions"))
}

fn get_session_path(session_id: &str) -> Result<PathBuf> {
    let sessions_dir = get_sessions_dir()?;
    Ok(sessions_dir.join(format!("{}.json", session_id)))
}

fn generate_session_id() -> String {
    // Use timestamp + random suffix for uniqueness
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let random_suffix: String = (0..6)
        .map(|_| {
            let idx = (rand::random::<u8>() % 36) as usize;
            "0123456789abcdefghijklmnopqrstuvwxyz".chars().nth(idx).unwrap()
        })
        .collect();
    format!("{}_{}", timestamp, random_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentBlock, Role};
    use tempfile::TempDir;
    use std::env;
    use serial_test::serial;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("HOME", temp_dir.path());
        temp_dir
    }

    #[test]
    fn test_session_new() {
        let session = Session::new("test-model".to_string());
        assert!(!session.id.is_empty());
        assert_eq!(session.model, "test-model");
        assert_eq!(session.messages.len(), 0);
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.last_modified);
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new("test-model".to_string());
        let initial_modified = session.last_modified;

        // Add a small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        let message = Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        };

        session.add_message(message);

        assert_eq!(session.messages.len(), 1);
        assert!(session.last_modified >= initial_modified);
    }

    #[test]
    fn test_session_serialization() {
        let mut session = Session::new("test-model".to_string());
        session.add_message(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Test message".to_string(),
            }],
        });

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.model, session.model);
        assert_eq!(deserialized.messages.len(), 1);
    }

    #[test]
    fn test_session_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // Override the sessions directory for this test
        env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        let mut session = Session::new("test-model".to_string());
        session.add_message(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Test".to_string(),
            }],
        });

        // Save session
        session.save().unwrap();

        // Load session
        let loaded = Session::load(&session.id).unwrap();

        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.model, session.model);
        assert_eq!(loaded.messages.len(), 1);
    }

    #[test]
    fn test_session_delete() {
        let temp_dir = TempDir::new().unwrap();

        // Override the sessions directory for this test
        env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        let session = Session::new("test-model".to_string());
        session.save().unwrap();

        // Verify it exists
        assert!(Session::load(&session.id).is_ok());

        // Delete it
        Session::delete(&session.id).unwrap();

        // Verify it's gone
        assert!(Session::load(&session.id).is_err());
    }

    #[test]
    #[serial]
    fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();

        // Override the sessions directory for this test
        env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        // Create a few sessions
        let session1 = Session::new("model1".to_string());
        session1.save().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));

        let session2 = Session::new("model2".to_string());
        session2.save().unwrap();

        let sessions = list_sessions().unwrap();

        assert_eq!(sessions.len(), 2, "Expected 2 sessions, found {}", sessions.len());
        // Should be sorted by last modified (most recent first)
        assert_eq!(sessions[0].id, session2.id);
        assert_eq!(sessions[1].id, session1.id);
    }

    #[test]
    #[serial]
    fn test_get_most_recent_session() {
        let temp_dir = TempDir::new().unwrap();

        // Override the sessions directory for this test
        env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        // No sessions yet
        assert!(get_most_recent_session().unwrap().is_none());

        // Create a session
        let session = Session::new("test-model".to_string());
        session.save().unwrap();

        // Get most recent
        let recent = get_most_recent_session().unwrap().unwrap();
        assert_eq!(recent.id, session.id);
    }

    #[test]
    fn test_session_info_from_session() {
        let mut session = Session::new("test-model".to_string());
        session.add_message(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Test".to_string(),
            }],
        });

        let info = SessionInfo::from(&session);

        assert_eq!(info.id, session.id);
        assert_eq!(info.model, session.model);
        assert_eq!(info.message_count, 1);
        assert_eq!(info.created_at, session.created_at);
        assert_eq!(info.last_modified, session.last_modified);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        // IDs should be unique (highly probable with random suffix)
        assert_ne!(id1, id2);
    }
}

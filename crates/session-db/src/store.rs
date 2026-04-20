use crate::error::{Result, SessionError};
use crate::models::{Message, MessageRole, Session};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn =
            Connection::open(path).map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    source TEXT NOT NULL DEFAULT 'cli',
                    model TEXT NOT NULL,
                    system_prompt TEXT NOT NULL DEFAULT '',
                    parent_session_id TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS messages (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tool_calls TEXT,
                    tool_name TEXT,
                    reasoning TEXT,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (session_id) REFERENCES sessions(id)
                );
                PRAGMA user_version = 1;",
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn create_session(&self, model: &str, system_prompt: &str) -> Result<Session> {
        let session = Session {
            id: Uuid::now_v7(),
            source: "cli".to_string(),
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
            parent_session_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.conn
            .execute(
                "INSERT INTO sessions (id, source, model, system_prompt, parent_session_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    session.id.to_string(),
                    &session.source,
                    &session.model,
                    &session.system_prompt,
                    session.parent_session_id.map(|u| u.to_string()),
                    session.created_at.to_rfc3339(),
                    session.updated_at.to_rfc3339(),
                ),
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        Ok(session)
    }

    pub fn get_session(&self, id: &Uuid) -> Result<Option<Session>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, source, model, system_prompt, parent_session_id, created_at, updated_at FROM sessions WHERE id = ?1")
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        let result = stmt
            .query_row([id.to_string()], |row| {
                Ok(Session {
                    id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                    source: row.get(1)?,
                    model: row.get(2)?,
                    system_prompt: row.get(3)?,
                    parent_session_id: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| s.parse().ok()),
                    created_at: row.get::<_, String>(5)?.parse().unwrap_or_default(),
                    updated_at: row.get::<_, String>(6)?.parse().unwrap_or_default(),
                })
            })
            .optional()
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<Session>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, source, model, system_prompt, parent_session_id, created_at, updated_at FROM sessions ORDER BY created_at DESC LIMIT ?1")
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        let sessions = stmt
            .query_map([limit as i64], |row| {
                Ok(Session {
                    id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                    source: row.get(1)?,
                    model: row.get(2)?,
                    system_prompt: row.get(3)?,
                    parent_session_id: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| s.parse().ok()),
                    created_at: row.get::<_, String>(5)?.parse().unwrap_or_default(),
                    updated_at: row.get::<_, String>(6)?.parse().unwrap_or_default(),
                })
            })
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?
            .filter_map(|s| s.ok())
            .collect();

        Ok(sessions)
    }

    pub fn append_message(
        &self,
        session_id: &Uuid,
        role: MessageRole,
        content: &str,
    ) -> Result<Message> {
        let msg = Message {
            id: Uuid::now_v7(),
            session_id: *session_id,
            role,
            content: content.to_string(),
            tool_calls: None,
            tool_name: None,
            reasoning: None,
            created_at: Utc::now(),
        };

        self.conn
            .execute(
                "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_name, reasoning, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    msg.id.to_string(),
                    msg.session_id.to_string(),
                    serde_json::to_string(&msg.role).unwrap_or_default(),
                    &msg.content,
                    msg.tool_calls.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                    msg.tool_name.as_deref(),
                    msg.reasoning.as_deref(),
                    msg.created_at.to_rfc3339(),
                ),
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        // Update session timestamp
        self.conn
            .execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                (Utc::now().to_rfc3339(), session_id.to_string()),
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        Ok(msg)
    }

    pub fn get_messages(&self, session_id: &Uuid) -> Result<Vec<Message>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, role, content, tool_calls, tool_name, reasoning, created_at
                 FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        let messages = stmt
            .query_map([session_id.to_string()], |row| {
                let role_str: String = row.get(2)?;
                let role = serde_json::from_str(&format!("\"{}\"", role_str.trim_matches('"')))
                    .unwrap_or(MessageRole::User);
                let tool_calls_str: Option<String> = row.get(4)?;
                Ok(Message {
                    id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                    session_id: row.get::<_, String>(1)?.parse().unwrap_or_default(),
                    role,
                    content: row.get(3)?,
                    tool_calls: tool_calls_str.and_then(|s| serde_json::from_str(&s).ok()),
                    tool_name: row.get(5)?,
                    reasoning: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap_or_default(),
                })
            })
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?
            .filter_map(|m| m.ok())
            .collect();

        Ok(messages)
    }

    pub fn delete_session(&self, id: &Uuid) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM messages WHERE session_id = ?1",
                [id.to_string()],
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", [id.to_string()])
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_round_trip() {
        let store = SessionStore::new_in_memory().unwrap();
        let session = store.create_session("test-model", "test prompt").unwrap();
        let loaded = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(loaded.model, "test-model");
        assert_eq!(loaded.system_prompt, "test prompt");
    }

    #[test]
    fn test_message_append_and_retrieve() {
        let store = SessionStore::new_in_memory().unwrap();
        let session = store.create_session("test-model", "").unwrap();
        store
            .append_message(&session.id, MessageRole::User, "hello")
            .unwrap();
        store
            .append_message(&session.id, MessageRole::Assistant, "world")
            .unwrap();
        store
            .append_message(&session.id, MessageRole::User, "how are you")
            .unwrap();
        let messages = store.get_messages(&session.id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].content, "world");
        assert_eq!(messages[2].content, "how are you");
    }

    #[test]
    fn test_wal_mode() {
        let store = SessionStore::new_in_memory().unwrap();
        let mode: String = store
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "memory");
    }

    #[test]
    fn test_delete_session() {
        let store = SessionStore::new_in_memory().unwrap();
        let session = store.create_session("test-model", "").unwrap();
        store
            .append_message(&session.id, MessageRole::User, "hello")
            .unwrap();
        store.delete_session(&session.id).unwrap();
        assert!(store.get_session(&session.id).unwrap().is_none());
        assert!(store.get_messages(&session.id).unwrap().is_empty());
    }

    #[test]
    fn test_list_sessions() {
        let store = SessionStore::new_in_memory().unwrap();
        store.create_session("model-1", "").unwrap();
        store.create_session("model-2", "").unwrap();
        let sessions = store.list_sessions(10).unwrap();
        assert_eq!(sessions.len(), 2);
    }
}

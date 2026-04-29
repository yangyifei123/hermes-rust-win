//! Token usage tracking store for context caching metrics.

use crate::error::{Result, SessionError};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub struct TokenStore {
    conn: Connection,
}

impl TokenStore {
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
                "CREATE TABLE IF NOT EXISTS context_tokens (
                    session_id TEXT PRIMARY KEY,
                    tokens_used INTEGER NOT NULL,
                    tokens_cached INTEGER NOT NULL,
                    updated_at TEXT NOT NULL
                );",
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn get(&self, session_id: &str) -> Result<Option<(u32, u32)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tokens_used, tokens_cached FROM context_tokens WHERE session_id = ?1")
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        let result = stmt
            .query_row([session_id], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
            .optional()
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub fn set(&self, session_id: &str, used: u32, cached: u32) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO context_tokens (session_id, tokens_used, tokens_cached, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(session_id) DO UPDATE SET tokens_used = ?2, tokens_cached = ?3, updated_at = datetime('now')",
                (session_id, used, cached),
            )
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_store_get_missing() {
        let store = TokenStore::new_in_memory().unwrap();
        assert!(store.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_token_store_set_and_get() {
        let store = TokenStore::new_in_memory().unwrap();
        store.set("sess-1", 100, 50).unwrap();
        let (used, cached) = store.get("sess-1").unwrap().unwrap();
        assert_eq!(used, 100);
        assert_eq!(cached, 50);
    }

    #[test]
    fn test_token_store_upsert() {
        let store = TokenStore::new_in_memory().unwrap();
        store.set("sess-1", 100, 50).unwrap();
        store.set("sess-1", 200, 75).unwrap();
        let (used, cached) = store.get("sess-1").unwrap().unwrap();
        assert_eq!(used, 200);
        assert_eq!(cached, 75);
    }
}

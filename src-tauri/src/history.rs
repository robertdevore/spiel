//! Local history persistence for Spiel Phase 8.
//!
//! Provides CRUD operations on the `history_entries` SQLite table.
//! All operations are synchronous (SQLite is fast and local).
//!
//! Architecture:
//! - `HistoryEntry`: serializable model for a saved session
//! - `HistoryState`: app-level state tracking (enabled flag, loading status)
//! - CRUD functions operate on `&Database`
//!
//! Privacy:
//! - No clipboard contents stored
//! - No API keys stored
//! - No audio file contents stored
//! - History is local only — no sync, no network

use crate::database::Database;
use serde::{Deserialize, Serialize};

/// A saved history entry representing a completed Spiel session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub raw_text: String,
    pub final_text: String,
    pub mode: String,
    pub cleanup_provider: String,
    pub transcription_engine: String,
    pub transcription_engine_kind: String,
    pub audio_file_path: Option<String>,
    pub audio_duration_ms: Option<i64>,
    pub transcript_duration_ms: Option<i64>,
    pub cleanup_duration_ms: Option<i64>,
    pub is_mock_transcript: bool,
    pub is_mock_cleanup: bool,
    pub error: Option<String>,
}

/// Request to save a new history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveHistoryRequest {
    pub raw_text: String,
    pub final_text: String,
    pub mode: String,
    pub cleanup_provider: String,
    pub transcription_engine: String,
    pub transcription_engine_kind: String,
    pub audio_file_path: Option<String>,
    pub audio_duration_ms: Option<i64>,
    pub transcript_duration_ms: Option<i64>,
    pub cleanup_duration_ms: Option<i64>,
    pub is_mock_transcript: bool,
    pub is_mock_cleanup: bool,
    pub error: Option<String>,
}

/// Application-level history state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStateData {
    /// Whether history saving is enabled
    pub enabled: bool,
    /// Total number of entries in the database
    pub entry_count: i64,
    /// Error message if a history operation failed
    pub error: Option<String>,
}

impl Default for HistoryStateData {
    fn default() -> Self {
        Self {
            enabled: true,
            entry_count: 0,
            error: None,
        }
    }
}

// ── CRUD Operations ─────────────────────────────────────────

/// Save a new history entry. Returns the created entry with its assigned id.
pub fn save_entry(db: &Database, request: &SaveHistoryRequest) -> Result<HistoryEntry, String> {
    if request.raw_text.trim().is_empty() {
        return Err("Cannot save an empty transcript to history.".into());
    }

    let now = crate::chrono_now_iso();
    let title = derive_title(&request.raw_text);

    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    conn.execute(
        "INSERT INTO history_entries (
            created_at, updated_at, title, raw_text, final_text, mode,
            cleanup_provider, transcription_engine, transcription_engine_kind,
            audio_file_path, audio_duration_ms, transcript_duration_ms,
            cleanup_duration_ms, is_mock_transcript, is_mock_cleanup, error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        rusqlite::params![
            now,
            now,
            title,
            request.raw_text,
            request.final_text,
            request.mode,
            request.cleanup_provider,
            request.transcription_engine,
            request.transcription_engine_kind,
            request.audio_file_path,
            request.audio_duration_ms,
            request.transcript_duration_ms,
            request.cleanup_duration_ms,
            request.is_mock_transcript as i32,
            request.is_mock_cleanup as i32,
            request.error,
        ],
    )
    .map_err(|e| format!("Failed to save history entry: {}", e))?;

    let id = conn.last_insert_rowid();

    drop(conn);

    get_entry(db, id)
}

/// Retrieve a single history entry by id.
pub fn get_entry(db: &Database, id: i64) -> Result<HistoryEntry, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    conn.query_row(
        "SELECT id, created_at, updated_at, title, raw_text, final_text, mode,
                cleanup_provider, transcription_engine, transcription_engine_kind,
                audio_file_path, audio_duration_ms, transcript_duration_ms,
                cleanup_duration_ms, is_mock_transcript, is_mock_cleanup, error
         FROM history_entries WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                title: row.get(3)?,
                raw_text: row.get(4)?,
                final_text: row.get(5)?,
                mode: row.get(6)?,
                cleanup_provider: row.get(7)?,
                transcription_engine: row.get(8)?,
                transcription_engine_kind: row.get(9)?,
                audio_file_path: row.get(10)?,
                audio_duration_ms: row.get(11)?,
                transcript_duration_ms: row.get(12)?,
                cleanup_duration_ms: row.get(13)?,
                is_mock_transcript: row.get::<_, i32>(14)? != 0,
                is_mock_cleanup: row.get::<_, i32>(15)? != 0,
                error: row.get(16)?,
            })
        },
    )
    .map_err(|e| format!("History entry not found (id={}): {}", id, e))
}

/// List recent history entries, newest first. Limited to `limit` entries.
pub fn list_entries(db: &Database, limit: i64) -> Result<Vec<HistoryEntry>, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, created_at, updated_at, title, raw_text, final_text, mode,
                    cleanup_provider, transcription_engine, transcription_engine_kind,
                    audio_file_path, audio_duration_ms, transcript_duration_ms,
                    cleanup_duration_ms, is_mock_transcript, is_mock_cleanup, error
             FROM history_entries ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare list query: {}", e))?;

    let entries = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                title: row.get(3)?,
                raw_text: row.get(4)?,
                final_text: row.get(5)?,
                mode: row.get(6)?,
                cleanup_provider: row.get(7)?,
                transcription_engine: row.get(8)?,
                transcription_engine_kind: row.get(9)?,
                audio_file_path: row.get(10)?,
                audio_duration_ms: row.get(11)?,
                transcript_duration_ms: row.get(12)?,
                cleanup_duration_ms: row.get(13)?,
                is_mock_transcript: row.get::<_, i32>(14)? != 0,
                is_mock_cleanup: row.get::<_, i32>(15)? != 0,
                error: row.get(16)?,
            })
        })
        .map_err(|e| format!("Failed to list history entries: {}", e))?;

    let mut result = Vec::new();
    for entry in entries {
        result.push(entry.map_err(|e| format!("Failed to read history entry: {}", e))?);
    }

    Ok(result)
}

/// Delete a single history entry by id.
pub fn delete_entry(db: &Database, id: i64) -> Result<(), String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    let affected = conn
        .execute(
            "DELETE FROM history_entries WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Failed to delete history entry: {}", e))?;

    if affected == 0 {
        return Err(format!("History entry not found (id={}).", id));
    }

    Ok(())
}

/// Delete all history entries.
pub fn clear_entries(db: &Database) -> Result<usize, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    let count = conn
        .execute("DELETE FROM history_entries", [])
        .map_err(|e| format!("Failed to clear history: {}", e))?;

    Ok(count)
}

/// Get the total number of history entries.
pub fn count_entries(db: &Database) -> Result<i64, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;

    conn.query_row("SELECT COUNT(*) FROM history_entries", [], |row| row.get(0))
        .map_err(|e| format!("Failed to count history entries: {}", e))
}

// ── Helpers ─────────────────────────────────────────────────

/// Derive a title from the first non-empty line of raw text.
fn derive_title(text: &str) -> String {
    let first_line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Untitled");

    let clean = first_line.trim();
    // Limit title length
    if clean.len() > 80 {
        format!("{}...", &clean[..77])
    } else {
        clean.to_string()
    }
}

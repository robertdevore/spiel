//! Database initialization and migration for Spiel Phase 8.
//!
//! Manages the local SQLite database for history persistence.
//! Database file: `{app_local_data_dir}/spiel_history.db`
//!
//! Architecture:
//! - `Database` struct wraps a `Mutex<Connection>` for thread-safe access
//! - `initialize()` runs on app setup, creates tables if missing (idempotent)
//! - Schema version tracked in `schema_version` table
//! - Uses bundled SQLite (no system dependency)

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// The current schema version. Increment when adding migrations.
const SCHEMA_VERSION: i64 = 4;

/// Wraps a SQLite connection with thread-safe access.
pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the database at the given path and run migrations.
    pub fn open(db_path: &PathBuf) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create database directory: {}", e))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open database at {:?}: {}", db_path, e))?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.run_migrations()?;

        Ok(db)
    }

    /// Run all migrations. Safe to call multiple times.
    fn run_migrations(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock error: {}", e))?;

        // Create schema_version table if not exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );",
        )
        .map_err(|e| format!("Migration error (schema_version): {}", e))?;

        // Check current version
        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS history_entries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    title TEXT NOT NULL DEFAULT '',
                    raw_text TEXT NOT NULL,
                    final_text TEXT NOT NULL DEFAULT '',
                    mode TEXT NOT NULL DEFAULT '',
                    cleanup_provider TEXT NOT NULL DEFAULT '',
                    transcription_engine TEXT NOT NULL DEFAULT '',
                    transcription_engine_kind TEXT NOT NULL DEFAULT '',
                    audio_file_path TEXT,
                    audio_duration_ms INTEGER,
                    transcript_duration_ms INTEGER,
                    cleanup_duration_ms INTEGER,
                    is_mock_transcript INTEGER NOT NULL DEFAULT 0,
                    is_mock_cleanup INTEGER NOT NULL DEFAULT 0,
                    error TEXT
                );",
            )
            .map_err(|e| format!("Migration error (v1 history_entries): {}", e))?;

            // Insert version 1
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])
                .map_err(|e| format!("Migration error (v1 version insert): {}", e))?;
        }

        if current_version < 2 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS app_settings (
                    id INTEGER PRIMARY KEY,
                    hotkey TEXT NOT NULL DEFAULT 'Cmd+Shift+S',
                    history_enabled INTEGER NOT NULL DEFAULT 1,
                    clipboard_restore_enabled INTEGER NOT NULL DEFAULT 1,
                    default_transcription_engine TEXT NOT NULL DEFAULT 'mock',
                    default_text_mode TEXT NOT NULL DEFAULT 'raw_dictation',
                    default_cleanup_provider TEXT NOT NULL DEFAULT 'basic',
                    local_whisper_binary_path TEXT NOT NULL DEFAULT '',
                    local_whisper_model_path TEXT NOT NULL DEFAULT '',
                    local_only_mode INTEGER NOT NULL DEFAULT 1,
                    debug_logging_enabled INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL DEFAULT ''
                );",
            )
            .map_err(|e| format!("Migration error (v2 app_settings): {}", e))?;

            conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])
                .map_err(|e| format!("Migration error (v2 version insert): {}", e))?;
        }

        if current_version < 3 {
            conn.execute_batch(
                "ALTER TABLE app_settings ADD COLUMN auto_transcribe_after_recording INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE app_settings ADD COLUMN auto_cleanup_after_transcription INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE app_settings ADD COLUMN auto_save_history_after_cleanup INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE app_settings ADD COLUMN auto_insert_after_cleanup INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|e| format!("Migration error (v3 workflow settings): {}", e))?;

            conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])
                .map_err(|e| format!("Migration error (v3 version insert): {}", e))?;
        }

        if current_version < 4 {
            conn.execute_batch(
                "ALTER TABLE app_settings ADD COLUMN cloud_providers_enabled INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE app_settings ADD COLUMN openai_transcription_model TEXT NOT NULL DEFAULT 'whisper-1';
                 ALTER TABLE app_settings ADD COLUMN openai_cleanup_model TEXT NOT NULL DEFAULT 'gpt-4o-mini';",
            )
            .map_err(|e| format!("Migration error (v4 cloud provider settings): {}", e))?;

            conn.execute("INSERT INTO schema_version (version) VALUES (4)", [])
                .map_err(|e| format!("Migration error (v4 version insert): {}", e))?;
        }

        Ok(())
    }

    /// Get the current schema version for diagnostics.
    pub fn schema_version(&self) -> Result<i64, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock error: {}", e))?;
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to read schema version: {}", e))
    }
}

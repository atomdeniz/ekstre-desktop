//! SQLite storage: schema, dedup index, and reminder-eligibility /
//! backfill-suppression rules.

use crate::parser::Statement;
use rusqlite::{params, Connection};
use std::sync::Mutex;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS card_statements (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bank TEXT NOT NULL,
  card_last4 TEXT,
  card_masked TEXT,
  total_due NUMERIC NOT NULL,
  min_due NUMERIC,
  due_date TEXT NOT NULL,
  statement_date TEXT,
  reminded_at TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_card_stmt
  ON card_statements (bank, ifnull(card_last4, ''), due_date);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

/// A row read back from the DB (superset of `Statement` fields plus `id`/`reminded_at`).
#[derive(Debug, Clone)]
pub struct StatementRow {
    pub id: i64,
    pub bank: String,
    pub card_last4: Option<String>,
    pub card_masked: Option<String>,
    pub total_due: f64,
    pub min_due: Option<f64>,
    pub due_date: String,
    pub statement_date: Option<String>,
    pub reminded_at: Option<String>,
}

/// A single-connection SQLite store, serialized behind a mutex.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a statement, returning `true` if a new row was added. Duplicates
    /// (same bank, card_last4-or-empty, due_date) are no-ops via the unique index.
    /// Statements already past their due date at capture time are pre-marked
    /// `reminded_at` so backfilling history never floods the reminder job.
    pub fn insert_statement(&self, s: &Statement) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "INSERT INTO card_statements
               (bank, card_last4, card_masked, total_due, min_due, due_date, statement_date, reminded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                     CASE WHEN ?6 < date('now','localtime') THEN datetime('now') ELSE NULL END)
             ON CONFLICT DO NOTHING",
            params![
                s.bank,
                s.card_last4,
                s.card_masked,
                s.total_due,
                s.min_due,
                s.due_date,
                s.statement_date,
            ],
        )?;
        Ok(n > 0)
    }

    /// The row id of a statement identified by its natural key (bank, card_last4,
    /// due_date) — the same tuple the unique index dedups on. Lets the shell locate
    /// the row after an insert (new or deduped) to name its stored PDF file.
    pub fn statement_id(&self, s: &Statement) -> rusqlite::Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id FROM card_statements
             WHERE bank = ?1 AND ifnull(card_last4, '') = ifnull(?2, '') AND due_date = ?3",
            params![s.bank, s.card_last4, s.due_date],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
    }

    /// A single statement row by id, for building a download filename.
    pub fn get_statement(&self, id: i64) -> rusqlite::Result<Option<StatementRow>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, bank, card_last4, card_masked, total_due, min_due, due_date, statement_date, reminded_at
             FROM card_statements WHERE id = ?1",
            params![id],
            row_to_statement,
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
    }

    /// Statements whose due date is within `days_before` days from today (past-due
    /// included) and not yet reminded, oldest first. `days_before` = 0 means "due
    /// today or earlier"; larger values fire the reminder that many days ahead.
    pub fn due_unreminded(&self, days_before: i64) -> rusqlite::Result<Vec<StatementRow>> {
        let conn = self.conn.lock().unwrap();
        let modifier = format!("+{} days", days_before.max(0));
        let mut stmt = conn.prepare(
            "SELECT id, bank, card_last4, card_masked, total_due, min_due, due_date, statement_date, reminded_at
             FROM card_statements
             WHERE due_date <= date('now','localtime',?1) AND reminded_at IS NULL
             ORDER BY due_date",
        )?;
        let rows = stmt
            .query_map([modifier], row_to_statement)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn mark_reminded(&self, id: i64, when_iso: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE card_statements SET reminded_at = ?1 WHERE id = ?2",
            params![when_iso, id],
        )?;
        Ok(())
    }

    /// The newest statement per (bank, card_last4) group, ordered by due_date asc.
    /// This is the dashboard's data source.
    pub fn latest_per_card(&self) -> rusqlite::Result<Vec<StatementRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, bank, card_last4, card_masked, total_due, min_due, due_date, statement_date, reminded_at
             FROM (
               SELECT *, ROW_NUMBER() OVER (
                 PARTITION BY bank, ifnull(card_last4, '')
                 ORDER BY due_date DESC
               ) AS rn
               FROM card_statements
             )
             WHERE rn = 1
             ORDER BY due_date",
        )?;
        let rows = stmt
            .query_map([], row_to_statement)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

impl Database {
    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Today's local date as ISO `YYYY-MM-DD`, from SQLite's clock (so it matches
    /// the comparisons used in `due_unreminded`/`insert_statement`).
    pub fn today_local(&self) -> rusqlite::Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT date('now','localtime')", [], |r| r.get(0))
    }

    /// Current local hour (0-23), from SQLite's clock.
    pub fn local_hour(&self) -> rusqlite::Result<u32> {
        let conn = self.conn.lock().unwrap();
        let h: String =
            conn.query_row("SELECT strftime('%H','now','localtime')", [], |r| r.get(0))?;
        Ok(h.parse().unwrap_or(0))
    }

    /// Current UTC timestamp as `datetime('now')` (for `reminded_at`).
    pub fn now_iso(&self) -> rusqlite::Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT datetime('now')", [], |r| r.get(0))
    }

    pub fn all_settings(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

fn row_to_statement(row: &rusqlite::Row) -> rusqlite::Result<StatementRow> {
    Ok(StatementRow {
        id: row.get(0)?,
        bank: row.get(1)?,
        card_last4: row.get(2)?,
        card_masked: row.get(3)?,
        total_due: row.get(4)?,
        min_due: row.get(5)?,
        due_date: row.get(6)?,
        statement_date: row.get(7)?,
        reminded_at: row.get(8)?,
    })
}

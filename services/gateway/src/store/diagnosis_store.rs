//! SQLite-backed diagnosis persistence. Schema-compatible with the legacy
//! Python `DiagnosisStore` (RFC-009).

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::error::Result;
use crate::store::now;

pub const DB_NAME: &str = "diagnoses.db";

#[derive(Debug, Clone)]
pub struct DiagnosisRecord {
    pub diagnosis_id: String,
    pub source_ids: Vec<String>,
    pub trigger_type: String,
    pub severity: String,
    pub summary: String,
    pub possible_causes: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence: f64,
    pub disclaimer: String,
    pub raw_prompt: String,
    pub error_code: String,
    pub error_message: String,
    pub metrics: Vec<Value>,
    pub created_at: f64,
}

impl DiagnosisRecord {
    pub fn new(diagnosis_id: impl Into<String>, trigger_type: impl Into<String>) -> Self {
        Self {
            diagnosis_id: diagnosis_id.into(),
            source_ids: Vec::new(),
            trigger_type: trigger_type.into(),
            severity: "normal".into(),
            summary: String::new(),
            possible_causes: Vec::new(),
            recommendations: Vec::new(),
            confidence: 0.0,
            disclaimer: String::new(),
            raw_prompt: String::new(),
            error_code: String::new(),
            error_message: String::new(),
            metrics: Vec::new(),
            created_at: now(),
        }
    }

    pub fn to_wire_dict(&self) -> Value {
        json!({
            "diagnosis_id": self.diagnosis_id,
            "source_ids": self.source_ids,
            "trigger_type": self.trigger_type,
            "severity": self.severity,
            "summary": self.summary,
            "possible_causes": self.possible_causes,
            "recommendations": self.recommendations,
            "confidence": self.confidence,
            "disclaimer": self.disclaimer,
            "raw_prompt": self.raw_prompt,
            "error_code": self.error_code,
            "error_message": self.error_message,
            "metrics": self.metrics,
            "created_at": self.created_at,
        })
    }
}

pub struct DiagnosisStore {
    conn: Mutex<Connection>,
}

impl DiagnosisStore {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        Self::from_connection(Connection::open(dir.join(DB_NAME))?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS diagnoses (
                diagnosis_id TEXT PRIMARY KEY,
                source_ids TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'normal',
                summary TEXT DEFAULT '',
                possible_causes TEXT DEFAULT '[]',
                recommendations TEXT DEFAULT '[]',
                confidence REAL DEFAULT 0.0,
                disclaimer TEXT DEFAULT '',
                raw_prompt TEXT DEFAULT '',
                error_code TEXT DEFAULT '',
                error_message TEXT DEFAULT '',
                metrics_json TEXT DEFAULT '[]',
                created_at REAL NOT NULL
            )",
            [],
        )?;
        let mut stmt = conn.prepare("PRAGMA table_info(diagnoses)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<_>>()?;
        if !cols.iter().any(|c| c == "metrics_json") {
            conn.execute(
                "ALTER TABLE diagnoses ADD COLUMN metrics_json TEXT DEFAULT '[]'",
                [],
            )?;
        }
        drop(stmt);
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn add(&self, rec: &DiagnosisRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO diagnoses
             (diagnosis_id, source_ids, trigger_type, severity, summary, possible_causes,
              recommendations, confidence, disclaimer, raw_prompt, error_code,
              error_message, metrics_json, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![
                rec.diagnosis_id,
                serde_json::to_string(&rec.source_ids)?,
                rec.trigger_type,
                rec.severity,
                rec.summary,
                serde_json::to_string(&rec.possible_causes)?,
                serde_json::to_string(&rec.recommendations)?,
                rec.confidence,
                rec.disclaimer,
                rec.raw_prompt,
                rec.error_code,
                rec.error_message,
                serde_json::to_string(&rec.metrics)?,
                rec.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, diagnosis_id: &str) -> Result<Option<DiagnosisRecord>> {
        let conn = self.conn.lock().unwrap();
        let rec = conn
            .query_row(
                "SELECT * FROM diagnoses WHERE diagnosis_id = ?1",
                [diagnosis_id],
                row_to_record,
            )
            .optional()?;
        Ok(rec)
    }

    pub fn list_all(&self, offset: i64, limit: i64) -> Result<Vec<DiagnosisRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT * FROM diagnoses ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt
            .query_map(rusqlite::params![limit, offset], row_to_record)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let n = conn.query_row("SELECT COUNT(*) FROM diagnoses", [], |r| r.get(0))?;
        Ok(n)
    }

    /// Delete diagnoses older than `max_age_s` seconds. Returns deleted count.
    /// `max_age_s <= 0` keeps everything forever.
    pub fn purge_older_than(&self, max_age_s: f64) -> Result<usize> {
        if max_age_s <= 0.0 {
            return Ok(0);
        }
        let cutoff = now() - max_age_s;
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM diagnoses WHERE created_at < ?1", [cutoff])?;
        Ok(deleted)
    }
}

fn json_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<DiagnosisRecord> {
    let metrics_json: String = row.get("metrics_json").unwrap_or_else(|_| "[]".into());
    Ok(DiagnosisRecord {
        diagnosis_id: row.get("diagnosis_id")?,
        source_ids: json_vec(&row.get::<_, String>("source_ids")?),
        trigger_type: row.get("trigger_type")?,
        severity: row.get("severity")?,
        summary: row.get("summary")?,
        possible_causes: json_vec(&row.get::<_, String>("possible_causes")?),
        recommendations: json_vec(&row.get::<_, String>("recommendations")?),
        confidence: row.get("confidence")?,
        disclaimer: row.get("disclaimer")?,
        raw_prompt: row.get("raw_prompt")?,
        error_code: row.get("error_code")?,
        error_message: row.get("error_message")?,
        metrics: serde_json::from_str(&metrics_json).unwrap_or_default(),
        created_at: row.get("created_at")?,
    })
}

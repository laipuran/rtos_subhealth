//! SQLite-backed task persistence. The schema is byte-compatible with the
//! legacy Python `TaskStore` so an existing `tasks.db` can be opened directly.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};

use crate::error::Result;
use crate::model::task::{Goal, TaskRecord, TaskUpdate};
use crate::store::now;

pub const DB_NAME: &str = "tasks.db";

pub struct TaskStore {
    conn: Mutex<Connection>,
}

impl TaskStore {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        Self::from_connection(Connection::open(dir.join(DB_NAME))?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                goal_id TEXT PRIMARY KEY,
                goal_json TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'accepted',
                progress REAL DEFAULT 0.0,
                current_tag INTEGER DEFAULT -1,
                next_tag INTEGER DEFAULT -1,
                error_code TEXT DEFAULT '',
                message TEXT DEFAULT '',
                result_json TEXT DEFAULT '',
                route_json TEXT DEFAULT '[]',
                finished_stages INTEGER DEFAULT 0,
                created_at REAL NOT NULL,
                updated_at REAL NOT NULL
            )",
            [],
        )?;
        Self::ensure_column(&conn, "route_json", "TEXT DEFAULT '[]'")?;
        Self::ensure_column(&conn, "finished_stages", "INTEGER DEFAULT 0")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Add a column if an older database predates it.
    fn ensure_column(conn: &Connection, name: &str, decl: &str) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(tasks)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<_, _>>()?;
        if !cols.iter().any(|c| c == name) {
            conn.execute(&format!("ALTER TABLE tasks ADD COLUMN {name} {decl}"), [])?;
        }
        Ok(())
    }

    pub fn add(&self, record: &TaskRecord) -> Result<()> {
        let goal_json = serde_json::to_string(&record.goal)?;
        let result_json = match &record.final_state {
            Some(fs) => serde_json::json!({
                "final_state": fs, "error_code": "", "message": ""
            })
            .to_string(),
            None => String::new(),
        };
        let route_json = serde_json::to_string(&record.route)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tasks
             (goal_id, goal_json, state, progress, current_tag, next_tag, error_code,
              message, result_json, route_json, finished_stages, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            rusqlite::params![
                record.goal_id,
                goal_json,
                record.state,
                record.progress,
                record.current_tag,
                record.next_tag,
                record.error_code,
                record.message,
                result_json,
                route_json,
                record.finished_stages,
                record.created_at,
                record.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, goal_id: &str) -> Result<Option<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let rec = conn
            .query_row(
                "SELECT * FROM tasks WHERE goal_id = ?1",
                [goal_id],
                row_to_record,
            )
            .optional()?;
        Ok(rec)
    }

    pub fn update(&self, goal_id: &str, patch: &TaskUpdate) -> Result<bool> {
        if patch.is_empty() {
            return Ok(false);
        }
        let mut fields: Vec<String> = Vec::new();
        let mut values: Vec<SqlValue> = Vec::new();

        macro_rules! set {
            ($opt:expr, $col:expr) => {
                if let Some(v) = $opt.clone() {
                    fields.push(format!("{} = ?", $col));
                    values.push(v.into());
                }
            };
        }
        set!(patch.state, "state");
        set!(patch.progress, "progress");
        set!(patch.current_tag, "current_tag");
        set!(patch.next_tag, "next_tag");
        set!(patch.error_code, "error_code");
        set!(patch.message, "message");
        set!(patch.finished_stages, "finished_stages");

        if let Some(route) = &patch.route {
            fields.push("route_json = ?".into());
            values.push(SqlValue::Text(serde_json::to_string(route)?));
        }
        if let Some(fs) = &patch.final_state {
            fields.push("result_json = ?".into());
            values.push(SqlValue::Text(
                serde_json::json!({ "final_state": fs, "error_code": "", "message": "" })
                    .to_string(),
            ));
        }
        fields.push("updated_at = ?".into());
        values.push(SqlValue::Real(now()));
        values.push(SqlValue::Text(goal_id.to_string()));

        let sql = format!("UPDATE tasks SET {} WHERE goal_id = ?", fields.join(", "));
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(&sql, params_from_iter(values))?;
        Ok(changed > 0)
    }

    pub fn list_all(&self) -> Result<Vec<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tasks ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], row_to_record)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn list_active(&self) -> Result<Vec<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM tasks WHERE state IN ('accepted','running') ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], row_to_record)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn exists(&self, goal_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT 1 FROM tasks WHERE goal_id = ?1", [goal_id], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(found.is_some())
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    let goal_json: String = row.get("goal_json")?;
    let goal: Goal = serde_json::from_str(&goal_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let result_json: String = row.get("result_json").unwrap_or_default();
    let final_state = if result_json.is_empty() {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&result_json)
            .ok()
            .and_then(|v| {
                v.get("final_state")
                    .and_then(|s| s.as_str())
                    .map(str::to_string)
            })
            .filter(|s| !s.is_empty())
    };

    let route_json: String = row.get("route_json").unwrap_or_else(|_| "[]".into());
    let route: Vec<i32> = serde_json::from_str(&route_json).unwrap_or_default();

    Ok(TaskRecord {
        goal_id: row.get("goal_id")?,
        goal,
        state: row.get("state")?,
        progress: row.get("progress")?,
        current_tag: row.get("current_tag")?,
        next_tag: row.get("next_tag")?,
        error_code: row.get("error_code")?,
        message: row.get("message")?,
        final_state,
        route,
        finished_stages: row.get("finished_stages").unwrap_or(0),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

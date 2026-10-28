use std::path::Path;

use chrono::{DateTime, NaiveDate, NaiveTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::error::Result;
use crate::model::{Priority, Task};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    notes TEXT,
    due_date TEXT,
    due_time TEXT,
    priority TEXT NOT NULL,
    tags TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

const COLS: &str = "id, title, notes, due_date, due_time, priority, tags, \
                    completed_at, created_at, updated_at, updated_by, deleted";

/// 저장소 추상화. SQLite 외의 구현으로 교체할 수 있다.
pub trait TaskStore {
    fn upsert(&self, task: &Task) -> Result<()>;
    /// 삭제 표시된 할 일도 반환한다. (동기화 병합에 필요)
    fn get(&self, id: Uuid) -> Result<Option<Task>>;
    /// 삭제 표시되지 않은 할 일만 반환한다.
    fn all(&self) -> Result<Vec<Task>>;
    fn all_including_deleted(&self) -> Result<Vec<Task>>;
    fn meta_get(&self, key: &str) -> Result<Option<String>>;
    fn meta_set(&self, key: &str, value: &str) -> Result<()>;
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(SCHEMA)?;
        Ok(SqliteStore { conn })
    }

    fn query_tasks(&self, sql: &str) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], row_to_task)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

impl TaskStore for SqliteStore {
    fn upsert(&self, t: &Task) -> Result<()> {
        self.conn.execute(
            &format!(
                "INSERT OR REPLACE INTO tasks ({COLS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
            ),
            params![
                t.id.to_string(),
                t.title,
                t.notes,
                t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                t.due_time.map(|tm| tm.format("%H:%M:%S").to_string()),
                t.priority.as_str(),
                serde_json::to_string(&t.tags)?,
                t.completed_at.as_ref().map(fmt_dt),
                fmt_dt(&t.created_at),
                fmt_dt(&t.updated_at),
                t.updated_by,
                t.deleted,
            ],
        )?;
        Ok(())
    }

    fn get(&self, id: Uuid) -> Result<Option<Task>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {COLS} FROM tasks WHERE id = ?1"),
                [id.to_string()],
                row_to_task,
            )
            .optional()?)
    }

    fn all(&self) -> Result<Vec<Task>> {
        self.query_tasks(&format!("SELECT {COLS} FROM tasks WHERE deleted = 0"))
    }

    fn all_including_deleted(&self) -> Result<Vec<Task>> {
        self.query_tasks(&format!("SELECT {COLS} FROM tasks"))
    }

    fn meta_get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            [key, value],
        )?;
        Ok(())
    }
}

pub(crate) fn fmt_dt(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn conv<T, E>(idx: usize, r: std::result::Result<T, E>) -> rusqlite::Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
{
    r.map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn parse_dt(idx: usize, s: &str) -> rusqlite::Result<DateTime<Utc>> {
    conv(
        idx,
        DateTime::parse_from_rfc3339(s).map(|d| d.with_timezone(&Utc)),
    )
}

fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
    let id: String = row.get(0)?;
    let due_date: Option<String> = row.get(3)?;
    let due_time: Option<String> = row.get(4)?;
    let priority: String = row.get(5)?;
    let tags: String = row.get(6)?;
    let completed_at: Option<String> = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;

    Ok(Task {
        id: conv(0, Uuid::parse_str(&id))?,
        title: row.get(1)?,
        notes: row.get(2)?,
        due_date: due_date
            .map(|s| conv(3, NaiveDate::parse_from_str(&s, "%Y-%m-%d")))
            .transpose()?,
        due_time: due_time
            .map(|s| conv(4, NaiveTime::parse_from_str(&s, "%H:%M:%S")))
            .transpose()?,
        priority: conv(5, priority.parse::<Priority>())?,
        tags: conv(6, serde_json::from_str(&tags))?,
        completed_at: completed_at.map(|s| parse_dt(7, &s)).transpose()?,
        created_at: parse_dt(8, &created_at)?,
        updated_at: parse_dt(9, &updated_at)?,
        updated_by: row.get(10)?,
        deleted: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 6, 0, 0).unwrap()
    }

    fn full_task() -> Task {
        let mut t = Task::new("팀플 회의", t0(), "pc");
        t.notes = Some("자료 준비\n두 번째 줄".into());
        t.due_date = NaiveDate::from_ymd_opt(2026, 10, 29);
        t.due_time = NaiveTime::from_hms_opt(15, 0, 0);
        t.priority = Priority::High;
        t.tags = vec!["학교".into(), "팀플".into()];
        t.completed_at = Some(t0() + chrono::Duration::hours(1));
        t
    }

    #[test]
    fn upsert_then_get_roundtrips_every_field() {
        let store = SqliteStore::open_in_memory().unwrap();
        let t = full_task();
        store.upsert(&t).unwrap();
        assert_eq!(store.get(t.id).unwrap(), Some(t));
    }

    #[test]
    fn upsert_replaces_existing_row() {
        let store = SqliteStore::open_in_memory().unwrap();
        let mut t = full_task();
        store.upsert(&t).unwrap();
        t.title = "바뀐 제목".into();
        store.upsert(&t).unwrap();
        assert_eq!(store.all_including_deleted().unwrap().len(), 1);
        assert_eq!(store.get(t.id).unwrap().unwrap().title, "바뀐 제목");
    }

    #[test]
    fn get_unknown_id_returns_none() {
        let store = SqliteStore::open_in_memory().unwrap();
        assert_eq!(store.get(Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn all_excludes_deleted_but_get_and_all_including_deleted_keep_them() {
        let store = SqliteStore::open_in_memory().unwrap();
        let alive = Task::new("alive", t0(), "pc");
        let mut dead = Task::new("dead", t0(), "pc");
        dead.deleted = true;
        store.upsert(&alive).unwrap();
        store.upsert(&dead).unwrap();

        assert_eq!(store.all().unwrap(), vec![alive]);
        assert_eq!(store.all_including_deleted().unwrap().len(), 2);
        assert!(store.get(dead.id).unwrap().unwrap().deleted);
    }

    #[test]
    fn meta_set_get_and_overwrite() {
        let store = SqliteStore::open_in_memory().unwrap();
        assert_eq!(store.meta_get("k").unwrap(), None);
        store.meta_set("k", "v1").unwrap();
        store.meta_set("k", "v2").unwrap();
        assert_eq!(store.meta_get("k").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn data_survives_reopening_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.db");
        let t = full_task();
        SqliteStore::open(&path).unwrap().upsert(&t).unwrap();
        let reopened = SqliteStore::open(&path).unwrap();
        assert_eq!(reopened.get(t.id).unwrap(), Some(t));
    }
}

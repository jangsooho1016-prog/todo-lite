use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use todo_core::clock::Now;
use todo_core::model::{Priority, Task};
use todo_core::parser::{ParsedTask, RuleBasedParser};
use todo_core::query::{Filter, TagCount, View};
use todo_core::service::{TaskEdit, TodoService};
use todo_core::store::{SqliteStore, TaskStore};
use todo_core::sync::{ensure_device_id, SyncEngine};
use todo_core::Error;
use uuid::Uuid;

const KEY_SYNC_FOLDER: &str = "sync_folder";

type Service = TodoService<SqliteStore, RuleBasedParser>;

#[derive(Debug)]
pub enum ApiError {
    Core(Error),
}

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError::Core(e)
    }
}

/// 화면에 그대로 보여 줄 한국어 메시지가 `Display`이자 직렬화 결과다.
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    /// 사용자에게 그대로 보여 줄 한국어 메시지.
    fn message(&self) -> String {
        match self {
            ApiError::Core(Error::EmptyTitle) => "제목을 입력해 주세요".to_string(),
            ApiError::Core(Error::NotFound(_)) => "할 일을 찾을 수 없어요".to_string(),
            ApiError::Core(Error::Io(e)) => format!("폴더에 접근할 수 없어요: {e}"),
            ApiError::Core(e) => format!("저장 중 문제가 생겼어요: {e}"),
        }
    }
}

impl Serialize for ApiError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.message())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewName {
    Today,
    Upcoming,
    Overdue,
    All,
    Completed,
}

impl From<ViewName> for View {
    fn from(v: ViewName) -> Self {
        match v {
            ViewName::Today => View::Today,
            ViewName::Upcoming => View::Upcoming,
            ViewName::Overdue => View::Overdue,
            ViewName::All => View::All,
            ViewName::Completed => View::Completed,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub view: ViewName,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ParsedDto {
    pub title: String,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

impl From<ParsedTask> for ParsedDto {
    fn from(p: ParsedTask) -> Self {
        ParsedDto {
            title: p.title,
            due_date: p.due_date,
            due_time: p.due_time,
            priority: p.priority,
            tags: p.tags,
        }
    }
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SyncStatus {
    pub configured: bool,
    pub exported: usize,
    pub applied: usize,
    pub corrupt_lines: usize,
}

/// Tauri와 무관한 앱 로직. `rusqlite::Connection`은 `Sync`가 아니라서 `Mutex`로 감싼다.
pub struct Api {
    service: Mutex<Service>,
    device_id: String,
    clock: fn() -> Now,
}

impl Api {
    pub fn open(path: &Path) -> Result<Self, ApiError> {
        Self::from_store(SqliteStore::open(path)?, Now::system)
    }

    pub fn from_store(store: SqliteStore, clock: fn() -> Now) -> Result<Self, ApiError> {
        let device_id = ensure_device_id(&store)?;
        let service = TodoService::new(store, RuleBasedParser::new(), device_id.clone());
        Ok(Api {
            service: Mutex::new(service),
            device_id,
            clock,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Service> {
        self.service.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn preview(&self, input: &str) -> ParsedDto {
        self.lock().preview(input, (self.clock)()).into()
    }

    pub fn add_task(&self, input: &str) -> Result<Task, ApiError> {
        Ok(self.lock().add_from_input(input, (self.clock)())?)
    }

    pub fn list_tasks(&self, q: &ListQuery) -> Result<Vec<Task>, ApiError> {
        let filter = Filter {
            view: q.view.into(),
            tag: q.tag.as_deref(),
            search: q.search.as_deref(),
        };
        Ok(self.lock().list(&filter, (self.clock)())?)
    }

    pub fn list_tags(&self) -> Result<Vec<TagCount>, ApiError> {
        Ok(self.lock().tags()?)
    }

    pub fn edit_task(&self, id: Uuid, edit: TaskEdit) -> Result<Task, ApiError> {
        Ok(self.lock().edit(id, edit, (self.clock)())?)
    }

    pub fn set_completed(&self, id: Uuid, done: bool) -> Result<Task, ApiError> {
        Ok(self.lock().set_completed(id, done, (self.clock)())?)
    }

    pub fn delete_task(&self, id: Uuid) -> Result<(), ApiError> {
        Ok(self.lock().delete(id, (self.clock)())?)
    }

    pub fn sync_folder(&self) -> Result<Option<String>, ApiError> {
        let value = self.lock().store().meta_get(KEY_SYNC_FOLDER)?;
        Ok(value.filter(|f| !f.is_empty()))
    }

    /// `None`이나 빈 문자열이면 해제한다. (`TaskStore`에 삭제 API가 없어 빈 값으로 표시)
    pub fn set_sync_folder(&self, folder: Option<String>) -> Result<(), ApiError> {
        let value = folder.map(|f| f.trim().to_string()).unwrap_or_default();
        self.lock().store().meta_set(KEY_SYNC_FOLDER, &value)?;
        Ok(())
    }

    /// 다른 기기의 변경을 먼저 가져오고, 이 기기의 변경을 내보낸다.
    pub fn sync_now(&self) -> Result<SyncStatus, ApiError> {
        let service = self.lock();
        let Some(folder) = service
            .store()
            .meta_get(KEY_SYNC_FOLDER)?
            .filter(|f| !f.is_empty())
        else {
            return Ok(SyncStatus::default());
        };
        let engine = SyncEngine::new(self.device_id.clone(), folder);
        let report = engine.import(service.store())?;
        let exported = engine.export(service.store())?;
        Ok(SyncStatus {
            configured: true,
            exported,
            applied: report.applied,
            corrupt_lines: report.corrupt_lines,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use todo_core::Error;

    fn fixed() -> Now {
        Now::fixed(
            NaiveDate::from_ymd_opt(2026, 10, 28)
                .unwrap()
                .and_hms_opt(15, 0, 0)
                .unwrap(),
        )
    }

    fn api() -> Api {
        Api::from_store(SqliteStore::open_in_memory().unwrap(), fixed).unwrap()
    }

    fn query(view: &str) -> ListQuery {
        serde_json::from_str(&format!(r#"{{"view":"{view}"}}"#)).unwrap()
    }

    #[test]
    fn add_then_list_today_and_tags() {
        let api = api();
        let task = api.add_task("오늘 오후 6시 퇴근 #개인 !중요").unwrap();
        assert_eq!(task.title, "퇴근");

        let today = api.list_tasks(&query("today")).unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(api.list_tags().unwrap()[0].name, "개인");
    }

    #[test]
    fn preview_does_not_persist() {
        let api = api();
        let parsed = api.preview("내일 오후 3시 회의 #학교");
        assert_eq!(parsed.title, "회의");
        assert_eq!(parsed.tags, ["학교"]);
        assert!(api.list_tasks(&query("all")).unwrap().is_empty());
    }

    #[test]
    fn list_query_accepts_optional_fields() {
        let q: ListQuery =
            serde_json::from_str(r#"{"view":"all","tag":"학교","search":null}"#).unwrap();
        assert_eq!(q.tag.as_deref(), Some("학교"));
        assert_eq!(q.search, None);
        assert!(serde_json::from_str::<ListQuery>(r#"{"view":"nope"}"#).is_err());
    }

    #[test]
    fn completing_moves_task_out_of_today() {
        let api = api();
        let task = api.add_task("오늘 운동").unwrap();
        api.set_completed(task.id, true).unwrap();
        assert!(api.list_tasks(&query("today")).unwrap().is_empty());
        assert_eq!(api.list_tasks(&query("completed")).unwrap().len(), 1);
    }

    #[test]
    fn edit_and_delete_roundtrip() {
        let api = api();
        let task = api.add_task("장보기").unwrap();
        let edited = api
            .edit_task(
                task.id,
                TaskEdit {
                    title: "마트 가기".into(),
                    notes: Some("우유".into()),
                    due_date: None,
                    due_time: None,
                    priority: Priority::Low,
                    tags: vec!["집".into()],
                },
            )
            .unwrap();
        assert_eq!(edited.title, "마트 가기");

        api.delete_task(task.id).unwrap();
        assert!(api.list_tasks(&query("all")).unwrap().is_empty());
    }

    #[test]
    fn errors_serialize_as_korean_messages() {
        let empty = serde_json::to_string(&ApiError::Core(Error::EmptyTitle)).unwrap();
        assert_eq!(empty, "\"제목을 입력해 주세요\"");

        let api = api();
        let err = api.add_task("내일").unwrap_err();
        assert!(matches!(err, ApiError::Core(Error::EmptyTitle)));

        let missing = ApiError::Core(Error::NotFound(Uuid::nil())).to_string();
        assert_eq!(missing, "할 일을 찾을 수 없어요");
    }

    #[test]
    fn sync_folder_set_unset_and_unconfigured_sync() {
        let api = api();
        assert_eq!(api.sync_folder().unwrap(), None);
        assert_eq!(api.sync_now().unwrap(), SyncStatus::default());

        api.set_sync_folder(Some("  C:/sync  ".into())).unwrap();
        assert_eq!(api.sync_folder().unwrap().as_deref(), Some("C:/sync"));

        api.set_sync_folder(None).unwrap();
        assert_eq!(api.sync_folder().unwrap(), None);
    }

    #[test]
    fn two_apis_sync_through_a_shared_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().to_string();
        let (a, b) = (api(), api());
        a.set_sync_folder(Some(folder.clone())).unwrap();
        b.set_sync_folder(Some(folder)).unwrap();

        a.add_task("pc에서 만든 일").unwrap();
        let sent = a.sync_now().unwrap();
        assert_eq!((sent.configured, sent.exported), (true, 1));

        let got = b.sync_now().unwrap();
        assert_eq!(got.applied, 1);
        assert_eq!(b.list_tasks(&query("all")).unwrap().len(), 1);
    }
}

use tauri::State;
use todo_core::model::Task;
use todo_core::query::TagCount;
use todo_core::service::TaskEdit;
use uuid::Uuid;

use crate::api::{Api, ApiError, ListQuery, ParsedDto, SyncStatus};
use crate::Startup;

#[tauri::command(async)]
pub fn preview_input(api: State<'_, Api>, input: String) -> ParsedDto {
    api.preview(&input)
}

#[tauri::command(async)]
pub fn add_task(api: State<'_, Api>, input: String) -> Result<Task, ApiError> {
    api.add_task(&input)
}

#[tauri::command(async)]
pub fn list_tasks(api: State<'_, Api>, query: ListQuery) -> Result<Vec<Task>, ApiError> {
    api.list_tasks(&query)
}

#[tauri::command(async)]
pub fn list_tags(api: State<'_, Api>) -> Result<Vec<TagCount>, ApiError> {
    api.list_tags()
}

#[tauri::command(async)]
pub fn edit_task(api: State<'_, Api>, id: Uuid, edit: TaskEdit) -> Result<Task, ApiError> {
    api.edit_task(id, edit)
}

#[tauri::command(async)]
pub fn set_completed(api: State<'_, Api>, id: Uuid, done: bool) -> Result<Task, ApiError> {
    api.set_completed(id, done)
}

#[tauri::command(async)]
pub fn delete_task(api: State<'_, Api>, id: Uuid) -> Result<(), ApiError> {
    api.delete_task(id)
}

#[tauri::command(async)]
pub fn get_sync_folder(api: State<'_, Api>) -> Result<Option<String>, ApiError> {
    api.sync_folder()
}

#[tauri::command(async)]
pub fn set_sync_folder(api: State<'_, Api>, folder: Option<String>) -> Result<(), ApiError> {
    api.set_sync_folder(folder)
}

#[tauri::command(async)]
pub fn sync_now(api: State<'_, Api>) -> Result<SyncStatus, ApiError> {
    api.sync_now()
}

/// 첫 화면이 그려진 뒤 프론트엔드가 부른다. 시작 시간 측정용.
#[tauri::command]
pub fn report_ready(startup: State<'_, Startup>, marks: Option<serde_json::Value>) {
    if std::env::var_os("TODO_LITE_TIMING").is_some() {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let marks = marks.map_or_else(|| "null".to_string(), |m| m.to_string());
        eprintln!(
            "js_marks={marks} ready_epoch_ms={epoch} ready_ms={}",
            startup.0.elapsed().as_millis()
        );
    }
}

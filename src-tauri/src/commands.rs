use tauri::{AppHandle, State};
use todo_core::model::Task;
use todo_core::query::TagCount;
use todo_core::service::TaskEdit;
use uuid::Uuid;

use crate::api::{Api, ApiError, ListQuery, ParsedDto, SyncStatus};
use crate::shortcut::{self, ShortcutState, ShortcutStatus};
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

#[tauri::command(async)]
pub fn get_quick_shortcut(state: State<'_, ShortcutState>) -> ShortcutStatus {
    state.lock().clone()
}

/// 새 조합을 등록하는 데 성공했을 때만 저장한다. 실패하면 이전 조합이 그대로 유지된다.
#[tauri::command(async)]
pub fn set_quick_shortcut(
    app: AppHandle,
    api: State<'_, Api>,
    state: State<'_, ShortcutState>,
    shortcut: String,
) -> Result<ShortcutStatus, String> {
    let new = shortcut::normalize(&shortcut).map_err(|e| e.to_string())?;
    // 시작할 때 등록에 실패했다면 현재 조합은 등록되어 있지 않다. 빈 문자열을 넘겨
    // 같은 조합을 다시 시도할 때도 실제로 등록하게 하고, 해제는 건너뛴다.
    let old = {
        let status = state.lock();
        if status.error.is_some() {
            String::new()
        } else {
            status.current.clone()
        }
    };
    shortcut::replace(&app, &old, &new).map_err(|e| e.to_string())?;
    api.set_quick_shortcut(&new).map_err(|e| e.to_string())?;
    let mut status = state.lock();
    status.current = new;
    status.error = None;
    Ok(status.clone())
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

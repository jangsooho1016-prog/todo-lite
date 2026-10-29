mod api;
mod commands;

use std::path::PathBuf;
use std::time::Instant;

use tauri::Manager;

pub struct Startup(pub Instant);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = Instant::now();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let dir = match std::env::var_os("TODO_LITE_DATA_DIR") {
                Some(custom) => PathBuf::from(custom),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&dir)?;
            app.manage(api::Api::open(&dir.join("todo.db"))?);
            app.manage(Startup(started));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::preview_input,
            commands::add_task,
            commands::list_tasks,
            commands::list_tags,
            commands::edit_task,
            commands::set_completed,
            commands::delete_task,
            commands::get_sync_folder,
            commands::set_sync_folder,
            commands::sync_now,
            commands::report_ready,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

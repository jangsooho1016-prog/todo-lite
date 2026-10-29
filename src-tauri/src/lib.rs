mod api;
mod commands;

use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::Manager;

pub struct Startup(pub Instant);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = Instant::now();
    let timing = std::env::var_os("TODO_LITE_TIMING").is_some();
    if timing {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        eprintln!("t_run_start_epoch_ms={epoch}");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_page_load(move |webview, payload| {
            if timing {
                let name = match payload.event() {
                    tauri::webview::PageLoadEvent::Started => "t_page_started_ms",
                    tauri::webview::PageLoadEvent::Finished => "t_page_finished_ms",
                };
                eprintln!(
                    "{name}={} url={} label={}",
                    started.elapsed().as_millis(),
                    payload.url(),
                    webview.label()
                );
            }
        })
        .setup(move |app| {
            let dir = match std::env::var_os("TODO_LITE_DATA_DIR") {
                Some(custom) => PathBuf::from(custom),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&dir)?;
            if timing {
                eprintln!("t_data_dir_ms={}", started.elapsed().as_millis());
            }
            app.manage(api::Api::open(&dir.join("todo.db"))?);
            if timing {
                eprintln!("t_db_open_ms={}", started.elapsed().as_millis());
            }
            app.manage(Startup(started));
            if timing {
                eprintln!("t_setup_done_ms={}", started.elapsed().as_millis());
            }
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

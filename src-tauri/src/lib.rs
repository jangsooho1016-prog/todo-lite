mod api;
mod commands;
mod shortcut;

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
        // Task 4: 이 플러그인에 `with_handler`로 빠른 입력창 토글을 연결한다.
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            let api = api::Api::open(&dir.join("todo.db"))?;
            if timing {
                eprintln!("t_db_open_ms={}", started.elapsed().as_millis());
            }
            // 단축키 등록 실패(다른 프로그램과 충돌 등)는 치명적이지 않다. 이유만 기록해 둔다.
            let stored = api
                .quick_shortcut()
                .unwrap_or_else(|_| shortcut::DEFAULT_SHORTCUT.to_string());
            let error = shortcut::register(app.handle(), &stored)
                .err()
                .map(|e| e.to_string());
            app.manage(shortcut::ShortcutState::new(stored, error));
            app.manage(api);
            app.manage(Startup(started));
            if timing {
                eprintln!("t_setup_done_ms={}", started.elapsed().as_millis());
            }
            // 임시: 메인 창이 숨김 상태로 만들어지므로 이후 작업에서 `--minimized` 처리로 대체할 때까지 바로 보여 준다.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
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
            commands::get_quick_shortcut,
            commands::set_quick_shortcut,
            commands::report_ready,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

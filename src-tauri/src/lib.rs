mod api;
mod commands;
mod quick;
mod shortcut;
mod tray;

use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState as KeyState;

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
    let mut builder = tauri::Builder::default();
    // 단일 인스턴스 플러그인은 가장 먼저 등록해야 한다. 데이터 폴더를 따로 지정한 격리 실행
    // (측정·시연 스크립트)은 사용자의 실행 중인 앱과 나란히 돌 수 있도록 등록하지 않는다.
    if std::env::var_os("TODO_LITE_DATA_DIR").is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        // 우리가 등록하는 단축키는 하나뿐이므로 어떤 단축키든 빠른 입력창을 토글한다.
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == KeyState::Pressed {
                        quick::toggle(app);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            (quick::LABEL, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            (quick::LABEL, WindowEvent::Focused(false)) => {
                quick::on_blur(window.app_handle());
            }
            _ => {}
        })
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
            app.manage(quick::QuickState::new());
            if timing {
                eprintln!("t_setup_done_ms={}", started.elapsed().as_millis());
            }
            tray::create(app.handle())?;
            // 검증용: 키 입력을 흉내 내지 않고 빠른 입력창이 뜨는지 확인한다.
            if std::env::var_os("TODO_LITE_AUTOTEST_QUICK").is_some() {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    if timing {
                        eprintln!("t_quick_trigger_ms={}", started.elapsed().as_millis());
                    }
                    quick::show(&handle);
                });
            }
            // 메인 창은 숨김 상태로 만들어진다. 자동 실행(`--minimized`)이 아니면 보여 준다.
            if !std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                }
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
            commands::show_quick,
            commands::hide_quick,
            commands::get_autostart,
            commands::set_autostart,
            commands::report_ready,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

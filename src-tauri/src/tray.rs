//! 시스템 트레이 아이콘과 메인 창 보이기/숨기기.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 보이고 최소화되지 않은 상태면 숨기고, 아니면 앞으로 가져온다.
pub fn toggle_main(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let showing = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
    if showing {
        let _ = w.hide();
    } else {
        show_main(app);
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "빠른 추가", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quick, &separator, &quit])?;

    // `build`가 아이콘을 앱의 리소스 테이블에 등록하므로 반환값을 버려도 아이콘이 유지된다.
    let mut builder = TrayIconBuilder::new()
        .tooltip("Todo-Lite")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

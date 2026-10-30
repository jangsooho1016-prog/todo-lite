//! 전역 단축키로 띄우는 빠른 입력창. 처음 필요할 때 만들고, 이후에는 숨겨 두고 재사용한다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::Startup;

pub const LABEL: &str = "quick";
/// 창이 뜬 직후에는 포커스 이동이 잦으므로 이 시간 동안은 포커스를 잃어도 닫지 않는다.
const BLUR_GRACE: Duration = Duration::from_millis(400);
/// 포커스를 잃으면 창을 닫을지 여부. 직접 써 보고 불안정하면 끈다.
pub const HIDE_ON_BLUR: bool = true;

/// 빠른 입력창의 상태.
pub struct QuickState {
    /// 마지막으로 빠른 입력창을 보여 준 시각.
    shown_at: Mutex<Option<Instant>>,
    /// 창을 만드는 중인지. 만들기가 끝나기 전에 또 열기 요청이 오면 창이 둘 생기므로 막는다.
    creating: AtomicBool,
    /// 창 이벤트로 추적하는 포커스 여부. `is_focused()`는 WebView2가 포커스를 가질 때 부정확할 수 있다.
    focused: AtomicBool,
}

impl QuickState {
    pub fn new() -> Self {
        QuickState {
            shown_at: Mutex::new(None),
            creating: AtomicBool::new(false),
            focused: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Instant>> {
        self.shown_at.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 창 이벤트(`Focused`)에서 호출한다.
    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::SeqCst);
    }

    fn is_focused(&self) -> bool {
        self.focused.load(Ordering::SeqCst)
    }
}

/// 빠른 입력창을 보여 준다. 창 만들기는 메인 스레드(이벤트 루프 콜백·동기 컨텍스트)에서
/// 하면 Windows에서 교착될 수 있어 비동기 작업에서 처리한다.
pub fn show(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let window = match app.get_webview_window(LABEL) {
            Some(w) => w,
            None => {
                // 창은 build()가 끝나야 등록된다. 그 사이에 온 요청은 먼저 시작한 쪽이 처리한다.
                let state = app.state::<QuickState>();
                if state
                    .creating
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return;
                }
                let built = WebviewWindowBuilder::new(
                    &app,
                    LABEL,
                    WebviewUrl::App("index.html?view=quick".into()),
                )
                .title("Todo-Lite 빠른 입력")
                .inner_size(560.0, 150.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .center()
                .focused(true)
                .build();
                state.creating.store(false, Ordering::SeqCst);
                match built {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("빠른 입력창을 만들지 못했어요: {e}");
                        return;
                    }
                }
            }
        };
        let _ = window.center();
        if window.show().is_err() {
            return;
        }
        let _ = window.set_focus();
        *app.state::<QuickState>().lock() = Some(Instant::now());
        if std::env::var_os("TODO_LITE_TIMING").is_some() {
            eprintln!(
                "t_quick_shown_ms={}",
                app.state::<Startup>().0.elapsed().as_millis()
            );
        }
        // 방금 만든 창은 프론트엔드가 아직 이 이벤트를 듣고 있지 않을 수 있다. 그래도 괜찮다:
        // 프론트엔드가 화면을 마운트할 때 입력칸에 직접 포커스를 준다.
        let _ = app.emit_to(LABEL, "quick-opened", ());
    });
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        app.state::<QuickState>().set_focused(false);
        let _ = w.hide();
    }
}

/// 보이고 포커스가 있으면 숨기고, 아니면 보여 준다.
pub fn toggle(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        if w.is_visible().unwrap_or(false) && app.state::<QuickState>().is_focused() {
            hide(app);
            return;
        }
    }
    show(app);
}

/// 빠른 입력창이 포커스를 잃었을 때. 뜬 직후 잠깐은 바로 닫지 않고, 그 시간이 지난 뒤에도
/// 여전히 포커스가 없으면 닫는다.
pub fn on_blur(app: &AppHandle) {
    if !HIDE_ON_BLUR {
        return;
    }
    let Some(shown_at) = *app.state::<QuickState>().lock() else {
        return;
    };
    let elapsed = shown_at.elapsed();
    if elapsed > BLUR_GRACE {
        hide(app);
        return;
    }
    let app = app.clone();
    let remaining = BLUR_GRACE - elapsed;
    std::thread::spawn(move || {
        std::thread::sleep(remaining);
        let still_blurred = !app.state::<QuickState>().is_focused();
        let visible = app
            .get_webview_window(LABEL)
            .is_some_and(|w| w.is_visible().unwrap_or(false));
        if still_blurred && visible {
            hide(&app);
        }
    });
}

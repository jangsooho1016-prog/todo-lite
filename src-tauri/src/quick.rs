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
/// 작업 영역(작업 표시줄 제외) 아래쪽과 창 사이의 간격(논리 픽셀).
const BOTTOM_MARGIN: f64 = 24.0;

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

/// 작업 영역 안에서 창을 아래쪽 가운데에 놓았을 때의 왼쪽 위 좌표. 모두 물리 픽셀이다.
/// 창이 작업 영역보다 넓으면 왼쪽에 붙이고, 세로로는 작업 영역 위쪽을 넘지 않는다.
fn bottom_center(
    work_x: i32,
    work_y: i32,
    work_w: u32,
    work_h: u32,
    win_w: u32,
    win_h: u32,
    margin: u32,
) -> (i32, i32) {
    let x = work_x + (work_w.saturating_sub(win_w) / 2) as i32;
    let below = (win_h as i64) + (margin as i64);
    let y = work_y + ((work_h as i64 - below).max(0) as i32);
    (x, y)
}

/// 마우스 커서가 있는 모니터(없으면 창이 있는 모니터, 그것도 없으면 주 모니터)의 작업 영역 아래쪽
/// 가운데로 창을 옮긴다. 모니터를 알 수 없으면 화면 가운데로 둔다.
fn place_bottom_center(app: &AppHandle, window: &tauri::WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size), Ok(window_scale)) =
        (monitor, window.outer_size(), window.scale_factor())
    else {
        let _ = window.center();
        return;
    };
    // outer_size는 창이 지금 있는 모니터 기준이다. 배율이 다른 모니터로 옮기면 크기가 달라지므로
    // 논리 크기를 거쳐 대상 모니터의 물리 크기로 바꾼다.
    let scale = monitor.scale_factor();
    let ratio = scale / window_scale;
    let win_w = (size.width as f64 * ratio).round() as u32;
    let win_h = (size.height as f64 * ratio).round() as u32;
    let margin = (BOTTOM_MARGIN * scale).round() as u32;
    let area = monitor.work_area();
    let (x, y) = bottom_center(
        area.position.x,
        area.position.y,
        area.size.width,
        area.size.height,
        win_w,
        win_h,
        margin,
    );
    if window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .is_err()
    {
        let _ = window.center();
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
        // 보이기 전에 위치를 잡아서 화면 가운데에서 번쩍이지 않게 한다.
        place_bottom_center(&app, &window);
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

#[cfg(test)]
mod tests {
    use super::bottom_center;

    #[test]
    fn primary_monitor_1080p() {
        assert_eq!(bottom_center(0, 0, 1920, 1040, 560, 150, 24), (680, 866));
    }

    #[test]
    fn secondary_monitor_with_negative_origin() {
        assert_eq!(
            bottom_center(-1920, 0, 1920, 1040, 560, 150, 24),
            (-1240, 866)
        );
    }

    #[test]
    fn scaled_150_percent_uses_physical_sizes() {
        assert_eq!(bottom_center(0, 0, 2560, 1392, 840, 225, 36), (860, 1131));
    }

    #[test]
    fn window_wider_than_work_area_sticks_to_left_edge() {
        assert_eq!(bottom_center(100, 0, 400, 1000, 560, 150, 24), (100, 826));
    }

    #[test]
    fn tiny_work_area_never_goes_above_the_top() {
        assert_eq!(bottom_center(0, 50, 1920, 100, 560, 150, 24), (680, 50));
    }
}

//! 전역 단축키: 사용자 입력 문자열의 검증·정규화, 플러그인 `Shortcut` 변환, 등록/교체.

use std::fmt;
use std::str::FromStr;
use std::sync::Mutex;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+Space";

#[derive(Debug, PartialEq, Eq)]
pub enum ShortcutError {
    Empty,
    NoModifier,
    TooFewModifiers,
    UnsupportedKey(String),
    Invalid(String),
    Taken(String),
}

impl fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShortcutError::Empty => f.write_str("단축키를 입력해 주세요"),
            ShortcutError::NoModifier => {
                f.write_str("Ctrl·Alt·Shift 중 하나 이상을 함께 눌러 주세요")
            }
            ShortcutError::TooFewModifiers => f.write_str(
                "글자·숫자·Space 키에는 Ctrl·Alt·Shift 중 두 개 이상을 함께 눌러 주세요",
            ),
            ShortcutError::UnsupportedKey(k) => write!(f, "지원하지 않는 키예요: {k}"),
            ShortcutError::Invalid(s) => write!(f, "단축키 형식이 올바르지 않아요: {s}"),
            ShortcutError::Taken(s) => write!(
                f,
                "다른 프로그램이 이미 쓰고 있거나 등록할 수 없는 조합이에요: {s}"
            ),
        }
    }
}

impl std::error::Error for ShortcutError {}

/// 지원하는 주 키인지 확인하고 표준 표기(`A`, `5`, `F5`, `Space`)로 돌려준다.
fn canonical_key(token: &str) -> Option<String> {
    let upper = token.to_ascii_uppercase();
    if upper == "SPACE" {
        return Some("Space".to_string());
    }
    let mut chars = upper.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => return Some(upper),
        _ => {}
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
        if (1..=12).contains(&n) && upper.len() == n.to_string().len() + 1 {
            return Some(upper);
        }
    }
    None
}

/// `Ctrl+Alt+Space` 같은 표준 형식으로 바꾼다. 수정자 순서는 Ctrl, Alt, Shift.
pub fn normalize(input: &str) -> Result<String, ShortcutError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ShortcutError::Empty);
    }
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    let mut key: Option<String> = None;
    for raw in trimmed.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            return Err(ShortcutError::Invalid(trimmed.to_string()));
        }
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            "super" | "win" | "meta" | "cmd" | "command" => {
                return Err(ShortcutError::UnsupportedKey(token.to_string()));
            }
            _ => {
                if key.is_some() {
                    return Err(ShortcutError::Invalid(trimmed.to_string()));
                }
                key = Some(
                    canonical_key(token)
                        .ok_or_else(|| ShortcutError::UnsupportedKey(token.to_string()))?,
                );
            }
        }
    }
    if !(ctrl || alt || shift) {
        return Err(ShortcutError::NoModifier);
    }
    let key = key.ok_or_else(|| ShortcutError::Invalid(trimmed.to_string()))?;
    // 글자·숫자·Space는 수정자 하나로는 Shift+A, Ctrl+C 같은 흔한 입력을 시스템 전체에서 가로챈다.
    let is_function_key = key.starts_with('F') && key.len() > 1;
    let modifiers = [ctrl, alt, shift].into_iter().filter(|&m| m).count();
    if !is_function_key && modifiers < 2 {
        return Err(ShortcutError::TooFewModifiers);
    }
    let mut parts = Vec::with_capacity(4);
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    parts.push(&key);
    Ok(parts.join("+"))
}

/// 저장돼 있던 값이 지금 규칙으로는 쓸 수 없을 때 사용자에게 알리는 말.
pub const STORED_REJECTED: &str = "저장된 단축키가 더 이상 허용되지 않아 기본값으로 바꿨어요";

/// 시작할 때 저장된 값을 표준 표기로 바꾼다. 규칙이 바뀌어 더는 허용되지 않는 값이면
/// 기본값과 그 사실을 알리는 말을 돌려준다.
pub fn resolve_stored(stored: &str) -> (String, Option<&'static str>) {
    match normalize(stored) {
        Ok(canonical) => (canonical, None),
        Err(_) => (DEFAULT_SHORTCUT.to_string(), Some(STORED_REJECTED)),
    }
}

/// 표준 표기를 플러그인의 `Shortcut`으로 바꾼다. `global-hotkey`의 `FromStr`가
/// `Ctrl`/`Alt`/`Shift`, `A`, `1`, `F5`, `Space`를 대소문자 무관하게 받아들인다.
pub fn to_plugin(canonical: &str) -> Result<Shortcut, ShortcutError> {
    Shortcut::from_str(canonical).map_err(|_| ShortcutError::Invalid(canonical.to_string()))
}

/// 설정 화면에 보여 줄 현재 단축키와 마지막 등록 오류.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShortcutStatus {
    pub current: String,
    pub error: Option<String>,
}

pub struct ShortcutState(pub Mutex<ShortcutStatus>);

impl ShortcutState {
    pub fn new(current: String, error: Option<String>) -> Self {
        ShortcutState(Mutex::new(ShortcutStatus { current, error }))
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, ShortcutStatus> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub fn register(app: &AppHandle, canonical: &str) -> Result<(), ShortcutError> {
    let shortcut = to_plugin(canonical)?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|_| ShortcutError::Taken(canonical.to_string()))
}

/// 새 조합을 먼저 등록하고, 성공했을 때만 이전 조합을 해제한다.
pub fn replace(app: &AppHandle, old: &str, new: &str) -> Result<(), ShortcutError> {
    if old == new {
        return Ok(());
    }
    register(app, new)?;
    if let Ok(old) = to_plugin(old) {
        let _ = app.global_shortcut().unregister(old);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_canonicalizes_case_order_and_whitespace() {
        assert_eq!(normalize(" ctrl + alt + space ").unwrap(), "Ctrl+Alt+Space");
        assert_eq!(normalize("shift+ctrl+a").unwrap(), "Ctrl+Shift+A");
        assert_eq!(normalize("Alt+F1").unwrap(), "Alt+F1");
        assert_eq!(normalize("Alt+shift+1").unwrap(), "Alt+Shift+1");
        assert_eq!(normalize("control+shift+f12").unwrap(), "Ctrl+Shift+F12");
        assert_eq!(normalize(DEFAULT_SHORTCUT).unwrap(), DEFAULT_SHORTCUT);
    }

    #[test]
    fn normalize_rejects_empty_and_missing_modifier() {
        assert!(matches!(normalize(""), Err(ShortcutError::Empty)));
        assert!(matches!(normalize("   "), Err(ShortcutError::Empty)));
        assert!(matches!(normalize("A"), Err(ShortcutError::NoModifier)));
        assert!(matches!(normalize("F5"), Err(ShortcutError::NoModifier)));
    }

    #[test]
    fn normalize_requires_two_modifiers_for_letters_digits_and_space() {
        for s in ["Shift+A", "Ctrl+C", "Alt+F", "Shift+1", "Ctrl+Space"] {
            assert!(
                matches!(normalize(s), Err(ShortcutError::TooFewModifiers)),
                "{s}"
            );
        }
        for s in [
            "Ctrl+Alt+Space",
            "Ctrl+Shift+A",
            "Alt+Shift+1",
            "Alt+F1",
            "Ctrl+F5",
        ] {
            assert_eq!(normalize(s).unwrap(), s);
        }
    }

    #[test]
    fn normalize_rejects_missing_key_and_unsupported_keys() {
        assert!(matches!(
            normalize("Ctrl+Alt"),
            Err(ShortcutError::Invalid(_))
        ));
        assert!(matches!(
            normalize("Ctrl+Enter"),
            Err(ShortcutError::UnsupportedKey(k)) if k == "Enter"
        ));
        assert!(matches!(
            normalize("Ctrl+F13"),
            Err(ShortcutError::UnsupportedKey(_))
        ));
        assert!(matches!(
            normalize("Win+A"),
            Err(ShortcutError::UnsupportedKey(_))
        ));
        assert!(matches!(
            normalize("Ctrl+Super+A"),
            Err(ShortcutError::UnsupportedKey(_))
        ));
    }

    #[test]
    fn normalize_rejects_multiple_keys_and_empty_tokens() {
        assert!(matches!(
            normalize("Ctrl+A+B"),
            Err(ShortcutError::Invalid(_))
        ));
        assert!(matches!(
            normalize("Ctrl++A"),
            Err(ShortcutError::Invalid(_))
        ));
        assert!(matches!(
            normalize("Ctrl+A+"),
            Err(ShortcutError::Invalid(_))
        ));
    }

    #[test]
    fn error_messages_are_korean() {
        assert_eq!(ShortcutError::Empty.to_string(), "단축키를 입력해 주세요");
        assert_eq!(
            ShortcutError::NoModifier.to_string(),
            "Ctrl·Alt·Shift 중 하나 이상을 함께 눌러 주세요"
        );
        assert_eq!(
            ShortcutError::TooFewModifiers.to_string(),
            "글자·숫자·Space 키에는 Ctrl·Alt·Shift 중 두 개 이상을 함께 눌러 주세요"
        );
        assert_eq!(
            ShortcutError::UnsupportedKey("Enter".into()).to_string(),
            "지원하지 않는 키예요: Enter"
        );
    }

    #[test]
    fn resolve_stored_falls_back_to_default_when_no_longer_allowed() {
        assert_eq!(
            resolve_stored("shift+ctrl+a"),
            ("Ctrl+Shift+A".to_string(), None)
        );
        for stored in ["Shift+1", "Ctrl+C", "garbage", "Ctrl+Enter"] {
            assert_eq!(
                resolve_stored(stored),
                (DEFAULT_SHORTCUT.to_string(), Some(STORED_REJECTED)),
                "{stored}"
            );
        }
    }

    #[test]
    fn to_plugin_accepts_supported_combinations() {
        for s in [DEFAULT_SHORTCUT, "Ctrl+Shift+A", "Alt+F1", "Alt+Shift+1"] {
            assert!(to_plugin(s).is_ok(), "{s}");
        }
    }

    #[test]
    fn to_plugin_is_stable_for_equal_strings() {
        assert_eq!(
            to_plugin("Ctrl+Alt+Space").unwrap(),
            to_plugin(&normalize("alt+ctrl+space").unwrap()).unwrap()
        );
        assert_ne!(
            to_plugin("Ctrl+Alt+Space").unwrap(),
            to_plugin("Ctrl+Alt+A").unwrap()
        );
    }
}

# Todo-Lite 데스크톱 UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 완성된 `todo-core` 위에 Tauri 2 + Svelte 5 데스크톱 앱을 얹는다. 사이드바·목록·상세 3단 레이아웃(다크 고정), 자연어 빠른 입력과 실시간 칩 미리보기, 키보드 단축키(PRD F10), 클라우드 폴더 동기화 설정을 제공하고, PRD의 크기·메모리·시작 시간 목표를 실제로 측정한다.

**Architecture:** `src-tauri`(패키지명 `todo-lite`)는 `todo-core`를 얇게 감싼다. 로직은 Tauri와 무관한 `Api` 구조체(`api.rs`)에 두어 단위 테스트하고, `commands.rs`는 `#[tauri::command]` 래퍼만 둔다. 프론트엔드(`ui/`)는 순수 로직(`format.ts`, `keys.ts`)을 따로 두어 Vitest로 검증하고, 상태는 Svelte 5 룬(`state.svelte.ts`) 한 곳에 모은다. Android(v3)를 위해 `lib.rs`에 `run()`과 `mobile_entry_point`를 둔다.

**Tech Stack:** Tauri 2, tauri-plugin-dialog, Svelte 5 (runes) + TypeScript + Vite 6, Vitest 3, svelte-check

**전제:** `docs/superpowers/plans/2026-10-28-todo-core.md`의 코어가 `master`에 병합되어 있다. (테스트 72개 통과). WebView2 런타임(156.x), Node 24, Rust 1.99는 설치되어 있다.

> 커밋 메시지 끝에는 세션이 지정한 attribution 라인(`Co-Authored-By: ...`)을 붙인다.

---

## 이번 계획에서 정한 것 (PRD가 정하지 않은 부분)

- **레이아웃:** 왼쪽 사이드바(보기 5개 + 태그 + 설정) · 가운데 목록(위에 빠른 입력·검색) · 오른쪽 상세 패널. 사용자가 고른 안.
- **테마:** 다크 고정. 라이트/시스템 연동은 만들지 않는다.
- 태그를 누르면 **전체 보기**에서 그 태그로 필터한다. 같은 태그를 다시 누르면 해제. 보기 버튼을 누르면 태그 필터가 풀린다.
- 검색어가 있으면 현재 보기와 관계없이 **전체**에서 검색한다.
- 완료 항목은 목록 아래 **접힌 "완료 N개"** 섹션에 둔다(PRD F1). `완료됨` 보기에서는 완료 항목이 본 목록이다.
- 체크하면 "되돌리기" 토스트를 5초 보여 준다. (보기에서 사라지므로 실수 복구용)
- 삭제는 `confirm` 후 소프트 삭제. 키보드 `Del`도 같은 확인을 거친다.
- 동기화는 **프론트엔드가 시점을 관리**한다: 시작 시, 변경 2초 뒤(디바운스), 5분마다, 수동 버튼. 백엔드는 `sync_now`(가져오기 → 내보내기)만 제공한다.
- 동기화 폴더는 DB `meta` 키 `sync_folder`에 저장한다. (빈 문자열 = 해제. `TaskStore`에 삭제 API가 없다.)
- 데이터 위치: 기본 `app_data_dir()/todo.db`. 환경변수 `TODO_LITE_DATA_DIR`가 있으면 그 폴더를 쓴다. (측정·시연용)
- 시작 시간 측정용: 환경변수 `TODO_LITE_TIMING=1`이면 첫 화면 렌더 뒤 `ready_ms=<ms>`를 stderr에 찍는다.
- IME 안전: 빠른 입력은 `e.isComposing` 중 Enter를 무시한다. (한글 조합 중 전송 방지)
- CSP는 `null`. 원격 콘텐츠와 네트워크 호출이 없는 로컬 앱이기 때문이다.

## 알려진 위험 (실행 전에 알아 둘 것)

- **메모리 목표(약 100MB)는 못 맞출 수도 있다.** WebView2는 여러 프로세스로 구성돼서, 합산 작업 집합이 이를 넘기는 일이 흔하다. Task 11에서 실측하고, 넘으면 조용히 고치지 말고 수치를 그대로 보고한다.
- **첫 Tauri 빌드는 오래 걸린다.** (Windows에서 3–8분) 이후는 증분 빌드.
- `tauri::generate_context!`가 `ui/dist` 존재를 요구한다. 새로 클론하면 `npm --prefix ui run build`를 먼저 하거나 Task 1의 플레이스홀더를 만든다.
- 화면 컴포넌트는 jsdom 없이 `svelte-check` + `vite build` + 실제 실행으로 검증한다. 단위 테스트는 순수 로직(`format.ts`, `keys.ts`)과 Rust `Api`에 둔다.

## 파일 구조

```
Cargo.toml                      # workspace: + src-tauri, release 프로필
package.json                    # 루트: @tauri-apps/cli, 스크립트
.gitignore                      # + node_modules/, /ui/dist, /src-tauri/gen
scripts/
├─ make-icon.mjs                # 의존성 없는 아이콘 원본 PNG 생성
└─ measure.ps1                  # 설치 파일 크기·메모리·시작 시간 측정
crates/todo-core/examples/seed_demo.rs   # 시연용 샘플 데이터
src-tauri/
├─ Cargo.toml  build.rs  tauri.conf.json
├─ capabilities/default.json
├─ icons/                       # tauri icon 으로 생성
└─ src/ main.rs  lib.rs  api.rs  commands.rs
ui/
├─ package.json  vite.config.ts  svelte.config.js  tsconfig.json  index.html
└─ src/
   ├─ main.ts  app.css  App.svelte
   ├─ lib/ types.ts  api.ts  format.ts  format.test.ts  keys.ts  keys.test.ts  state.svelte.ts
   └─ components/ Sidebar  QuickAdd  TaskRow  TaskList  DetailPanel  Settings (.svelte)
```

---

### Task 1: Tauri 셸 스캐폴딩

**Files:**
- Create: `package.json`, `scripts/make-icon.mjs`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/icons/*`
- Modify: `Cargo.toml`, `.gitignore`

- [ ] **Step 1: 루트 `Cargo.toml` 교체**

```toml
[workspace]
resolver = "2"
members = ["crates/todo-core", "src-tauri"]

[profile.release]
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 2: `.gitignore`에 추가**

기존 내용 뒤에 다음 세 줄을 덧붙인다.

```
node_modules/
/ui/dist
/src-tauri/gen/
```

- [ ] **Step 3: 루트 `package.json` 작성**

```json
{
  "name": "todo-lite",
  "private": true,
  "version": "0.1.0",
  "scripts": {
    "dev": "tauri dev",
    "build": "tauri build",
    "tauri": "tauri",
    "ui:check": "npm --prefix ui run check",
    "ui:test": "npm --prefix ui run test"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2"
  }
}
```

- [ ] **Step 4: `src-tauri/Cargo.toml` 작성**

```toml
[package]
name = "todo-lite"
version = "0.1.0"
edition = "2021"

[lib]
name = "todo_lite_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
todo-core = { path = "../crates/todo-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: `src-tauri/build.rs`, `src-tauri/src/main.rs`, 임시 `src-tauri/src/lib.rs`**

`build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

`src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    todo_lite_lib::run()
}
```

`src/lib.rs` (Task 3에서 완성한다. 지금은 빌드만 되게):

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: `src-tauri/tauri.conf.json` 작성**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Todo-Lite",
  "version": "0.1.0",
  "identifier": "com.todolite.app",
  "build": {
    "beforeDevCommand": "npm --prefix ui run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm --prefix ui run build",
    "frontendDist": "../ui/dist"
  },
  "app": {
    "windows": [
      {
        "title": "Todo-Lite",
        "width": 1000,
        "height": 680,
        "minWidth": 720,
        "minHeight": 480,
        "theme": "Dark"
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 7: `src-tauri/capabilities/default.json` 작성**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "main window permissions",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:default"]
}
```

- [ ] **Step 8: 아이콘 원본 생성 스크립트 `scripts/make-icon.mjs`**

외부 패키지 없이 1024×1024 PNG를 만든다. (Node 22 이상의 `zlib.crc32` 사용)

```js
import { mkdirSync, writeFileSync } from 'node:fs';
import { crc32, deflateSync } from 'node:zlib';

const N = 1024;
const BG = [24, 27, 34];
const ACCENT = [124, 156, 255];

// signed distance helpers (pixels)
function roundRect(x, y, half, r) {
  const qx = Math.abs(x) - (half - r);
  const qy = Math.abs(y) - (half - r);
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}
function segment(px, py, ax, ay, bx, by) {
  const pax = px - ax;
  const pay = py - ay;
  const bax = bx - ax;
  const bay = by - ay;
  const h = Math.max(0, Math.min(1, (pax * bax + pay * bay) / (bax * bax + bay * bay)));
  return Math.hypot(pax - bax * h, pay - bay * h);
}
const coverage = (d) => Math.max(0, Math.min(1, 0.5 - d));

const raw = Buffer.alloc((1 + N * 4) * N);
for (let y = 0; y < N; y++) {
  raw[y * (1 + N * 4)] = 0; // filter: none
  for (let x = 0; x < N; x++) {
    const cx = x - N / 2 + 0.5;
    const cy = y - N / 2 + 0.5;
    const tile = coverage(roundRect(cx, cy, 480, 200));
    const check = Math.min(segment(cx, cy, -190, 10, -60, 150), segment(cx, cy, -60, 150, 210, -150)) - 55;
    const mark = coverage(check);
    const o = y * (1 + N * 4) + 1 + x * 4;
    for (let c = 0; c < 3; c++) raw[o + c] = Math.round(BG[c] * (1 - mark) + ACCENT[c] * mark);
    raw[o + 3] = Math.round(tile * 255);
  }
}

function chunk(type, data) {
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body) >>> 0);
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(N, 0);
ihdr.writeUInt32BE(N, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

mkdirSync('src-tauri/icons', { recursive: true });
writeFileSync('src-tauri/icons/source.png', png);
console.log(`wrote src-tauri/icons/source.png (${png.length} bytes)`);
```

- [ ] **Step 9: 의존성 설치, 아이콘 생성, `ui/dist` 플레이스홀더**

```bash
npm install
node scripts/make-icon.mjs
npx tauri icon src-tauri/icons/source.png
mkdir -p ui/dist && printf '<!doctype html><title>placeholder</title>' > ui/dist/index.html
```

Expected: `src-tauri/icons/`에 `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns` 등이 생성된다. `ui/dist`는 gitignore 대상이다.

- [ ] **Step 10: 빌드 확인**

Run: `cargo check -p todo-lite`
Expected: `Finished` (첫 빌드는 수 분 걸린다. 타임아웃을 넉넉히 잡는다.)

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore package.json package-lock.json scripts src-tauri
git commit -m "chore: scaffold Tauri 2 shell (todo-lite) with generated icons"
```

---

### Task 2: `Api` 계층 (TDD)

**Files:**
- Create: `src-tauri/src/api.rs`
- Modify: `src-tauri/src/lib.rs` (모듈 선언만)

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/api.rs`에 테스트 모듈만 먼저 쓴다.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use todo_core::Error;

    fn fixed() -> Now {
        Now::fixed(
            NaiveDate::from_ymd_opt(2026, 10, 28)
                .unwrap()
                .and_hms_opt(15, 0, 0)
                .unwrap(),
        )
    }

    fn api() -> Api {
        Api::from_store(SqliteStore::open_in_memory().unwrap(), fixed).unwrap()
    }

    fn query(view: &str) -> ListQuery {
        serde_json::from_str(&format!(r#"{{"view":"{view}"}}"#)).unwrap()
    }

    #[test]
    fn add_then_list_today_and_tags() {
        let api = api();
        let task = api.add_task("오늘 오후 6시 퇴근 #개인 !중요").unwrap();
        assert_eq!(task.title, "퇴근");

        let today = api.list_tasks(&query("today")).unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(api.list_tags().unwrap()[0].name, "개인");
    }

    #[test]
    fn preview_does_not_persist() {
        let api = api();
        let parsed = api.preview("내일 오후 3시 회의 #학교");
        assert_eq!(parsed.title, "회의");
        assert_eq!(parsed.tags, ["학교"]);
        assert!(api.list_tasks(&query("all")).unwrap().is_empty());
    }

    #[test]
    fn list_query_accepts_optional_fields() {
        let q: ListQuery =
            serde_json::from_str(r#"{"view":"all","tag":"학교","search":null}"#).unwrap();
        assert_eq!(q.tag.as_deref(), Some("학교"));
        assert_eq!(q.search, None);
        assert!(serde_json::from_str::<ListQuery>(r#"{"view":"nope"}"#).is_err());
    }

    #[test]
    fn completing_moves_task_out_of_today() {
        let api = api();
        let task = api.add_task("오늘 운동").unwrap();
        api.set_completed(task.id, true).unwrap();
        assert!(api.list_tasks(&query("today")).unwrap().is_empty());
        assert_eq!(api.list_tasks(&query("completed")).unwrap().len(), 1);
    }

    #[test]
    fn edit_and_delete_roundtrip() {
        let api = api();
        let task = api.add_task("장보기").unwrap();
        let edited = api
            .edit_task(
                task.id,
                TaskEdit {
                    title: "마트 가기".into(),
                    notes: Some("우유".into()),
                    due_date: None,
                    due_time: None,
                    priority: Priority::Low,
                    tags: vec!["집".into()],
                },
            )
            .unwrap();
        assert_eq!(edited.title, "마트 가기");

        api.delete_task(task.id).unwrap();
        assert!(api.list_tasks(&query("all")).unwrap().is_empty());
    }

    #[test]
    fn errors_serialize_as_korean_messages() {
        let empty = serde_json::to_string(&ApiError::Core(Error::EmptyTitle)).unwrap();
        assert_eq!(empty, "\"제목을 입력해 주세요\"");

        let api = api();
        let err = api.add_task("내일").unwrap_err();
        assert!(matches!(err, ApiError::Core(Error::EmptyTitle)));

        let missing = ApiError::Core(Error::NotFound(Uuid::nil())).to_string();
        assert_eq!(missing, "할 일을 찾을 수 없어요");
    }

    #[test]
    fn sync_folder_set_unset_and_unconfigured_sync() {
        let api = api();
        assert_eq!(api.sync_folder().unwrap(), None);
        assert_eq!(api.sync_now().unwrap(), SyncStatus::default());

        api.set_sync_folder(Some("  C:/sync  ".into())).unwrap();
        assert_eq!(api.sync_folder().unwrap().as_deref(), Some("C:/sync"));

        api.set_sync_folder(None).unwrap();
        assert_eq!(api.sync_folder().unwrap(), None);
    }

    #[test]
    fn two_apis_sync_through_a_shared_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().to_string();
        let (a, b) = (api(), api());
        a.set_sync_folder(Some(folder.clone())).unwrap();
        b.set_sync_folder(Some(folder)).unwrap();

        a.add_task("pc에서 만든 일").unwrap();
        let sent = a.sync_now().unwrap();
        assert_eq!((sent.configured, sent.exported), (true, 1));

        let got = b.sync_now().unwrap();
        assert_eq!(got.applied, 1);
        assert_eq!(b.list_tasks(&query("all")).unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: 모듈 선언하고 실패 확인**

`src-tauri/src/lib.rs` 맨 위에 `mod api;`를 추가한다. (아직 `Api`가 없어 컴파일이 실패한다.)

Run: `cargo test -p todo-lite api`
Expected: FAIL (컴파일 에러: cannot find type `Api`, `ListQuery`, ...)

- [ ] **Step 3: 구현 작성**

`api.rs`의 `#[cfg(test)]` 위에 추가:

```rust
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use todo_core::clock::Now;
use todo_core::model::{Priority, Task};
use todo_core::parser::{ParsedTask, RuleBasedParser};
use todo_core::query::{Filter, TagCount, View};
use todo_core::service::{TaskEdit, TodoService};
use todo_core::store::{SqliteStore, TaskStore};
use todo_core::sync::{ensure_device_id, SyncEngine};
use todo_core::Error;
use uuid::Uuid;

const KEY_SYNC_FOLDER: &str = "sync_folder";

type Service = TodoService<SqliteStore, RuleBasedParser>;

#[derive(Debug)]
pub enum ApiError {
    Core(Error),
}

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError::Core(e)
    }
}

/// 화면에 그대로 보여 줄 한국어 메시지가 `Display`이자 직렬화 결과다.
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    /// 사용자에게 그대로 보여 줄 한국어 메시지.
    fn message(&self) -> String {
        match self {
            ApiError::Core(Error::EmptyTitle) => "제목을 입력해 주세요".to_string(),
            ApiError::Core(Error::NotFound(_)) => "할 일을 찾을 수 없어요".to_string(),
            ApiError::Core(Error::Io(e)) => format!("폴더에 접근할 수 없어요: {e}"),
            ApiError::Core(e) => format!("저장 중 문제가 생겼어요: {e}"),
        }
    }
}

impl Serialize for ApiError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.message())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewName {
    Today,
    Upcoming,
    Overdue,
    All,
    Completed,
}

impl From<ViewName> for View {
    fn from(v: ViewName) -> Self {
        match v {
            ViewName::Today => View::Today,
            ViewName::Upcoming => View::Upcoming,
            ViewName::Overdue => View::Overdue,
            ViewName::All => View::All,
            ViewName::Completed => View::Completed,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub view: ViewName,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ParsedDto {
    pub title: String,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

impl From<ParsedTask> for ParsedDto {
    fn from(p: ParsedTask) -> Self {
        ParsedDto {
            title: p.title,
            due_date: p.due_date,
            due_time: p.due_time,
            priority: p.priority,
            tags: p.tags,
        }
    }
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SyncStatus {
    pub configured: bool,
    pub exported: usize,
    pub applied: usize,
    pub corrupt_lines: usize,
}

/// Tauri와 무관한 앱 로직. `rusqlite::Connection`은 `Sync`가 아니라서 `Mutex`로 감싼다.
pub struct Api {
    service: Mutex<Service>,
    device_id: String,
    clock: fn() -> Now,
}

impl Api {
    pub fn open(path: &Path) -> Result<Self, ApiError> {
        Self::from_store(SqliteStore::open(path)?, Now::system)
    }

    pub fn from_store(store: SqliteStore, clock: fn() -> Now) -> Result<Self, ApiError> {
        let device_id = ensure_device_id(&store)?;
        let service = TodoService::new(store, RuleBasedParser::new(), device_id.clone());
        Ok(Api {
            service: Mutex::new(service),
            device_id,
            clock,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Service> {
        self.service.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn preview(&self, input: &str) -> ParsedDto {
        self.lock().preview(input, (self.clock)()).into()
    }

    pub fn add_task(&self, input: &str) -> Result<Task, ApiError> {
        Ok(self.lock().add_from_input(input, (self.clock)())?)
    }

    pub fn list_tasks(&self, q: &ListQuery) -> Result<Vec<Task>, ApiError> {
        let filter = Filter {
            view: q.view.into(),
            tag: q.tag.as_deref(),
            search: q.search.as_deref(),
        };
        Ok(self.lock().list(&filter, (self.clock)())?)
    }

    pub fn list_tags(&self) -> Result<Vec<TagCount>, ApiError> {
        Ok(self.lock().tags()?)
    }

    pub fn edit_task(&self, id: Uuid, edit: TaskEdit) -> Result<Task, ApiError> {
        Ok(self.lock().edit(id, edit, (self.clock)())?)
    }

    pub fn set_completed(&self, id: Uuid, done: bool) -> Result<Task, ApiError> {
        Ok(self.lock().set_completed(id, done, (self.clock)())?)
    }

    pub fn delete_task(&self, id: Uuid) -> Result<(), ApiError> {
        Ok(self.lock().delete(id, (self.clock)())?)
    }

    pub fn sync_folder(&self) -> Result<Option<String>, ApiError> {
        let value = self.lock().store().meta_get(KEY_SYNC_FOLDER)?;
        Ok(value.filter(|f| !f.is_empty()))
    }

    /// `None`이나 빈 문자열이면 해제한다. (`TaskStore`에 삭제 API가 없어 빈 값으로 표시)
    pub fn set_sync_folder(&self, folder: Option<String>) -> Result<(), ApiError> {
        let value = folder.map(|f| f.trim().to_string()).unwrap_or_default();
        self.lock().store().meta_set(KEY_SYNC_FOLDER, &value)?;
        Ok(())
    }

    /// 다른 기기의 변경을 먼저 가져오고, 이 기기의 변경을 내보낸다.
    pub fn sync_now(&self) -> Result<SyncStatus, ApiError> {
        let service = self.lock();
        let Some(folder) = service
            .store()
            .meta_get(KEY_SYNC_FOLDER)?
            .filter(|f| !f.is_empty())
        else {
            return Ok(SyncStatus::default());
        };
        let engine = SyncEngine::new(self.device_id.clone(), folder);
        let report = engine.import(service.store())?;
        let exported = engine.export(service.store())?;
        Ok(SyncStatus {
            configured: true,
            exported,
            applied: report.applied,
            corrupt_lines: report.corrupt_lines,
        })
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p todo-lite api`
Expected: PASS (8 tests). 실패하면 출력을 보고 테스트 기대값이 틀렸는지 구현이 틀렸는지 먼저 판단한다. (테스트를 구현에 맞춰 고치지 않는다.)

(clippy는 `Api`가 아직 커맨드에 연결되지 않아 `dead_code` 경고가 나므로 Task 3에서 함께 확인한다.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src
git commit -m "feat(app): add Tauri-independent Api layer with tests"
```

---

### Task 3: Tauri 커맨드와 앱 조립

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `commands.rs` 작성**

```rust
use tauri::State;
use todo_core::model::Task;
use todo_core::query::TagCount;
use todo_core::service::TaskEdit;
use uuid::Uuid;

use crate::api::{Api, ApiError, ListQuery, ParsedDto, SyncStatus};
use crate::Startup;

#[tauri::command]
pub fn preview_input(api: State<'_, Api>, input: String) -> ParsedDto {
    api.preview(&input)
}

#[tauri::command]
pub fn add_task(api: State<'_, Api>, input: String) -> Result<Task, ApiError> {
    api.add_task(&input)
}

#[tauri::command]
pub fn list_tasks(api: State<'_, Api>, query: ListQuery) -> Result<Vec<Task>, ApiError> {
    api.list_tasks(&query)
}

#[tauri::command]
pub fn list_tags(api: State<'_, Api>) -> Result<Vec<TagCount>, ApiError> {
    api.list_tags()
}

#[tauri::command]
pub fn edit_task(api: State<'_, Api>, id: Uuid, edit: TaskEdit) -> Result<Task, ApiError> {
    api.edit_task(id, edit)
}

#[tauri::command]
pub fn set_completed(api: State<'_, Api>, id: Uuid, done: bool) -> Result<Task, ApiError> {
    api.set_completed(id, done)
}

#[tauri::command]
pub fn delete_task(api: State<'_, Api>, id: Uuid) -> Result<(), ApiError> {
    api.delete_task(id)
}

#[tauri::command]
pub fn get_sync_folder(api: State<'_, Api>) -> Result<Option<String>, ApiError> {
    api.sync_folder()
}

#[tauri::command]
pub fn set_sync_folder(api: State<'_, Api>, folder: Option<String>) -> Result<(), ApiError> {
    api.set_sync_folder(folder)
}

#[tauri::command]
pub fn sync_now(api: State<'_, Api>) -> Result<SyncStatus, ApiError> {
    api.sync_now()
}

/// 첫 화면이 그려진 뒤 프론트엔드가 부른다. 시작 시간 측정용.
#[tauri::command]
pub fn report_ready(startup: State<'_, Startup>) {
    if std::env::var_os("TODO_LITE_TIMING").is_some() {
        eprintln!("ready_ms={}", startup.0.elapsed().as_millis());
    }
}
```

- [ ] **Step 2: `lib.rs` 완성**

```rust
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
```

- [ ] **Step 3: 빌드·린트·테스트**

```bash
cargo clippy -p todo-lite --all-targets -- -D warnings
cargo test -p todo-lite
cargo fmt --all --check
```
Expected: clippy 깨끗, 테스트 8개 통과, fmt 통과. (`cargo fmt --all`로 정리한 뒤 다시 확인해도 된다.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src
git commit -m "feat(app): wire Api to Tauri commands and app setup"
```

---

### Task 4: UI 프로젝트 스캐폴딩

**Files:**
- Create: `ui/package.json`, `ui/vite.config.ts`, `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.ts`, `ui/src/App.svelte`(임시)

- [ ] **Step 1: `ui/package.json`**

```json
{
  "name": "todo-lite-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "test": "vitest run"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0",
    "vitest": "^3.0.0"
  }
}
```

- [ ] **Step 2: 설정 파일들**

`ui/vite.config.ts`:

```ts
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2021', sourcemap: false },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
});
```

`ui/svelte.config.js`:

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default { preprocess: vitePreprocess() };
```

`ui/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2021",
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

`ui/index.html`:

```html
<!doctype html>
<html lang="ko">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Todo-Lite</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`ui/src/main.ts`:

```ts
import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';

export default mount(App, { target: document.getElementById('app')! });
```

임시 `ui/src/App.svelte`와 `ui/src/app.css`:

```svelte
<h1>Todo-Lite</h1>
```

```css
body { background: #14161b; color: #e6e8ee; }
```

- [ ] **Step 3: 설치와 빌드 확인**

```bash
npm --prefix ui install
npm --prefix ui run check
npm --prefix ui run build
```
Expected: `svelte-check found 0 errors`, 빌드 후 `ui/dist/index.html` 생성. (Task 1의 플레이스홀더를 덮어쓴다.)

- [ ] **Step 4: Commit**

```bash
git add ui/package.json ui/package-lock.json ui/vite.config.ts ui/svelte.config.js ui/tsconfig.json ui/index.html ui/src
git commit -m "chore(ui): scaffold Svelte 5 + Vite + Vitest project"
```

---

### Task 5: 순수 로직 — 타입, 포맷, 키보드 (TDD)

**Files:**
- Create: `ui/src/lib/types.ts`, `ui/src/lib/format.ts`, `ui/src/lib/format.test.ts`, `ui/src/lib/keys.ts`, `ui/src/lib/keys.test.ts`

- [ ] **Step 1: `types.ts` 작성** (백엔드 JSON과 1:1. 필드명은 snake_case 그대로)

```ts
export type Priority = 'high' | 'normal' | 'low';
export type ViewName = 'today' | 'upcoming' | 'overdue' | 'all' | 'completed';

export interface Task {
  id: string;
  title: string;
  notes: string | null;
  due_date: string | null; // "2026-10-29"
  due_time: string | null; // "15:00:00"
  priority: Priority;
  tags: string[];
  completed_at: string | null;
  created_at: string;
  updated_at: string;
  updated_by: string;
  deleted: boolean;
}

export interface ParsedTask {
  title: string;
  due_date: string | null;
  due_time: string | null;
  priority: Priority;
  tags: string[];
}

export interface TagCount {
  name: string;
  count: number;
}

export interface ListQuery {
  view: ViewName;
  tag: string | null;
  search: string | null;
}

export interface TaskEdit {
  title: string;
  notes: string | null;
  due_date: string | null;
  due_time: string | null;
  priority: Priority;
  tags: string[];
}

export interface SyncStatus {
  configured: boolean;
  exported: number;
  applied: number;
  corrupt_lines: number;
}
```

- [ ] **Step 2: `format.test.ts` — 실패하는 테스트 작성**

```ts
import { describe, expect, it } from 'vitest';
import {
  describeSync,
  errorText,
  formatDue,
  isOverdue,
  nowParts,
  parseTagInput,
  tagsToText,
  timeInputValue,
  todayIso,
} from './format';

const TODAY = '2026-10-28';

describe('todayIso / nowParts', () => {
  it('uses the local calendar date', () => {
    const d = new Date(2026, 9, 28, 15, 5);
    expect(todayIso(d)).toBe('2026-10-28');
    expect(nowParts(d)).toEqual({ date: '2026-10-28', time: '15:05' });
  });
});

describe('formatDue', () => {
  it('returns empty when there is no date', () => {
    expect(formatDue(null, '15:00:00', TODAY)).toBe('');
  });
  it('uses relative words for nearby days', () => {
    expect(formatDue('2026-10-28', null, TODAY)).toBe('오늘');
    expect(formatDue('2026-10-29', null, TODAY)).toBe('내일');
    expect(formatDue('2026-10-27', null, TODAY)).toBe('어제');
  });
  it('uses M/D within the year and Y/M/D otherwise', () => {
    expect(formatDue('2026-10-30', null, TODAY)).toBe('10/30');
    expect(formatDue('2027-01-05', null, TODAY)).toBe('2027/1/5');
  });
  it('appends HH:MM and trims seconds', () => {
    expect(formatDue('2026-10-28', '15:00:00', TODAY)).toBe('오늘 15:00');
    expect(formatDue('2026-10-29', '09:30', TODAY)).toBe('내일 09:30');
  });
});

describe('isOverdue', () => {
  const now = { date: '2026-10-28', time: '15:00' };
  it('is false for done tasks and tasks without a date', () => {
    expect(isOverdue({ due_date: '2026-10-01', due_time: null, completed_at: 'x' }, now)).toBe(false);
    expect(isOverdue({ due_date: null, due_time: null, completed_at: null }, now)).toBe(false);
  });
  it('is true for past dates', () => {
    expect(isOverdue({ due_date: '2026-10-27', due_time: null, completed_at: null }, now)).toBe(true);
  });
  it('compares the time only on the same day', () => {
    expect(isOverdue({ due_date: '2026-10-28', due_time: '09:00:00', completed_at: null }, now)).toBe(true);
    expect(isOverdue({ due_date: '2026-10-28', due_time: '19:00:00', completed_at: null }, now)).toBe(false);
    expect(isOverdue({ due_date: '2026-10-28', due_time: null, completed_at: null }, now)).toBe(false);
  });
});

describe('time and tag helpers', () => {
  it('timeInputValue trims seconds', () => {
    expect(timeInputValue('15:00:00')).toBe('15:00');
    expect(timeInputValue(null)).toBe('');
  });
  it('parseTagInput splits on spaces/commas, strips # and dedupes case-insensitively', () => {
    expect(parseTagInput('#학교, 개인 #학교 Work work')).toEqual(['학교', '개인', 'Work']);
    expect(parseTagInput('  # ,, ')).toEqual([]);
  });
  it('tagsToText prefixes #', () => {
    expect(tagsToText(['학교', '개인'])).toBe('#학교 #개인');
    expect(tagsToText([])).toBe('');
  });
});

describe('errorText', () => {
  it('handles strings, Errors and unknown values', () => {
    expect(errorText('제목을 입력해 주세요')).toBe('제목을 입력해 주세요');
    expect(errorText(new Error('boom'))).toBe('boom');
    expect(errorText(42)).toBe('42');
  });
});

describe('describeSync', () => {
  it('summarises counts', () => {
    expect(describeSync({ configured: true, exported: 2, applied: 1, corrupt_lines: 0 }, '15:05')).toBe(
      '15:05 동기화 · 가져옴 1 · 보냄 2',
    );
  });
  it('mentions skipped corrupt lines', () => {
    expect(describeSync({ configured: true, exported: 0, applied: 0, corrupt_lines: 3 }, '09:00')).toBe(
      '09:00 동기화 · 가져옴 0 · 보냄 0 · 손상된 줄 3개 건너뜀',
    );
  });
});
```

Run: `npm --prefix ui run test`
Expected: FAIL (`./format` 모듈이 없음)

- [ ] **Step 3: `format.ts` 구현**

```ts
import type { SyncStatus, Task } from './types';

const MS_PER_DAY = 86_400_000;
const pad = (n: number) => String(n).padStart(2, '0');

function dayNumber(iso: string): number {
  const [y, m, d] = iso.split('-').map(Number);
  return Date.UTC(y, m - 1, d) / MS_PER_DAY;
}

export function todayIso(now: Date = new Date()): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export function nowParts(now: Date = new Date()): { date: string; time: string } {
  return { date: todayIso(now), time: `${pad(now.getHours())}:${pad(now.getMinutes())}` };
}

/** "15:00:00" → "15:00". `<input type="time">`의 value로 쓴다. */
export function timeInputValue(time: string | null): string {
  return time ? time.slice(0, 5) : '';
}

export function formatDue(dueDate: string | null, dueTime: string | null, today: string): string {
  if (!dueDate) return '';
  const diff = dayNumber(dueDate) - dayNumber(today);
  let label: string;
  if (diff === 0) label = '오늘';
  else if (diff === 1) label = '내일';
  else if (diff === -1) label = '어제';
  else {
    const [y, m, d] = dueDate.split('-').map(Number);
    label = y === Number(today.slice(0, 4)) ? `${m}/${d}` : `${y}/${m}/${d}`;
  }
  const time = timeInputValue(dueTime);
  return time ? `${label} ${time}` : label;
}

export function isOverdue(
  task: Pick<Task, 'due_date' | 'due_time' | 'completed_at'>,
  now: { date: string; time: string },
): boolean {
  if (task.completed_at || !task.due_date) return false;
  if (task.due_date < now.date) return true;
  return task.due_date === now.date && !!task.due_time && timeInputValue(task.due_time) < now.time;
}

export function parseTagInput(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of text.split(/[\s,]+/)) {
    const tag = raw.replace(/^#+/, '').trim();
    if (!tag) continue;
    const key = tag.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(tag);
  }
  return out;
}

export const tagsToText = (tags: string[]): string => tags.map((t) => `#${t}`).join(' ');

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

export function describeSync(s: SyncStatus, time: string): string {
  const base = `${time} 동기화 · 가져옴 ${s.applied} · 보냄 ${s.exported}`;
  return s.corrupt_lines > 0 ? `${base} · 손상된 줄 ${s.corrupt_lines}개 건너뜀` : base;
}
```

Run: `npm --prefix ui run test`
Expected: PASS (format 테스트 전부)

- [ ] **Step 4: `keys.test.ts` — 실패하는 테스트 작성**

```ts
import { describe, expect, it } from 'vitest';
import { actionForKey, targetKind } from './keys';

const key = (k: string, extra: Partial<{ ctrlKey: boolean; isComposing: boolean }> = {}) => ({
  key: k,
  ctrlKey: false,
  ...extra,
});

describe('actionForKey', () => {
  it('Ctrl+N focuses quick add and Ctrl+F focuses search, even while typing', () => {
    for (const kind of ['none', 'text', 'control'] as const) {
      expect(actionForKey(key('n', { ctrlKey: true }), kind)).toEqual({ type: 'focus-add' });
      expect(actionForKey(key('F', { ctrlKey: true }), kind)).toEqual({ type: 'focus-search' });
    }
    expect(actionForKey(key('x', { ctrlKey: true }), 'none')).toBeNull();
  });

  it('Escape always works', () => {
    expect(actionForKey(key('Escape'), 'text')).toEqual({ type: 'escape' });
    expect(actionForKey(key('Escape'), 'none')).toEqual({ type: 'escape' });
  });

  it('ignores everything while IME composition is active', () => {
    expect(actionForKey(key('Enter', { isComposing: true }), 'none')).toBeNull();
    expect(actionForKey(key('n', { ctrlKey: true, isComposing: true }), 'none')).toBeNull();
  });

  it('list keys work when nothing text-like is focused', () => {
    expect(actionForKey(key('ArrowDown'), 'none')).toEqual({ type: 'move', delta: 1 });
    expect(actionForKey(key('ArrowUp'), 'none')).toEqual({ type: 'move', delta: -1 });
    expect(actionForKey(key(' '), 'none')).toEqual({ type: 'toggle' });
    expect(actionForKey(key('Enter'), 'none')).toEqual({ type: 'edit' });
    expect(actionForKey(key('Delete'), 'none')).toEqual({ type: 'delete' });
    expect(actionForKey(key('a'), 'none')).toBeNull();
  });

  it('does not steal keys from text fields', () => {
    for (const k of ['ArrowDown', 'ArrowUp', ' ', 'Enter', 'Delete']) {
      expect(actionForKey(key(k), 'text')).toBeNull();
    }
  });

  it('on buttons/checkboxes, arrows and Delete work but Space/Enter stay native', () => {
    expect(actionForKey(key('ArrowDown'), 'control')).toEqual({ type: 'move', delta: 1 });
    expect(actionForKey(key('Delete'), 'control')).toEqual({ type: 'delete' });
    expect(actionForKey(key(' '), 'control')).toBeNull();
    expect(actionForKey(key('Enter'), 'control')).toBeNull();
  });
});

describe('targetKind', () => {
  const el = (tagName: string, extra: Record<string, unknown> = {}) => ({ tagName, ...extra }) as unknown as EventTarget;

  it('classifies text-like fields', () => {
    expect(targetKind(el('TEXTAREA'))).toBe('text');
    expect(targetKind(el('SELECT'))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'text' }))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'search' }))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'date' }))).toBe('text');
    expect(targetKind(el('DIV', { isContentEditable: true }))).toBe('text');
  });
  it('classifies clickable controls', () => {
    expect(targetKind(el('BUTTON'))).toBe('control');
    expect(targetKind(el('INPUT', { type: 'checkbox' }))).toBe('control');
  });
  it('treats everything else as none', () => {
    expect(targetKind(el('DIV'))).toBe('none');
    expect(targetKind(null)).toBe('none');
  });
});
```

Run: `npm --prefix ui run test`
Expected: FAIL (`./keys` 모듈이 없음. format 테스트는 통과)

- [ ] **Step 5: `keys.ts` 구현**

```ts
export type KeyAction =
  | { type: 'focus-add' }
  | { type: 'focus-search' }
  | { type: 'move'; delta: 1 | -1 }
  | { type: 'toggle' }
  | { type: 'edit' }
  | { type: 'delete' }
  | { type: 'escape' };

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  isComposing?: boolean;
}

/** text: 글자를 입력하는 칸. control: 버튼·체크박스. none: 그 밖(본문 등). */
export type TargetKind = 'text' | 'control' | 'none';

export function actionForKey(e: KeyInput, target: TargetKind): KeyAction | null {
  // 한글 조합 중에는 어떤 단축키도 가로채지 않는다.
  if (e.isComposing) return null;

  if (e.ctrlKey) {
    const k = e.key.toLowerCase();
    if (k === 'n') return { type: 'focus-add' };
    if (k === 'f') return { type: 'focus-search' };
    return null;
  }
  if (e.key === 'Escape') return { type: 'escape' };
  if (target === 'text') return null;

  switch (e.key) {
    case 'ArrowDown':
      return { type: 'move', delta: 1 };
    case 'ArrowUp':
      return { type: 'move', delta: -1 };
    case 'Delete':
      return { type: 'delete' };
    case ' ':
      // 버튼·체크박스에서는 브라우저 기본 동작(클릭)을 그대로 둔다. 이중 토글 방지.
      return target === 'control' ? null : { type: 'toggle' };
    case 'Enter':
      return target === 'control' ? null : { type: 'edit' };
    default:
      return null;
  }
}

const CLICKABLE_INPUTS = new Set(['checkbox', 'radio', 'button', 'submit', 'reset']);

export function targetKind(target: EventTarget | null): TargetKind {
  const el = target as { tagName?: unknown; type?: unknown; isContentEditable?: unknown } | null;
  if (!el || typeof el.tagName !== 'string') return 'none';
  if (el.isContentEditable === true) return 'text';
  switch (el.tagName) {
    case 'TEXTAREA':
    case 'SELECT':
      return 'text';
    case 'BUTTON':
      return 'control';
    case 'INPUT':
      return CLICKABLE_INPUTS.has(String(el.type)) ? 'control' : 'text';
    default:
      return 'none';
  }
}
```

- [ ] **Step 6: 전체 통과 확인**

Run: `npm --prefix ui run test && npm --prefix ui run check`
Expected: 테스트 전부 PASS, svelte-check 에러 0

- [ ] **Step 7: Commit**

```bash
git add ui/src/lib
git commit -m "feat(ui): add typed API shapes, formatting helpers and keyboard map with tests"
```

---

### Task 6: API 클라이언트와 앱 상태

**Files:**
- Create: `ui/src/lib/api.ts`, `ui/src/lib/state.svelte.ts`

- [ ] **Step 1: `api.ts` 작성** (Tauri 커맨드 인자는 최상위 이름이 camelCase로 변환된다. 모두 한 단어라 그대로다.)

```ts
import { invoke } from '@tauri-apps/api/core';
import type { ListQuery, ParsedTask, SyncStatus, TagCount, Task, TaskEdit } from './types';

export const api = {
  preview: (input: string) => invoke<ParsedTask>('preview_input', { input }),
  addTask: (input: string) => invoke<Task>('add_task', { input }),
  listTasks: (query: ListQuery) => invoke<Task[]>('list_tasks', { query }),
  listTags: () => invoke<TagCount[]>('list_tags'),
  editTask: (id: string, edit: TaskEdit) => invoke<Task>('edit_task', { id, edit }),
  setCompleted: (id: string, done: boolean) => invoke<Task>('set_completed', { id, done }),
  deleteTask: (id: string) => invoke<void>('delete_task', { id }),
  syncFolder: () => invoke<string | null>('get_sync_folder'),
  setSyncFolder: (folder: string | null) => invoke<void>('set_sync_folder', { folder }),
  syncNow: () => invoke<SyncStatus>('sync_now'),
  ready: () => invoke<void>('report_ready'),
};
```

- [ ] **Step 2: `state.svelte.ts` 작성**

```ts
import { api } from './api';
import { describeSync, errorText, nowParts } from './format';
import type { TagCount, Task, TaskEdit, ViewName } from './types';

export const VIEWS: { name: ViewName; label: string }[] = [
  { name: 'today', label: '오늘' },
  { name: 'upcoming', label: '예정' },
  { name: 'overdue', label: '지난 일' },
  { name: 'all', label: '전체' },
  { name: 'completed', label: '완료됨' },
];

const SYNC_INTERVAL_MS = 5 * 60 * 1000;
const SYNC_DEBOUNCE_MS = 2000;
const SEARCH_DEBOUNCE_MS = 150;
const TOAST_MS = 5000;

class AppState {
  view = $state<ViewName>('today');
  tag = $state<string | null>(null);
  search = $state('');
  tasks = $state<Task[]>([]);
  tags = $state<TagCount[]>([]);
  selectedId = $state<string | null>(null);
  doneOpen = $state(false);
  error = $state<string | null>(null);
  undo = $state<{ id: string; title: string } | null>(null);
  syncFolder = $state<string | null>(null);
  syncMessage = $state<string | null>(null);
  syncing = $state(false);

  selected = $derived(this.tasks.find((t) => t.id === this.selectedId) ?? null);
  openTasks = $derived(this.tasks.filter((t) => !t.completed_at));
  doneTasks = $derived(this.tasks.filter((t) => !!t.completed_at));
  /** 본 목록: 완료됨 보기에서는 완료 항목, 그 밖에는 미완료 항목. */
  mainTasks = $derived(this.view === 'completed' && !this.searching ? this.doneTasks : this.openTasks);
  /** ↑/↓로 오갈 수 있는 순서. */
  navTasks = $derived(
    this.view === 'completed' && !this.searching
      ? this.doneTasks
      : this.doneOpen
        ? [...this.openTasks, ...this.doneTasks]
        : this.openTasks,
  );

  private refreshSeq = 0;
  private syncTimer: ReturnType<typeof setTimeout> | undefined;
  private searchTimer: ReturnType<typeof setTimeout> | undefined;
  private undoTimer: ReturnType<typeof setTimeout> | undefined;
  private errorTimer: ReturnType<typeof setTimeout> | undefined;

  get searching(): boolean {
    return this.search.trim() !== '';
  }

  async init() {
    try {
      this.syncFolder = await api.syncFolder();
    } catch (e) {
      this.fail(e);
    }
    await this.refresh();
    void this.runSync();
    setInterval(() => void this.runSync(), SYNC_INTERVAL_MS);
    void api.ready().catch(() => {});
  }

  async refresh() {
    const seq = ++this.refreshSeq;
    try {
      const [tasks, tags] = await Promise.all([
        api.listTasks({
          view: this.searching ? 'all' : this.view,
          tag: this.tag,
          search: this.searching ? this.search.trim() : null,
        }),
        api.listTags(),
      ]);
      if (seq !== this.refreshSeq) return; // 더 최근 요청이 있으면 버린다
      this.tasks = tasks;
      this.tags = tags;
      if (this.selectedId && !tasks.some((t) => t.id === this.selectedId)) this.selectedId = null;
    } catch (e) {
      this.fail(e);
    }
  }

  setView(view: ViewName) {
    this.view = view;
    this.tag = null;
    this.search = '';
    this.selectedId = null;
    void this.refresh();
  }

  setTag(name: string) {
    this.tag = this.tag === name ? null : name;
    this.view = 'all';
    this.search = '';
    this.selectedId = null;
    void this.refresh();
  }

  onSearchInput() {
    clearTimeout(this.searchTimer);
    this.searchTimer = setTimeout(() => void this.refresh(), SEARCH_DEBOUNCE_MS);
  }

  clearSearch() {
    this.search = '';
    void this.refresh();
  }

  async addFromInput(text: string): Promise<Task | null> {
    try {
      const task = await api.addTask(text);
      await this.refresh();
      if (this.tasks.some((t) => t.id === task.id)) this.selectedId = task.id;
      this.scheduleSync();
      return task;
    } catch (e) {
      this.fail(e);
      return null;
    }
  }

  async toggle(id: string) {
    const task = this.tasks.find((t) => t.id === id);
    if (!task) return;
    const done = !task.completed_at;
    try {
      await api.setCompleted(id, done);
      if (done) this.showUndo({ id, title: task.title });
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  toggleSelected() {
    if (this.selectedId) void this.toggle(this.selectedId);
  }

  async undoComplete() {
    const pending = this.undo;
    if (!pending) return;
    this.undo = null;
    try {
      await api.setCompleted(pending.id, false);
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  /** 저장에 성공하면 true. 실패하면 화면이 원래 값으로 되돌릴 수 있게 false. */
  async save(id: string, edit: TaskEdit): Promise<boolean> {
    try {
      await api.editTask(id, edit);
      await this.refresh();
      this.scheduleSync();
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    }
  }

  async remove(id: string) {
    if (!confirm('이 할 일을 삭제할까요?')) return;
    try {
      await api.deleteTask(id);
      if (this.selectedId === id) this.selectedId = null;
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  removeSelected() {
    if (this.selectedId) void this.remove(this.selectedId);
  }

  moveSelection(delta: 1 | -1) {
    const list = this.navTasks;
    if (list.length === 0) return;
    const i = list.findIndex((t) => t.id === this.selectedId);
    const next = i === -1 ? (delta === 1 ? 0 : list.length - 1) : Math.min(list.length - 1, Math.max(0, i + delta));
    this.selectedId = list[next].id;
  }

  async setSyncFolder(folder: string | null) {
    try {
      await api.setSyncFolder(folder);
      this.syncFolder = folder;
      this.syncMessage = null;
      if (folder) await this.runSync();
    } catch (e) {
      this.fail(e);
    }
  }

  async runSync() {
    if (!this.syncFolder || this.syncing) return;
    this.syncing = true;
    try {
      const status = await api.syncNow();
      if (status.configured) {
        this.syncMessage = describeSync(status, nowParts().time);
        if (status.applied > 0) await this.refresh();
      }
    } catch (e) {
      this.syncMessage = `동기화 실패: ${errorText(e)}`;
    } finally {
      this.syncing = false;
    }
  }

  private scheduleSync() {
    if (!this.syncFolder) return;
    clearTimeout(this.syncTimer);
    this.syncTimer = setTimeout(() => void this.runSync(), SYNC_DEBOUNCE_MS);
  }

  private showUndo(undo: { id: string; title: string }) {
    this.undo = undo;
    clearTimeout(this.undoTimer);
    this.undoTimer = setTimeout(() => (this.undo = null), TOAST_MS);
  }

  private fail(e: unknown) {
    this.error = errorText(e);
    clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), TOAST_MS);
  }
}

export const app = new AppState();
```

- [ ] **Step 3: 타입 검사**

Run: `npm --prefix ui run check`
Expected: 에러 0

- [ ] **Step 4: Commit**

```bash
git add ui/src/lib/api.ts ui/src/lib/state.svelte.ts
git commit -m "feat(ui): add Tauri API client and reactive app state"
```

---

### Task 7: 컴포넌트 — 사이드바, 빠른 입력, 목록

**Files:**
- Create: `ui/src/components/Sidebar.svelte`, `QuickAdd.svelte`, `TaskRow.svelte`, `TaskList.svelte`

- [ ] **Step 1: `Sidebar.svelte`**

```svelte
<script lang="ts">
  import { app, VIEWS } from '../lib/state.svelte';

  let { onsettings }: { onsettings: () => void } = $props();
</script>

<nav class="sidebar" aria-label="보기">
  <ul class="nav-list">
    {#each VIEWS as v (v.name)}
      <li>
        <button
          class="nav-item"
          class:active={app.view === v.name && !app.tag && !app.searching}
          onclick={() => app.setView(v.name)}
        >
          {v.label}
        </button>
      </li>
    {/each}
  </ul>

  {#if app.tags.length > 0}
    <div class="nav-title">태그</div>
    <ul class="nav-list">
      {#each app.tags as t (t.name)}
        <li>
          <button class="nav-item" class:active={app.tag === t.name} onclick={() => app.setTag(t.name)}>
            <span>#{t.name}</span>
            <span class="count">{t.count}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="spacer"></div>
  <button class="nav-item" onclick={onsettings}>
    설정{#if app.syncFolder}<span class="dot" title="동기화 사용 중"></span>{/if}
  </button>
</nav>
```

- [ ] **Step 2: `QuickAdd.svelte`** — 한글 조합 중 Enter를 무시하고, 늦게 도착한 미리보기 응답은 버린다.

```svelte
<script lang="ts">
  import { api } from '../lib/api';
  import { formatDue, todayIso } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { ParsedTask } from '../lib/types';

  let text = $state('');
  let parsed = $state<ParsedTask | null>(null);
  let seq = 0;

  const due = $derived(parsed ? formatDue(parsed.due_date, parsed.due_time, todayIso()) : '');

  async function onInput() {
    const mine = ++seq;
    if (!text.trim()) {
      parsed = null;
      return;
    }
    try {
      const result = await api.preview(text);
      if (mine === seq) parsed = result;
    } catch {
      // 미리보기 실패는 조용히 넘어간다. 저장할 때 오류가 표시된다.
    }
  }

  async function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    const task = await app.addFromInput(text);
    if (task) {
      text = '';
      parsed = null;
      seq++;
    }
  }
</script>

<div class="quick-add">
  <input
    id="quick-add-input"
    type="text"
    placeholder="예: 내일 오후 3시 팀플 회의 #학교 !중요   (Ctrl+N)"
    autocomplete="off"
    bind:value={text}
    oninput={onInput}
    onkeydown={onKeydown}
  />
  {#if parsed && text.trim()}
    <div class="chips" aria-live="polite">
      <span class="chip title">{parsed.title || '제목 없음'}</span>
      {#if due}<span class="chip due">마감 {due}</span>{/if}
      {#if parsed.priority === 'high'}<span class="chip high">중요</span>{/if}
      {#if parsed.priority === 'low'}<span class="chip low">낮음</span>{/if}
      {#each parsed.tags as tag (tag)}<span class="chip tag">#{tag}</span>{/each}
    </div>
  {/if}
</div>
```

- [ ] **Step 3: `TaskRow.svelte`**

```svelte
<script lang="ts">
  import { formatDue, isOverdue, nowParts, todayIso } from '../lib/format';
  import type { Task } from '../lib/types';

  let {
    task,
    selected,
    onselect,
    ontoggle,
  }: { task: Task; selected: boolean; onselect: () => void; ontoggle: () => void } = $props();

  let el: HTMLLIElement | undefined = $state();

  const done = $derived(!!task.completed_at);
  const due = $derived(formatDue(task.due_date, task.due_time, todayIso()));
  const overdue = $derived(isOverdue(task, nowParts()));

  $effect(() => {
    if (selected) el?.scrollIntoView({ block: 'nearest' });
  });
</script>

<li class="row" class:selected class:done data-priority={task.priority} bind:this={el}>
  <input type="checkbox" checked={done} onchange={ontoggle} aria-label="완료" />
  <button class="row-body" onclick={onselect}>
    <span class="row-title">{task.title}</span>
    <span class="row-meta">
      {#each task.tags as tag (tag)}<span class="meta-tag">#{tag}</span>{/each}
      {#if due}<span class="meta-due" class:overdue>{due}</span>{/if}
    </span>
  </button>
</li>
```

- [ ] **Step 4: `TaskList.svelte`**

```svelte
<script lang="ts">
  import { app } from '../lib/state.svelte';
  import TaskRow from './TaskRow.svelte';

  const emptyText = $derived(
    app.searching
      ? '검색 결과가 없어요'
      : app.tag
        ? `#${app.tag} 태그의 할 일이 없어요`
        : { today: '오늘 할 일이 없어요', upcoming: '예정된 할 일이 없어요', overdue: '지난 일이 없어요', all: '할 일이 없어요', completed: '완료한 일이 아직 없어요' }[app.view],
  );
</script>

<section class="list" aria-label="할 일 목록">
  {#if app.mainTasks.length === 0 && app.doneTasks.length === 0}
    <p class="empty">{emptyText}</p>
  {/if}

  <ul class="rows">
    {#each app.mainTasks as task (task.id)}
      <TaskRow
        {task}
        selected={app.selectedId === task.id}
        onselect={() => (app.selectedId = task.id)}
        ontoggle={() => app.toggle(task.id)}
      />
    {/each}
  </ul>

  {#if app.view !== 'completed' || app.searching}
    {#if app.doneTasks.length > 0}
      <button class="done-toggle" onclick={() => (app.doneOpen = !app.doneOpen)}>
        {app.doneOpen ? '▾' : '▸'} 완료 {app.doneTasks.length}개
      </button>
      {#if app.doneOpen}
        <ul class="rows">
          {#each app.doneTasks as task (task.id)}
            <TaskRow
              {task}
              selected={app.selectedId === task.id}
              onselect={() => (app.selectedId = task.id)}
              ontoggle={() => app.toggle(task.id)}
            />
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</section>
```

- [ ] **Step 5: 타입 검사**

Run: `npm --prefix ui run check`
Expected: 에러 0. (`state_referenced_locally` 같은 경고가 나올 수 있으나 에러가 아니면 넘어간다. 아직 `App.svelte`에서 이 컴포넌트들을 쓰지 않으므로 "unused" 경고도 무시.)

- [ ] **Step 6: Commit**

```bash
git add ui/src/components
git commit -m "feat(ui): add sidebar, quick-add with live chips, and task list components"
```

---

### Task 8: 컴포넌트 — 상세 패널, 설정

**Files:**
- Create: `ui/src/components/DetailPanel.svelte`, `ui/src/components/Settings.svelte`

- [ ] **Step 1: `DetailPanel.svelte`**

선택이 바뀌면 부모가 `{#key}`로 새로 만들기 때문에, 초기값만 복사하면 된다. 저장은 칸을 벗어날 때(blur/change)마다 한다.

```svelte
<script lang="ts">
  import { parseTagInput, tagsToText, timeInputValue } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { Priority, Task } from '../lib/types';

  let { task }: { task: Task } = $props();

  // 선택된 할 일이 바뀌면 이 컴포넌트가 새로 만들어지므로 초기값만 복사한다.
  /* svelte-ignore state_referenced_locally */
  let title = $state(task.title);
  /* svelte-ignore state_referenced_locally */
  let dueDate = $state(task.due_date ?? '');
  /* svelte-ignore state_referenced_locally */
  let dueTime = $state(timeInputValue(task.due_time));
  /* svelte-ignore state_referenced_locally */
  let priority = $state<Priority>(task.priority);
  /* svelte-ignore state_referenced_locally */
  let tagText = $state(tagsToText(task.tags));
  /* svelte-ignore state_referenced_locally */
  let notes = $state(task.notes ?? '');

  async function save() {
    const ok = await app.save(task.id, {
      title,
      notes: notes.trim() ? notes : null,
      due_date: dueDate || null,
      due_time: dueTime || null,
      priority,
      tags: parseTagInput(tagText),
    });
    if (!ok) title = task.title; // 제목을 비웠다면 원래 제목으로 되돌린다
  }
</script>

<div class="detail-panel">
  <label class="field">
    <span>제목</span>
    <input id="detail-title" type="text" bind:value={title} onblur={save} />
  </label>

  <div class="field-row">
    <label class="field">
      <span>마감일</span>
      <input type="date" bind:value={dueDate} onchange={save} />
    </label>
    <label class="field">
      <span>시각</span>
      <input type="time" bind:value={dueTime} onchange={save} />
    </label>
  </div>

  <label class="field">
    <span>우선순위</span>
    <select bind:value={priority} onchange={save}>
      <option value="high">높음</option>
      <option value="normal">보통</option>
      <option value="low">낮음</option>
    </select>
  </label>

  <label class="field">
    <span>태그</span>
    <input type="text" placeholder="#학교 #개인" bind:value={tagText} onblur={save} />
  </label>

  <label class="field grow">
    <span>메모</span>
    <textarea bind:value={notes} onblur={save}></textarea>
  </label>

  <div class="detail-actions">
    <button onclick={() => app.toggle(task.id)}>{task.completed_at ? '완료 취소' : '완료로 표시'}</button>
    <button class="danger" onclick={() => app.remove(task.id)}>삭제</button>
  </div>
</div>
```

- [ ] **Step 2: `Settings.svelte`** — 백드롭은 겹치는 별도 버튼으로 만들어 접근성 경고를 피한다.

```svelte
<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app } from '../lib/state.svelte';

  let { onclose }: { onclose: () => void } = $props();

  async function pick() {
    const dir = await open({ directory: true, multiple: false, title: '동기화 폴더 선택' });
    if (typeof dir === 'string') await app.setSyncFolder(dir);
  }
</script>

<button class="backdrop" aria-label="닫기" onclick={onclose}></button>
<div class="dialog" role="dialog" aria-modal="true" aria-label="설정">
  <h2>동기화 설정</h2>
  <p class="hint">
    Google Drive나 OneDrive처럼 자동으로 동기화되는 폴더를 고르면, 같은 폴더를 가리키는 다른 기기와 할 일이
    맞춰져요. 고르지 않으면 이 기기에만 저장돼요.
  </p>

  <div class="folder-line">
    <code>{app.syncFolder ?? '선택된 폴더 없음'}</code>
  </div>

  <div class="dialog-actions">
    <button onclick={pick}>폴더 선택…</button>
    <button onclick={() => app.setSyncFolder(null)} disabled={!app.syncFolder}>해제</button>
    <button onclick={() => app.runSync()} disabled={!app.syncFolder || app.syncing}>
      {app.syncing ? '동기화 중…' : '지금 동기화'}
    </button>
  </div>

  {#if app.syncMessage}<p class="sync-message">{app.syncMessage}</p>{/if}

  <div class="dialog-footer">
    <button class="primary" onclick={onclose}>닫기</button>
  </div>
</div>
```

- [ ] **Step 3: 타입 검사**

Run: `npm --prefix ui run check`
Expected: 에러 0

- [ ] **Step 4: Commit**

```bash
git add ui/src/components
git commit -m "feat(ui): add detail panel and sync settings dialog"
```

---

### Task 9: `App.svelte` 조립과 다크 테마

**Files:**
- Modify: `ui/src/App.svelte`, `ui/src/app.css`

- [ ] **Step 1: `App.svelte` 교체**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import DetailPanel from './components/DetailPanel.svelte';
  import QuickAdd from './components/QuickAdd.svelte';
  import Settings from './components/Settings.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TaskList from './components/TaskList.svelte';
  import { actionForKey, targetKind } from './lib/keys';
  import { app } from './lib/state.svelte';

  let settingsOpen = $state(false);

  onMount(() => {
    void app.init();
  });

  function focusById(id: string) {
    const el = document.getElementById(id) as HTMLInputElement | null;
    el?.focus();
    el?.select?.();
  }

  function onkeydown(e: KeyboardEvent) {
    if (settingsOpen) {
      if (e.key === 'Escape') settingsOpen = false;
      return;
    }
    const action = actionForKey(
      { key: e.key, ctrlKey: e.ctrlKey || e.metaKey, isComposing: e.isComposing },
      targetKind(e.target),
    );
    if (!action) return;
    e.preventDefault();

    switch (action.type) {
      case 'focus-add':
        focusById('quick-add-input');
        break;
      case 'focus-search':
        focusById('search-input');
        break;
      case 'move':
        app.moveSelection(action.delta);
        break;
      case 'toggle':
        app.toggleSelected();
        break;
      case 'edit':
        if (app.selected) focusById('detail-title');
        break;
      case 'delete':
        app.removeSelected();
        break;
      case 'escape': {
        const active = document.activeElement as HTMLElement | null;
        if (active?.id === 'search-input' && app.search) app.clearSearch();
        active?.blur();
        break;
      }
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="app">
  <Sidebar onsettings={() => (settingsOpen = true)} />

  <main class="center">
    <header class="topbar">
      <QuickAdd />
      <input
        id="search-input"
        class="search"
        type="search"
        placeholder="검색 (Ctrl+F)"
        autocomplete="off"
        bind:value={app.search}
        oninput={() => app.onSearchInput()}
      />
    </header>
    <TaskList />
  </main>

  <aside class="detail" aria-label="상세">
    {#key app.selected?.id}
      {#if app.selected}
        <DetailPanel task={app.selected} />
      {:else}
        <p class="hint">할 일을 선택하면 자세한 내용이 여기에 보여요.</p>
      {/if}
    {/key}
  </aside>
</div>

{#if settingsOpen}
  <Settings onclose={() => (settingsOpen = false)} />
{/if}

{#if app.undo}
  <div class="toast" role="status">
    “{app.undo.title}” 완료
    <button onclick={() => app.undoComplete()}>되돌리기</button>
  </div>
{/if}
{#if app.error}
  <div class="toast error" role="alert">{app.error}</div>
{/if}
```

- [ ] **Step 2: `app.css` 교체** (다크 고정)

```css
:root {
  color-scheme: dark;
  --bg: #14161b;
  --panel: #1a1d24;
  --panel-2: #20242c;
  --line: #2a2f3a;
  --text: #e6e8ee;
  --muted: #8b93a5;
  --accent: #7c9cff;
  --accent-soft: rgba(124, 156, 255, 0.16);
  --danger: #ff7a7a;
  --warn: #ffb86b;
  --radius: 8px;
  font-family: 'Pretendard', 'Malgun Gothic', 'Apple SD Gothic Neo', system-ui, sans-serif;
  font-size: 14px;
}

* { box-sizing: border-box; }
html, body, #app { height: 100%; margin: 0; }
body { background: var(--bg); color: var(--text); overflow: hidden; }
ul { list-style: none; margin: 0; padding: 0; }
button, input, select, textarea { font: inherit; color: inherit; }
button { cursor: pointer; }
button:disabled { opacity: 0.45; cursor: default; }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }

/* 레이아웃 */
.app { display: grid; grid-template-columns: 200px minmax(0, 1fr) 320px; height: 100%; }
.sidebar { display: flex; flex-direction: column; gap: 4px; padding: 12px 8px; background: var(--panel); border-right: 1px solid var(--line); overflow-y: auto; }
.center { display: flex; flex-direction: column; min-width: 0; }
.detail { background: var(--panel); border-left: 1px solid var(--line); padding: 16px; overflow-y: auto; }
.spacer { flex: 1; }

/* 사이드바 */
.nav-list { display: flex; flex-direction: column; gap: 2px; }
.nav-item { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; padding: 7px 10px; border: 0; border-radius: var(--radius); background: transparent; text-align: left; }
.nav-item:hover { background: var(--panel-2); }
.nav-item.active { background: var(--accent-soft); color: var(--accent); }
.nav-title { margin: 14px 10px 4px; font-size: 12px; color: var(--muted); }
.count { color: var(--muted); font-size: 12px; }
.dot { display: inline-block; width: 7px; height: 7px; margin-left: 6px; border-radius: 50%; background: var(--accent); }

/* 상단: 빠른 입력 + 검색 */
.topbar { display: flex; gap: 8px; align-items: flex-start; padding: 12px 16px; border-bottom: 1px solid var(--line); }
.quick-add { flex: 1; min-width: 0; }
.quick-add input, .search { width: 100%; padding: 9px 12px; border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel-2); }
.search { width: 180px; flex: none; }
.chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
.chip { padding: 2px 8px; border-radius: 999px; background: var(--panel-2); border: 1px solid var(--line); font-size: 12px; }
.chip.title { color: var(--text); font-weight: 600; }
.chip.due { color: var(--accent); border-color: var(--accent); }
.chip.high { color: var(--danger); border-color: var(--danger); }
.chip.low { color: var(--muted); }
.chip.tag { color: var(--muted); }

/* 목록 */
.list { flex: 1; overflow-y: auto; padding: 8px 12px 24px; }
.empty { margin: 48px 0; text-align: center; color: var(--muted); }
.row { display: flex; align-items: center; gap: 10px; padding: 2px 8px 2px 6px; border-left: 3px solid transparent; border-radius: var(--radius); }
.row[data-priority='high'] { border-left-color: var(--danger); }
.row[data-priority='low'] { border-left-color: var(--line); }
.row:hover { background: var(--panel); }
.row.selected { background: var(--accent-soft); }
.row.done .row-title { color: var(--muted); text-decoration: line-through; }
.row-body { flex: 1; display: flex; align-items: center; justify-content: space-between; gap: 12px; min-width: 0; padding: 8px 0; border: 0; background: transparent; text-align: left; }
.row-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-meta { display: flex; gap: 8px; flex: none; font-size: 12px; color: var(--muted); }
.meta-due.overdue { color: var(--danger); }
.done-toggle { margin: 12px 4px 4px; padding: 4px 8px; border: 0; background: transparent; color: var(--muted); }

/* 상세 패널 */
.detail-panel { display: flex; flex-direction: column; gap: 12px; height: 100%; }
.field { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
.field > span { font-size: 12px; color: var(--muted); }
.field input, .field select, .field textarea { padding: 7px 10px; border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel-2); }
.field-row { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.field.grow { flex: 1; }
.field.grow textarea { flex: 1; min-height: 100px; resize: none; }
.detail-actions { display: flex; gap: 8px; }
.detail-actions button, .dialog-actions button, .dialog-footer button, .toast button { padding: 6px 12px; border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel-2); }
.detail-actions button:hover:not(:disabled), .dialog-actions button:hover:not(:disabled) { border-color: var(--accent); }
.danger { color: var(--danger); }
.hint { color: var(--muted); line-height: 1.5; }

/* 설정 대화상자 */
.backdrop { position: fixed; inset: 0; border: 0; background: rgba(0, 0, 0, 0.55); cursor: default; }
.dialog { position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); width: min(480px, calc(100vw - 32px)); padding: 20px; border: 1px solid var(--line); border-radius: 12px; background: var(--panel); }
.dialog h2 { margin: 0 0 8px; font-size: 16px; }
.folder-line { margin: 12px 0; padding: 8px 10px; border-radius: var(--radius); background: var(--panel-2); overflow-wrap: anywhere; }
.dialog-actions { display: flex; flex-wrap: wrap; gap: 8px; }
.sync-message { margin: 12px 0 0; color: var(--muted); font-size: 12px; }
.dialog-footer { display: flex; justify-content: flex-end; margin-top: 16px; }
.primary { background: var(--accent) !important; border-color: var(--accent) !important; color: #0b1020; }

/* 토스트 */
.toast { position: fixed; left: 50%; bottom: 20px; transform: translateX(-50%); display: flex; align-items: center; gap: 12px; padding: 10px 14px; border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel-2); box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4); }
.toast.error { border-color: var(--danger); color: var(--danger); bottom: 64px; }
```

- [ ] **Step 3: 검사와 빌드**

```bash
npm --prefix ui run check
npm --prefix ui run test
npm --prefix ui run build
```
Expected: svelte-check 에러 0, 테스트 PASS, 빌드 성공(`ui/dist` 갱신).

- [ ] **Step 4: Commit**

```bash
git add ui/src/App.svelte ui/src/app.css
git commit -m "feat(ui): assemble three-pane layout, keyboard shortcuts and dark theme"
```

---

### Task 10: 시연 데이터와 실행 확인

**Files:**
- Create: `crates/todo-core/examples/seed_demo.rs`

- [ ] **Step 1: 시연용 샘플 데이터 스크립트**

```rust
//! 사용법: cargo run -p todo-core --example seed_demo -- <db 파일 경로>
use std::env;

use todo_core::clock::Now;
use todo_core::parser::RuleBasedParser;
use todo_core::service::{TaskEdit, TodoService};
use todo_core::store::SqliteStore;
use todo_core::sync::ensure_device_id;

fn main() {
    let path = env::args().nth(1).expect("usage: seed_demo <db path>");
    let store = SqliteStore::open(&path).expect("open db");
    let device = ensure_device_id(&store).expect("device id");
    let service = TodoService::new(store, RuleBasedParser::new(), device);
    let now = Now::system();

    let samples = [
        "오늘 오후 6시 퇴근 후 운동 #건강",
        "내일 오후 3시 팀플 회의 #학교 !중요",
        "모레 오전 10시 치과 예약 확인 !!",
        "이번 주 금요일 발표 자료 정리 #학교",
        "다음 주 월요일 장보기 #집 !낮음",
        "책 읽기 #개인",
    ];
    for sample in samples {
        service.add_from_input(sample, now).expect("add sample");
    }

    // 파서는 지난 날짜를 내년으로 넘기므로, "지난 일"은 만든 뒤 마감일을 어제로 고쳐서 시연한다.
    let late = service
        .add_from_input("보고서 제출 #회사 !중요", now)
        .expect("add overdue sample");
    service
        .edit(
            late.id,
            TaskEdit {
                title: late.title.clone(),
                notes: None,
                due_date: Some(now.local.date() - chrono::Duration::days(1)),
                due_time: None,
                priority: late.priority,
                tags: late.tags.clone(),
            },
            now,
        )
        .expect("backdate sample");

    println!("seeded {} tasks into {path}", samples.len() + 1);
}
```

- [ ] **Step 2: 샘플 DB 만들기**

```bash
mkdir -p "$TEMP/todo-lite-demo"
cargo run -p todo-core --example seed_demo -- "$TEMP/todo-lite-demo/todo.db"
```
Expected: `seeded 7 tasks into ...` (일곱 번째가 어제 마감의 "지난 일"이다)

- [ ] **Step 3: 앱 실행 (개발 모드 빌드)**

PowerShell:

```powershell
$env:TODO_LITE_DATA_DIR = "$env:TEMP\todo-lite-demo"
$env:TODO_LITE_TIMING = "1"
npm run dev
```
Expected: 창이 뜨고 사이드바(오늘/예정/지난 일/전체/완료됨, 태그 목록)와 샘플 할 일이 보인다. 터미널에 `ready_ms=...` 가 찍힌다. 수동 확인 항목:

- [ ] 빠른 입력창에 `내일 오후 3시 회의 #학교`를 입력하면 칩(제목·마감 내일 15:00·#학교)이 실시간으로 보이고, Enter로 저장된다
- [ ] 한글 입력 중 Enter(조합 확정)에는 저장되지 않는다
- [ ] `Ctrl+N` 입력창, `Ctrl+F` 검색창 포커스 / `↑ ↓` 선택 이동 / `Space` 완료 토글 / `Enter` 제목 편집 / `Del` 삭제(확인창) / `Esc` 포커스 해제
- [ ] 완료하면 "되돌리기" 토스트가 뜨고 누르면 복구된다. `전체` 보기 아래에 접힌 "완료 N개"가 보인다
- [ ] 상세 패널에서 제목·마감일·시각·우선순위·태그·메모를 고치면 목록에 반영된다
- [ ] 태그를 누르면 필터되고 한 번 더 누르면 풀린다. 검색은 제목·메모·태그에서 찾는다
- [ ] 설정에서 폴더를 고르면 "동기화" 문구가 갱신되고 해당 폴더에 `<기기ID>.jsonl`이 생긴다

수동 확인이 불가능한 환경(에이전트)에서는 Task 11의 스크린샷 확인으로 갈음하고, 위 목록은 사용자에게 넘긴다.

- [ ] **Step 4: Commit**

```bash
git add crates/todo-core/examples
git commit -m "chore(core): add seed_demo example for trying the UI with sample tasks"
```

---

### Task 11: 릴리스 빌드와 PRD 목표 측정

**Files:**
- Create: `scripts/measure.ps1`

- [ ] **Step 1: 측정 스크립트 `scripts/measure.ps1`**

```powershell
param([int]$IdleSeconds = 20)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\todo-lite.exe'
$installer = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis\*.exe') |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

'--- 설치 파일 ---'
'{0}: {1:N1} MB (목표 < 50 MB)' -f $installer.Name, ($installer.Length / 1MB)

$tmp = Join-Path $env:TEMP 'todo-lite-measure'
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $tmp | Out-Null
$env:TODO_LITE_DATA_DIR = $tmp
$env:TODO_LITE_TIMING = '1'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $tmp 'webview'   # 매번 콜드 프로필
$errFile = Join-Path $tmp 'stderr.txt'

$p = Start-Process $exe -PassThru -RedirectStandardError $errFile
Start-Sleep -Seconds $IdleSeconds

# 앱과 그 자식(WebView2) 프로세스를 모은다
$all = Get-CimInstance Win32_Process
$ids = New-Object System.Collections.Generic.HashSet[int]
[void]$ids.Add($p.Id)
do {
    $before = $ids.Count
    foreach ($proc in $all) { if ($ids.Contains([int]$proc.ParentProcessId)) { [void]$ids.Add([int]$proc.ProcessId) } }
} while ($ids.Count -gt $before)

$procs = Get-Process -Id ($ids | ForEach-Object { $_ }) -ErrorAction SilentlyContinue
$ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum
$priv = ($procs | Measure-Object PrivateMemorySize64 -Sum).Sum

'--- 유휴 메모리 ({0}초 후, 프로세스 {1}개, WebView2 포함) ---' -f $IdleSeconds, @($procs).Count
'작업 집합 합계: {0:N0} MB   (공유 페이지가 중복 계산됨)' -f ($ws / 1MB)
'프라이빗 메모리 합계: {0:N0} MB   (목표 약 100 MB 이하)' -f ($priv / 1MB)

'--- 시작 시간 ---'
$ready = Select-String -Path $errFile -Pattern 'ready_ms=(\d+)' | Select-Object -First 1
if ($ready) { 'ready_ms = {0} (목표 < 1000)' -f $ready.Matches[0].Groups[1].Value } else { 'ready_ms 출력을 찾지 못함 (stderr: ' + $errFile + ')' }

Stop-Process -Id $p.Id -Force
Get-Process msedgewebview2 -ErrorAction SilentlyContinue |
    Where-Object { $ids.Contains($_.Id) } | Stop-Process -Force -ErrorAction SilentlyContinue
```

- [ ] **Step 2: 릴리스 번들 빌드**

Run: `npm run build`
Expected: `target/release/bundle/nsis/Todo-Lite_0.1.0_x64-setup.exe` 생성. (첫 릴리스 빌드와 NSIS 도구 다운로드로 10분 이상 걸릴 수 있다. 타임아웃을 넉넉히 잡는다.)

- [ ] **Step 3: 측정**

Run (PowerShell): `powershell -ExecutionPolicy Bypass -File scripts/measure.ps1`

세 수치를 기록한다: 설치 파일 크기(MB), 프라이빗 메모리 합계(MB)와 작업 집합 합계(MB), `ready_ms`.

**판정 규칙:**
- 설치 파일 < 50MB, `ready_ms` < 1000 이면 목표 달성.
- 메모리가 약 100MB를 넘으면 **조용히 코드를 바꾸지 않는다.** 수치를 그대로 보고하고 사용자가 목표를 조정할지 정하게 한다. (WebView2는 여러 프로세스라 합산이 100MB를 넘기기 쉽다.)

- [ ] **Step 4: 화면 확인 (스크린샷)**

샘플 데이터 DB를 쓰는 릴리스 실행 파일을 띄우고 창을 캡처해 눈으로 확인한다.

```powershell
$env:TODO_LITE_DATA_DIR = "$env:TEMP\todo-lite-demo"
$p = Start-Process "target\release\todo-lite.exe" -PassThru
Start-Sleep -Seconds 4
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
[System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save("$env:TEMP\todo-lite-shot.png")
Stop-Process -Id $p.Id -Force
```

Read 도구로 `$env:TEMP\todo-lite-shot.png`를 열어 다음을 확인한다: 3단 레이아웃, 다크 테마, 한글 글자 깨짐 없음, 칩·마감일 색, 샘플 할 일 표시. 문제가 있으면 CSS/컴포넌트를 고친다.

- [ ] **Step 5: Commit**

```bash
git add scripts/measure.ps1
git commit -m "chore: add release measurement script for size, memory and startup"
```

---

### Task 12: 최종 검증과 문서

**Files:**
- Modify: `docs/PRD.md` (측정 결과 한 줄)

- [ ] **Step 1: 전체 검증**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release
npm --prefix ui run check
npm --prefix ui run test
```
Expected: 전부 통과. (Rust 테스트: 코어 72 + `todo-lite` 8, perf 1 포함 / 프론트 Vitest: format·keys 전부)

- [ ] **Step 2: PRD에 결과 기록**

`docs/PRD.md` 맨 끝에 다음 절을 추가한다. 값은 Task 11의 실측으로 채운다. 목표를 못 맞춘 항목은 숨기지 말고 그대로 적는다.

```markdown
## 10. 구현 상태 (v1 데스크톱)

- 화면: 사이드바·목록·상세 3단, 다크 고정. 키보드: Ctrl+N / Ctrl+F / ↑↓ / Space / Enter / Del / Esc
- 측정 (Windows, 릴리스 빌드):

| 항목 | 목표 | 실측 |
|---|---|---|
| 설치 파일 | < 50 MB | <측정값> MB |
| 유휴 메모리 (WebView2 포함, 프라이빗 합계) | 약 100 MB 이하 | <측정값> MB |
| 시작 → 첫 화면 | < 1 s | <측정값> ms |
| 1만 건 목록·검색 | < 100 ms | 약 44 ms (코어 perf 테스트) |
```

- [ ] **Step 3: Commit**

```bash
git add docs/PRD.md
git commit -m "docs: record v1 desktop measurements in PRD"
```

---

## Self-Review

**1. Spec 대응**

| 요구 | 위치 |
|---|---|
| F1 기본 관리·완료 접힘 섹션 | Task 6 `toggle/remove`, Task 7 `TaskList` |
| F2 자연어 입력 + 실시간 칩 | Task 7 `QuickAdd` (IME 가드 포함) |
| F3 날짜 선택기 수동 편집 | Task 8 `DetailPanel` |
| F4 우선순위(표시) | Task 7 `TaskRow` 막대, Task 8 선택 |
| F5 태그 사이드바·필터 | Task 6 `setTag`, Task 7 `Sidebar` |
| F6 검색 Ctrl+F | Task 6 `onSearchInput`, Task 9 헤더 |
| F7 보기 5종 | Task 6 `VIEWS`, Task 7 `Sidebar` |
| F8 메모 | Task 8 `DetailPanel` |
| F9 동기화 설정·시점 | Task 2 `sync_now`, Task 6 `runSync/scheduleSync/init`, Task 8 `Settings` |
| F10 키보드 | Task 5 `keys.ts`(테스트), Task 9 핸들러 |
| 크기/메모리/시작 시간 | Task 11 |
| Android 대비 | Task 3 `mobile_entry_point`, 코어/Api 분리 |

**2. Placeholder 스캔:** TBD/TODO 없음. 모든 코드 단계에 실제 코드가 있다. PRD 표의 `<측정값>`만 Task 11 실측값으로 채우는 자리이며 의도된 것이다.

**3. 타입 일관성:** `ListQuery{view,tag,search}`, `SyncStatus{configured,exported,applied,corrupt_lines}`, `ParsedTask`/`ParsedDto`, `TaskEdit` 필드(snake_case)가 Rust·TS·테스트에서 같다. `targetKind`는 `'text'|'control'|'none'`, `actionForKey(e, target)` 시그니처가 테스트·`App.svelte`에서 같다. `app.removeSelected()`·`app.toggleSelected()`·`app.moveSelection()`·`app.clearSearch()`·`app.onSearchInput()`·`app.searching`은 Task 6에서 정의하고 Task 7–9에서 쓴다.

**알려진 한계 (의도적으로 범위 밖)**
- 컴포넌트 단위 테스트 없음(jsdom 미사용). 화면은 svelte-check·빌드·실행 확인으로 검증한다.
- 라이트 테마, 다국어, 드래그 정렬, 알림, 반복 일정(v2)은 없다.
- Android 빌드와 폴더 연동(v3)은 이번 범위가 아니다.
- 동기화 시점은 프론트엔드가 관리하므로 창이 닫혀 있으면 동기화하지 않는다.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::Result;
use crate::model::Task;
use crate::store::{fmt_dt, TaskStore};

const KEY_DEVICE_ID: &str = "device_id";
const KEY_LAST_EXPORT: &str = "sync_last_export";

fn offset_key(file_stem: &str) -> String {
    format!("sync_offset:{file_stem}")
}

/// 이 기기의 고유 ID를 돌려준다. 처음이면 만들어서 저장한다.
pub fn ensure_device_id<S: TaskStore>(store: &S) -> Result<String> {
    if let Some(id) = store.meta_get(KEY_DEVICE_ID)? {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    store.meta_set(KEY_DEVICE_ID, &id)?;
    Ok(id)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// 로컬에 반영된 변경 수
    pub applied: usize,
    /// 이미 로컬이 같거나 더 새로워서 버린 변경 수
    pub skipped: usize,
    /// JSON으로 읽을 수 없어 건너뛴 줄 수
    pub corrupt_lines: usize,
}

/// 기기별 append-only 변경 로그(`<dir>/<device_id>.jsonl`)를 주고받는다.
pub struct SyncEngine {
    device_id: String,
    dir: PathBuf,
}

impl SyncEngine {
    pub fn new(device_id: impl Into<String>, dir: impl AsRef<Path>) -> Self {
        SyncEngine {
            device_id: device_id.into(),
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn own_file(&self) -> PathBuf {
        self.dir.join(format!("{}.jsonl", self.device_id))
    }

    /// 마지막 내보내기 이후 **이 기기가** 바꾼 할 일을 내 로그에 덧붙인다. 쓴 줄 수를 돌려준다.
    pub fn export<S: TaskStore>(&self, store: &S) -> Result<usize> {
        let watermark: Option<DateTime<Utc>> = store
            .meta_get(KEY_LAST_EXPORT)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc));

        let mut pending: Vec<Task> = store
            .all_including_deleted()?
            .into_iter()
            .filter(|t| t.updated_by == self.device_id)
            .filter(|t| watermark.is_none_or(|w| t.updated_at > w))
            .collect();
        if pending.is_empty() {
            return Ok(0);
        }
        pending.sort_by_key(|t| t.updated_at);

        fs::create_dir_all(&self.dir)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.own_file())?;
        for task in &pending {
            writeln!(file, "{}", serde_json::to_string(task)?)?;
        }
        file.flush()?;

        let newest = pending.last().expect("non-empty").updated_at;
        store.meta_set(KEY_LAST_EXPORT, &fmt_dt(&newest))?;
        Ok(pending.len())
    }

    /// 다른 기기들의 로그에서 아직 읽지 않은 부분을 로컬에 병합한다.
    pub fn import<S: TaskStore>(&self, store: &S) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        if !self.dir.is_dir() {
            return Ok(report);
        }

        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == self.device_id {
                continue;
            }

            let key = offset_key(stem);
            let mut file = File::open(&path)?;
            let mut offset: u64 = store
                .meta_get(&key)?
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if offset > file.metadata()?.len() {
                // 파일이 줄었다면 처음부터 다시 읽는다. 병합은 멱등이라 안전하다.
                offset = 0;
            }
            file.seek(SeekFrom::Start(offset))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            // 개행으로 끝난 완전한 줄까지만 소비한다. 업로드 중인 잘린 줄은 다음에 읽는다.
            let consumed = buf
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |i| i + 1);

            for line in buf[..consumed].split(|&b| b == b'\n') {
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                match serde_json::from_slice::<Task>(line) {
                    Ok(task) => {
                        if merge(store, task)? {
                            report.applied += 1;
                        } else {
                            report.skipped += 1;
                        }
                    }
                    Err(err) => {
                        eprintln!("sync: skipping corrupt line in {}: {err}", path.display());
                        report.corrupt_lines += 1;
                    }
                }
            }
            store.meta_set(&key, &(offset + consumed as u64).to_string())?;
        }
        Ok(report)
    }
}

/// Last-Write-Wins. `(updated_at, updated_by)`가 더 큰 쪽이 이긴다. 반영했으면 true.
fn merge<S: TaskStore>(store: &S, remote: Task) -> Result<bool> {
    if let Some(local) = store.get(remote.id)? {
        if (local.updated_at, &local.updated_by) >= (remote.updated_at, &remote.updated_by) {
            return Ok(false);
        }
    }
    store.upsert(&remote)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Now;
    use crate::model::Task;
    use crate::parser::RuleBasedParser;
    use crate::query::{Filter, View};
    use crate::service::{TaskEdit, TodoService};
    use crate::store::SqliteStore;
    use chrono::NaiveDate;

    type Svc = TodoService<SqliteStore, RuleBasedParser>;

    fn svc(device: &str) -> Svc {
        TodoService::new(
            SqliteStore::open_in_memory().unwrap(),
            RuleBasedParser::new(),
            device.to_string(),
        )
    }

    fn at(hour: u32) -> Now {
        Now::fixed(
            NaiveDate::from_ymd_opt(2026, 10, 28)
                .unwrap()
                .and_hms_opt(hour, 0, 0)
                .unwrap(),
        )
    }

    fn all(svc: &Svc) -> Vec<Task> {
        svc.list(&Filter::view(View::All), at(23)).unwrap()
    }

    fn rename(svc: &Svc, id: uuid::Uuid, title: &str, now: Now) {
        svc.edit(
            id,
            TaskEdit {
                title: title.into(),
                notes: None,
                due_date: None,
                due_time: None,
                priority: Default::default(),
                tags: vec![],
            },
            now,
        )
        .unwrap();
    }

    #[test]
    fn task_added_on_one_device_appears_on_the_other() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        a.add_from_input("내일 회의 #학교", at(9)).unwrap();
        assert_eq!(ea.export(a.store()).unwrap(), 1);

        let report = eb.import(b.store()).unwrap();
        assert_eq!(report, ImportReport { applied: 1, skipped: 0, corrupt_lines: 0 });
        let got = all(&b);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "회의");
        assert_eq!(got[0].tags, ["학교"]);
    }

    #[test]
    fn export_is_incremental_and_import_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        a.add_from_input("첫째", at(9)).unwrap();
        assert_eq!(ea.export(a.store()).unwrap(), 1);
        assert_eq!(ea.export(a.store()).unwrap(), 0); // 바뀐 게 없으면 아무것도 쓰지 않는다

        assert_eq!(eb.import(b.store()).unwrap().applied, 1);
        assert_eq!(eb.import(b.store()).unwrap().applied, 0); // 오프셋이 전진했으므로

        a.add_from_input("둘째", at(10)).unwrap();
        assert_eq!(ea.export(a.store()).unwrap(), 1);
        assert_eq!(eb.import(b.store()).unwrap().applied, 1);
        assert_eq!(all(&b).len(), 2);
    }

    #[test]
    fn last_write_wins_and_both_devices_converge() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        let task = a.add_from_input("원본", at(9)).unwrap();
        ea.export(a.store()).unwrap();
        eb.import(b.store()).unwrap();

        rename(&a, task.id, "pc가 10시에 수정", at(10));
        rename(&b, task.id, "phone이 11시에 수정", at(11));
        ea.export(a.store()).unwrap();
        eb.export(b.store()).unwrap();
        ea.import(a.store()).unwrap();
        eb.import(b.store()).unwrap();

        assert_eq!(all(&a)[0].title, "phone이 11시에 수정");
        assert_eq!(all(&b)[0].title, "phone이 11시에 수정");
    }

    #[test]
    fn identical_timestamps_are_broken_by_device_id_on_both_sides() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        let task = a.add_from_input("원본", at(9)).unwrap();
        ea.export(a.store()).unwrap();
        eb.import(b.store()).unwrap();

        rename(&a, task.id, "from pc", at(10));
        rename(&b, task.id, "from phone", at(10)); // 같은 시각
        ea.export(a.store()).unwrap();
        eb.export(b.store()).unwrap();
        ea.import(a.store()).unwrap();
        eb.import(b.store()).unwrap();

        // "phone" > "pc" 이므로 phone의 수정이 양쪽에서 이긴다.
        assert_eq!(all(&a)[0].title, "from phone");
        assert_eq!(all(&b)[0].title, "from phone");
    }

    #[test]
    fn deletion_propagates() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        let task = a.add_from_input("지울 것", at(9)).unwrap();
        ea.export(a.store()).unwrap();
        eb.import(b.store()).unwrap();
        assert_eq!(all(&b).len(), 1);

        a.delete(task.id, at(10)).unwrap();
        ea.export(a.store()).unwrap();
        eb.import(b.store()).unwrap();
        assert!(all(&b).is_empty());
    }

    #[test]
    fn imported_tasks_are_not_echoed_back_on_export() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (svc("pc"), svc("phone"));
        let (ea, eb) = (SyncEngine::new("pc", dir.path()), SyncEngine::new("phone", dir.path()));

        a.add_from_input("pc에서 만듦", at(9)).unwrap();
        ea.export(a.store()).unwrap();
        eb.import(b.store()).unwrap();
        assert_eq!(eb.export(b.store()).unwrap(), 0);
    }

    #[test]
    fn corrupt_lines_are_skipped_and_counted() {
        let src = tempfile::tempdir().unwrap();
        let a = svc("pc");
        a.add_from_input("정상", at(9)).unwrap();
        SyncEngine::new("pc", src.path()).export(a.store()).unwrap();
        let good = std::fs::read_to_string(src.path().join("pc.jsonl")).unwrap();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("other.jsonl"),
            format!("not json at all\n{good}"),
        )
        .unwrap();

        let b = svc("phone");
        let report = SyncEngine::new("phone", dir.path()).import(b.store()).unwrap();
        assert_eq!(report, ImportReport { applied: 1, skipped: 0, corrupt_lines: 1 });
        assert_eq!(all(&b).len(), 1);
    }

    #[test]
    fn partial_last_line_is_deferred_until_complete() {
        let src = tempfile::tempdir().unwrap();
        let a = svc("pc");
        a.add_from_input("정상", at(9)).unwrap();
        SyncEngine::new("pc", src.path()).export(a.store()).unwrap();
        let good = std::fs::read_to_string(src.path().join("pc.jsonl")).unwrap();
        let (head, tail) = good.trim_end().split_at(good.len() / 2);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("other.jsonl");
        std::fs::write(&path, head).unwrap(); // 개행 없이 잘린 상태

        let b = svc("phone");
        let eb = SyncEngine::new("phone", dir.path());
        assert_eq!(eb.import(b.store()).unwrap(), ImportReport::default());

        std::fs::write(&path, format!("{head}{tail}\n")).unwrap(); // 업로드 완료
        assert_eq!(eb.import(b.store()).unwrap().applied, 1);
    }

    #[test]
    fn import_from_missing_folder_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let b = svc("phone");
        let eb = SyncEngine::new("phone", dir.path().join("does-not-exist"));
        assert_eq!(eb.import(b.store()).unwrap(), ImportReport::default());
    }

    #[test]
    fn device_id_is_generated_once_and_persisted() {
        let store = SqliteStore::open_in_memory().unwrap();
        let first = ensure_device_id(&store).unwrap();
        assert!(!first.is_empty());
        assert_eq!(ensure_device_id(&store).unwrap(), first);
    }
}

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::clock::Now;
use crate::error::{Error, Result};
use crate::model::{normalize_tags, Priority, Task};
use crate::parser::{ParsedTask, TaskParser};
use crate::query::{all_tags, select, Filter, TagCount};
use crate::store::TaskStore;

/// 편집 폼이 한 번에 보내는 값. 모든 필드를 통째로 교체한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEdit {
    pub title: String,
    pub notes: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

/// UI가 호출하는 진입점. 저장소와 파서를 묶는다.
pub struct TodoService<S: TaskStore, P: TaskParser> {
    store: S,
    parser: P,
    device_id: String,
}

impl<S: TaskStore, P: TaskParser> TodoService<S, P> {
    pub fn new(store: S, parser: P, device_id: String) -> Self {
        TodoService {
            store,
            parser,
            device_id,
        }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    /// 입력창 미리보기용. 저장하지 않는다.
    pub fn preview(&self, input: &str, now: Now) -> ParsedTask {
        self.parser.parse(input, now.local)
    }

    pub fn add_from_input(&self, input: &str, now: Now) -> Result<Task> {
        let parsed = self.parser.parse(input, now.local);
        if parsed.title.is_empty() {
            return Err(Error::EmptyTitle);
        }
        let mut task = Task::new(parsed.title, now.utc, &self.device_id);
        task.due_date = parsed.due_date;
        task.due_time = parsed.due_time;
        task.priority = parsed.priority;
        task.tags = normalize_tags(parsed.tags);
        self.store.upsert(&task)?;
        Ok(task)
    }

    pub fn edit(&self, id: Uuid, edit: TaskEdit, now: Now) -> Result<Task> {
        let title = edit.title.trim();
        if title.is_empty() {
            return Err(Error::EmptyTitle);
        }
        let mut task = self.load(id)?;
        task.title = title.to_string();
        task.notes = edit
            .notes
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        task.due_date = edit.due_date;
        task.due_time = edit.due_time;
        task.priority = edit.priority;
        task.tags = normalize_tags(edit.tags);
        task.touch(now.utc, &self.device_id);
        self.store.upsert(&task)?;
        Ok(task)
    }

    pub fn set_completed(&self, id: Uuid, done: bool, now: Now) -> Result<Task> {
        let mut task = self.load(id)?;
        task.completed_at = done.then_some(now.utc);
        task.touch(now.utc, &self.device_id);
        self.store.upsert(&task)?;
        Ok(task)
    }

    /// 삭제 표시만 한다. 동기화로 다른 기기에 전파되어야 하기 때문이다.
    pub fn delete(&self, id: Uuid, now: Now) -> Result<()> {
        let mut task = self.load(id)?;
        task.deleted = true;
        task.touch(now.utc, &self.device_id);
        self.store.upsert(&task)
    }

    pub fn list(&self, filter: &Filter, now: Now) -> Result<Vec<Task>> {
        Ok(select(&self.store.all()?, filter, now.local))
    }

    pub fn tags(&self) -> Result<Vec<TagCount>> {
        Ok(all_tags(&self.store.all()?))
    }

    fn load(&self, id: Uuid) -> Result<Task> {
        self.store
            .get(id)?
            .filter(|t| !t.deleted)
            .ok_or(Error::NotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RuleBasedParser;
    use crate::query::View;
    use crate::store::SqliteStore;
    use chrono::NaiveDate;

    type Svc = TodoService<SqliteStore, RuleBasedParser>;

    fn svc() -> Svc {
        TodoService::new(
            SqliteStore::open_in_memory().unwrap(),
            RuleBasedParser::new(),
            "pc".to_string(),
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
        svc.list(&Filter::view(View::All), at(15)).unwrap()
    }

    #[test]
    fn add_from_input_parses_and_persists() {
        let svc = svc();
        let task = svc
            .add_from_input("내일 오후 3시 팀플 회의 #학교 !중요", at(15))
            .unwrap();
        assert_eq!(task.title, "팀플 회의");
        assert_eq!(task.due_date, NaiveDate::from_ymd_opt(2026, 10, 29));
        assert_eq!(task.due_time, NaiveTime::from_hms_opt(15, 0, 0));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags, ["학교"]);
        assert_eq!(task.updated_by, "pc");
        assert_eq!(svc.store().get(task.id).unwrap(), Some(task));
    }

    #[test]
    fn add_rejects_input_without_a_title_and_stores_nothing() {
        let svc = svc();
        assert!(matches!(
            svc.add_from_input("내일", at(15)),
            Err(Error::EmptyTitle)
        ));
        assert!(all(&svc).is_empty());
    }

    #[test]
    fn preview_parses_without_persisting() {
        let svc = svc();
        let parsed = svc.preview("모레 치과 #건강", at(15));
        assert_eq!(parsed.title, "치과");
        assert!(all(&svc).is_empty());
    }

    #[test]
    fn set_completed_toggles_and_touches() {
        let svc = svc();
        let task = svc.add_from_input("장보기", at(9)).unwrap();

        let done = svc.set_completed(task.id, true, at(10)).unwrap();
        assert_eq!(done.completed_at, Some(at(10).utc));
        assert_eq!(done.updated_at, at(10).utc);

        let undone = svc.set_completed(task.id, false, at(11)).unwrap();
        assert_eq!(undone.completed_at, None);
        assert_eq!(undone.updated_at, at(11).utc);
    }

    #[test]
    fn delete_is_soft_and_hides_the_task() {
        let svc = svc();
        let task = svc.add_from_input("장보기", at(9)).unwrap();
        svc.delete(task.id, at(10)).unwrap();

        assert!(all(&svc).is_empty());
        let stored = svc.store().get(task.id).unwrap().unwrap();
        assert!(stored.deleted);
        assert_eq!(stored.updated_at, at(10).utc);
        assert!(matches!(
            svc.set_completed(task.id, true, at(11)),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn edit_replaces_fields_and_validates_title() {
        let svc = svc();
        let task = svc.add_from_input("장보기", at(9)).unwrap();

        let edited = svc
            .edit(
                task.id,
                TaskEdit {
                    title: "  마트 가기 ".into(),
                    notes: Some("우유, 계란".into()),
                    due_date: NaiveDate::from_ymd_opt(2026, 11, 1),
                    due_time: None,
                    priority: Priority::Low,
                    tags: vec!["#집".into(), "집".into(), "마트".into()],
                },
                at(10),
            )
            .unwrap();
        assert_eq!(edited.title, "마트 가기");
        assert_eq!(edited.notes.as_deref(), Some("우유, 계란"));
        assert_eq!(edited.tags, ["집", "마트"]);
        assert_eq!(edited.updated_at, at(10).utc);

        let blank = TaskEdit {
            title: "   ".into(),
            notes: None,
            due_date: None,
            due_time: None,
            priority: Priority::Normal,
            tags: vec![],
        };
        assert!(matches!(
            svc.edit(task.id, blank, at(11)),
            Err(Error::EmptyTitle)
        ));
    }

    #[test]
    fn tags_reflect_open_tasks() {
        let svc = svc();
        svc.add_from_input("a #학교", at(9)).unwrap();
        svc.add_from_input("b #학교 #개인", at(9)).unwrap();
        let names: Vec<_> = svc
            .tags()
            .unwrap()
            .into_iter()
            .map(|t| (t.name, t.count))
            .collect();
        assert_eq!(names, [("학교".to_string(), 2), ("개인".to_string(), 1)]);
    }
}

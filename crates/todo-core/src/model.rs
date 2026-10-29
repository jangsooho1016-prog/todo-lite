use std::collections::HashSet;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    #[default]
    Normal,
    Low,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown priority: {0}")]
pub struct ParsePriorityError(String);

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::High => "high",
            Priority::Normal => "normal",
            Priority::Low => "low",
        }
    }

    /// 정렬 키. 값이 작을수록 먼저 나온다.
    pub fn rank(self) -> u8 {
        match self {
            Priority::High => 0,
            Priority::Normal => 1,
            Priority::Low => 2,
        }
    }
}

impl FromStr for Priority {
    type Err = ParsePriorityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "high" => Ok(Priority::High),
            "normal" => Ok(Priority::Normal),
            "low" => Ok(Priority::Low),
            other => Err(ParsePriorityError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 마지막으로 이 할 일을 수정한 기기의 ID.
    pub updated_by: String,
    pub deleted: bool,
}

impl Task {
    pub fn new(title: impl Into<String>, now: DateTime<Utc>, device_id: &str) -> Self {
        Task {
            id: Uuid::now_v7(),
            title: title.into(),
            notes: None,
            due_date: None,
            due_time: None,
            priority: Priority::Normal,
            tags: Vec::new(),
            completed_at: None,
            created_at: now,
            updated_at: now,
            updated_by: device_id.to_string(),
            deleted: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn touch(&mut self, now: DateTime<Utc>, device_id: &str) {
        // 시계가 어긋난 기기끼리도 수정이 항상 이전 버전보다 새롭게 보이도록 단조 증가시킨다.
        let floor = self.updated_at + chrono::Duration::microseconds(1);
        self.updated_at = now.max(floor);
        self.updated_by = device_id.to_string();
    }
}

/// 앞뒤 공백과 `#`을 제거하고, 빈 태그를 버리고, 대소문자 무시로 중복을 제거한다.
/// 처음 나온 표기를 유지한다.
pub fn normalize_tags<I, S>(tags: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tag in tags {
        let tag = tag.as_ref().trim().trim_start_matches('#').trim();
        if tag.is_empty() {
            continue;
        }
        if seen.insert(tag.to_lowercase()) {
            out.push(tag.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 6, 0, 0).unwrap()
    }

    #[test]
    fn priority_string_roundtrip() {
        for p in [Priority::High, Priority::Normal, Priority::Low] {
            assert_eq!(p.as_str().parse::<Priority>().unwrap(), p);
        }
        assert!("urgent".parse::<Priority>().is_err());
    }

    #[test]
    fn priority_rank_orders_high_first() {
        assert!(Priority::High.rank() < Priority::Normal.rank());
        assert!(Priority::Normal.rank() < Priority::Low.rank());
    }

    #[test]
    fn new_task_has_defaults() {
        let t = Task::new("장보기", t0(), "pc");
        assert_eq!(t.title, "장보기");
        assert_eq!(t.priority, Priority::Normal);
        assert!(t.tags.is_empty());
        assert!(!t.deleted);
        assert!(!t.is_done());
        assert_eq!(t.created_at, t0());
        assert_eq!(t.updated_at, t0());
        assert_eq!(t.updated_by, "pc");
    }

    #[test]
    fn touch_updates_time_and_device() {
        let mut t = Task::new("a", t0(), "pc");
        let later = t0() + chrono::Duration::minutes(5);
        t.touch(later, "phone");
        assert_eq!(t.updated_at, later);
        assert_eq!(t.updated_by, "phone");
        assert_eq!(t.created_at, t0());
    }

    #[test]
    fn touch_is_monotonic_even_if_clock_went_backwards() {
        let mut t = Task::new("a", t0(), "pc");
        t.touch(t0() - chrono::Duration::minutes(1), "pc");
        assert_eq!(t.updated_at, t0() + chrono::Duration::microseconds(1));
        let later = t0() + chrono::Duration::minutes(5);
        t.touch(later, "pc");
        assert_eq!(t.updated_at, later);
    }

    #[test]
    fn task_json_roundtrip() {
        let mut t = Task::new("a", t0(), "pc");
        t.tags = vec!["학교".into()];
        t.due_date = NaiveDate::from_ymd_opt(2026, 10, 30);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(serde_json::from_str::<Task>(&json).unwrap(), t);
    }

    #[test]
    fn normalize_tags_strips_hash_trims_and_dedupes() {
        let got = normalize_tags(["#학교", " 학교 ", "Work", "work", "", "#"]);
        assert_eq!(got, vec!["학교".to_string(), "Work".to_string()]);
    }
}

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Serialize;

use crate::model::{Priority, Task};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// 미완료이면서 마감일이 오늘이거나 지난 것
    Today,
    /// 미완료이면서 마감일이 내일 이후
    Upcoming,
    /// 마감 시각이 지난 미완료
    Overdue,
    /// 삭제되지 않은 전부 (완료 포함)
    All,
    Completed,
}

#[derive(Debug, Clone, Copy)]
pub struct Filter<'a> {
    pub view: View,
    pub tag: Option<&'a str>,
    pub search: Option<&'a str>,
}

impl<'a> Filter<'a> {
    pub fn view(view: View) -> Self {
        Filter {
            view,
            tag: None,
            search: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: usize,
}

pub fn select(tasks: &[Task], filter: &Filter, now: NaiveDateTime) -> Vec<Task> {
    let needle = filter
        .search
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    let tag = filter.tag.map(|t| t.to_lowercase());

    let mut out: Vec<Task> = tasks
        .iter()
        .filter(|t| !t.deleted)
        .filter(|t| in_view(t, filter.view, now))
        .filter(|t| {
            tag.as_ref()
                .is_none_or(|tag| t.tags.iter().any(|x| x.to_lowercase() == *tag))
        })
        .filter(|t| needle.as_ref().is_none_or(|n| matches_search(t, n)))
        .cloned()
        .collect();

    if filter.view == View::Completed {
        out.sort_by_key(|a| std::cmp::Reverse(a.completed_at));
    } else {
        out.sort_by_key(sort_key);
    }
    out
}

/// 사이드바/자동완성용 태그 목록. 미완료 할 일 기준, 개수 내림차순 → 이름순.
pub fn all_tags(tasks: &[Task]) -> Vec<TagCount> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for t in tasks.iter().filter(|t| !t.deleted && !t.is_done()) {
        for tag in &t.tags {
            *counts.entry(tag.as_str()).or_default() += 1;
        }
    }
    let mut out: Vec<TagCount> = counts
        .into_iter()
        .map(|(name, count)| TagCount {
            name: name.to_string(),
            count,
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    out
}

fn in_view(t: &Task, view: View, now: NaiveDateTime) -> bool {
    let today = now.date();
    match view {
        View::All => true,
        View::Completed => t.is_done(),
        View::Today => !t.is_done() && t.due_date.is_some_and(|d| d <= today),
        View::Upcoming => !t.is_done() && t.due_date.is_some_and(|d| d > today),
        View::Overdue => !t.is_done() && is_overdue(t, now),
    }
}

fn is_overdue(t: &Task, now: NaiveDateTime) -> bool {
    match t.due_date {
        None => false,
        Some(d) => {
            d < now.date() || (d == now.date() && t.due_time.is_some_and(|tm| tm < now.time()))
        }
    }
}

fn matches_search(t: &Task, needle: &str) -> bool {
    t.title.to_lowercase().contains(needle)
        || t.notes
            .as_ref()
            .is_some_and(|n| n.to_lowercase().contains(needle))
        || t.tags.iter().any(|tag| tag.to_lowercase().contains(needle))
}

type SortKey = (
    bool,
    Option<NaiveDate>,
    bool,
    Option<NaiveTime>,
    u8,
    DateTime<Utc>,
);

/// 마감일 있는 것 먼저(빠른 날짜 순) → 같은 날은 시각 있는 것 먼저(빠른 시각 순)
/// → 우선순위 높은 순 → 먼저 만든 순. `Option`은 `None`이 작으므로 bool 플래그로 뒤로 보낸다.
fn sort_key(t: &Task) -> SortKey {
    (
        t.due_date.is_none(),
        t.due_date,
        t.due_time.is_none(),
        t.due_time,
        Priority::rank(t.priority),
        t.created_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn task(
        title: &str,
        due: Option<&str>,
        time: Option<&str>,
        priority: Priority,
        tags: &[&str],
        done: bool,
    ) -> Task {
        let created = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
        let mut t = Task::new(title, created, "pc");
        t.due_date = due.map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap());
        t.due_time = time.map(|d| NaiveTime::parse_from_str(d, "%H:%M").unwrap());
        t.priority = priority;
        t.tags = tags.iter().map(|s| s.to_string()).collect();
        if done {
            t.completed_at = Some(created);
        }
        t
    }

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 28)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap()
    }

    fn fixture() -> Vec<Task> {
        use Priority::*;
        vec![
            task(
                "지난 일",
                Some("2026-10-27"),
                None,
                Normal,
                &["학교"],
                false,
            ),
            task(
                "오늘 아침",
                Some("2026-10-28"),
                Some("09:00"),
                Normal,
                &[],
                false,
            ),
            task(
                "오늘 저녁",
                Some("2026-10-28"),
                Some("19:00"),
                High,
                &["개인"],
                false,
            ),
            task("오늘 종일", Some("2026-10-28"), None, Low, &[], false),
            task("내일", Some("2026-10-29"), None, Normal, &["학교"], false),
            task("언젠가", None, None, Normal, &[], false),
            task("끝난 일", Some("2026-10-27"), None, Normal, &["학교"], true),
        ]
    }

    fn titles(v: &[Task]) -> Vec<&str> {
        v.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn today_view_includes_overdue_and_excludes_done_and_undated() {
        let got = select(&fixture(), &Filter::view(View::Today), now());
        assert_eq!(
            titles(&got),
            ["지난 일", "오늘 아침", "오늘 저녁", "오늘 종일"]
        );
    }

    #[test]
    fn upcoming_view_is_after_today() {
        let got = select(&fixture(), &Filter::view(View::Upcoming), now());
        assert_eq!(titles(&got), ["내일"]);
    }

    #[test]
    fn overdue_view_uses_time_when_present() {
        let got = select(&fixture(), &Filter::view(View::Overdue), now());
        // 오늘 아침(09:00)은 지났고, 오늘 저녁(19:00)과 시각 없는 오늘 종일은 아직이다.
        assert_eq!(titles(&got), ["지난 일", "오늘 아침"]);
    }

    #[test]
    fn completed_view_lists_only_done() {
        let got = select(&fixture(), &Filter::view(View::Completed), now());
        assert_eq!(titles(&got), ["끝난 일"]);
    }

    #[test]
    fn all_view_includes_done_and_puts_undated_last() {
        let got = select(&fixture(), &Filter::view(View::All), now());
        assert_eq!(got.len(), 7);
        assert_eq!(got.last().unwrap().title, "언젠가");
    }

    #[test]
    fn same_day_sorts_by_time_then_priority() {
        use Priority::*;
        let tasks = vec![
            task("낮음", Some("2026-10-30"), None, Low, &[], false),
            task("높음", Some("2026-10-30"), None, High, &[], false),
            task("보통", Some("2026-10-30"), None, Normal, &[], false),
            task(
                "시각 있음",
                Some("2026-10-30"),
                Some("08:00"),
                Low,
                &[],
                false,
            ),
        ];
        let got = select(&tasks, &Filter::view(View::All), now());
        assert_eq!(titles(&got), ["시각 있음", "높음", "보통", "낮음"]);
    }

    #[test]
    fn deleted_tasks_are_never_returned() {
        let mut tasks = fixture();
        tasks[0].deleted = true;
        let got = select(&tasks, &Filter::view(View::All), now());
        assert!(!titles(&got).contains(&"지난 일"));
    }

    #[test]
    fn tag_filter_matches_exact_tag() {
        let filter = Filter {
            view: View::All,
            tag: Some("학교"),
            search: None,
        };
        let got = select(&fixture(), &filter, now());
        assert_eq!(titles(&got), ["지난 일", "끝난 일", "내일"]);
    }

    #[test]
    fn search_matches_title_notes_and_tags_case_insensitively() {
        let mut tasks = fixture();
        tasks[5].notes = Some("Call ABC Corp".into());
        tasks[4].tags.push("Project".into());

        let by_title = Filter {
            view: View::All,
            tag: None,
            search: Some("저녁"),
        };
        assert_eq!(titles(&select(&tasks, &by_title, now())), ["오늘 저녁"]);

        let by_notes = Filter {
            view: View::All,
            tag: None,
            search: Some("abc corp"),
        };
        assert_eq!(titles(&select(&tasks, &by_notes, now())), ["언젠가"]);

        let by_tag = Filter {
            view: View::All,
            tag: None,
            search: Some("PROJECT"),
        };
        assert_eq!(titles(&select(&tasks, &by_tag, now())), ["내일"]);
    }

    #[test]
    fn blank_search_is_ignored() {
        let filter = Filter {
            view: View::All,
            tag: None,
            search: Some("  "),
        };
        // 공백만 있는 검색어는 "필터 없음"과 같아야 한다.
        assert_eq!(select(&fixture(), &filter, now()).len(), 7);
    }

    #[test]
    fn all_tags_counts_open_tasks_only() {
        let got = all_tags(&fixture());
        assert_eq!(
            got,
            vec![
                TagCount {
                    name: "학교".into(),
                    count: 2
                },
                TagCount {
                    name: "개인".into(),
                    count: 1
                },
            ]
        );
    }
}

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;

use super::{ParsedTask, TaskParser};
use crate::model::{normalize_tags, Priority};

pub struct RuleBasedParser {
    tag: Regex,
    priority: Regex,
}

impl Default for RuleBasedParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleBasedParser {
    pub fn new() -> Self {
        RuleBasedParser {
            tag: re(r"#(\S+)"),
            priority: re(r"!(?:(높음|중요|낮음)|!)"),
        }
    }
}

impl TaskParser for RuleBasedParser {
    fn parse(&self, input: &str, _now: NaiveDateTime) -> ParsedTask {
        let mut rest = input.to_string();

        let mut tags = Vec::new();
        while let Some(tag) = extract(&mut rest, &self.tag, |g| Some(g[0].clone())) {
            tags.push(tag);
        }

        let priority = extract(&mut rest, &self.priority, |g| {
            Some(if g[0] == "낮음" { Priority::Low } else { Priority::High })
        })
        .unwrap_or_default();

        ParsedTask {
            title: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            due_date: None::<NaiveDate>,
            due_time: None::<NaiveTime>,
            priority,
            tags: normalize_tags(tags),
        }
    }
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

/// `re`의 첫 번째 "유효한" 매치를 찾아 `rest`에서 지우고(공백으로 치환) `f`의 결과를 돌려준다.
/// `f`가 `None`을 돌려주면 그 매치는 무효로 보고 `rest`를 건드리지 않은 채 다음 매치를 시도한다.
/// `f`에는 캡처 그룹 1..n의 문자열이 전달된다. (참여하지 않은 그룹은 빈 문자열)
fn extract<T>(rest: &mut String, re: &Regex, f: impl Fn(&[String]) -> Option<T>) -> Option<T> {
    let mut found = None;
    for caps in re.captures_iter(rest.as_str()) {
        let groups: Vec<String> = caps
            .iter()
            .skip(1)
            .map(|g| g.map_or(String::new(), |m| m.as_str().to_string()))
            .collect();
        if let Some(value) = f(&groups) {
            let whole = caps.get(0).expect("group 0 always exists");
            found = Some((whole.start(), whole.end(), value));
            break;
        }
    }
    let (start, end, value) = found?;
    rest.replace_range(start..end, " ");
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn now() -> NaiveDateTime {
        // 2026-10-28 (수) 15:00
        NaiveDate::from_ymd_opt(2026, 10, 28)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap()
    }

    pub(super) fn parse(input: &str) -> ParsedTask {
        RuleBasedParser::new().parse(input, now())
    }

    #[test]
    fn plain_text_becomes_title_with_whitespace_collapsed() {
        let p = parse("  장보기   가기  ");
        assert_eq!(p.title, "장보기 가기");
        assert_eq!(p.priority, Priority::Normal);
        assert!(p.tags.is_empty());
    }

    #[test]
    fn empty_input_gives_empty_title() {
        assert_eq!(parse("   ").title, "");
    }

    #[test]
    fn tags_are_extracted_in_order_and_removed_from_title() {
        let p = parse("장보기 #집 #마트");
        assert_eq!(p.title, "장보기");
        assert_eq!(p.tags, ["집", "마트"]);
    }

    #[test]
    fn tag_may_come_first_and_duplicates_collapse() {
        let p = parse("#집 빨래 #집");
        assert_eq!(p.title, "빨래");
        assert_eq!(p.tags, ["집"]);
    }

    #[test]
    fn priority_markers() {
        assert_eq!(parse("a !중요").priority, Priority::High);
        assert_eq!(parse("a !높음").priority, Priority::High);
        assert_eq!(parse("a !!").priority, Priority::High);
        assert_eq!(parse("a !낮음").priority, Priority::Low);
        assert_eq!(parse("a").priority, Priority::Normal);
        assert_eq!(parse("a !중요").title, "a");
        assert_eq!(parse("a !!").title, "a");
    }
}

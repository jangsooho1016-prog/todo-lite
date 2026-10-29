use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;

use super::{ParsedTask, TaskParser};
use crate::model::{normalize_tags, Priority};

/// 날짜·시각 표현 뒤에 붙는 조사. 함께 소비한다. ("내일까지", "3시에")
const SUFFIX: &str = "(?:까지|부터|에)?";

pub struct RuleBasedParser {
    tag: Regex,
    priority: Regex,
    iso_date: Regex,
    month_day: Regex,
    slash_date: Regex,
    days_later: Regex,
    week_weekday: Regex,
    next_week: Regex,
    day_word: Regex,
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
            iso_date: re(&format!(r"(\d{{4}})-(\d{{1,2}})-(\d{{1,2}}){SUFFIX}")),
            month_day: re(&format!(r"(\d{{1,2}})월\s*(\d{{1,2}})일{SUFFIX}")),
            slash_date: re(&format!(r"(\d{{1,2}})/(\d{{1,2}}){SUFFIX}")),
            days_later: re(&format!(r"(\d+)일\s*(?:후|뒤){SUFFIX}")),
            week_weekday: re(&format!(r"(이번|다음)\s*주\s*(월|화|수|목|금|토|일)요일{SUFFIX}")),
            next_week: re(&format!(r"다음\s*주{SUFFIX}")),
            day_word: re(&format!(r"(오늘|내일|모레|글피){SUFFIX}")),
        }
    }

    /// 날짜 표현 하나를 찾아 지운다. 구체적인 표현부터 시도한다.
    fn take_date(&self, rest: &mut String, today: NaiveDate) -> Option<NaiveDate> {
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));

        if let Some(date) = extract(rest, &self.iso_date, |g| {
            NaiveDate::from_ymd_opt(num(&g[0])?, num(&g[1])?, num(&g[2])?)
        }) {
            return Some(date);
        }
        if let Some(date) = extract(rest, &self.month_day, |g| {
            upcoming(today, num(&g[0])?, num(&g[1])?)
        }) {
            return Some(date);
        }
        if let Some(date) = extract(rest, &self.slash_date, |g| {
            upcoming(today, num(&g[0])?, num(&g[1])?)
        }) {
            return Some(date);
        }
        if let Some(date) = extract(rest, &self.days_later, |g| {
            let n: i64 = num(&g[0])?;
            if n > 36_500 {
                return None;
            }
            today.checked_add_signed(Duration::days(n))
        }) {
            return Some(date);
        }
        if let Some(date) = extract(rest, &self.week_weekday, |g| {
            let offset = if g[0] == "다음" { 7 } else { 0 };
            Some(monday + Duration::days(offset + weekday_index(&g[1])?))
        }) {
            return Some(date);
        }
        if let Some(date) = extract(rest, &self.next_week, |_| Some(monday + Duration::days(7))) {
            return Some(date);
        }
        extract(rest, &self.day_word, |g| {
            let days = match g[0].as_str() {
                "오늘" => 0,
                "내일" => 1,
                "모레" => 2,
                "글피" => 3,
                _ => return None,
            };
            Some(today + Duration::days(days))
        })
    }
}

impl TaskParser for RuleBasedParser {
    fn parse(&self, input: &str, now: NaiveDateTime) -> ParsedTask {
        let mut rest = input.to_string();

        let mut tags = Vec::new();
        while let Some(tag) = extract(&mut rest, &self.tag, |g| Some(g[0].clone())) {
            tags.push(tag);
        }

        let priority = extract(&mut rest, &self.priority, |g| {
            Some(if g[0] == "낮음" { Priority::Low } else { Priority::High })
        })
        .unwrap_or_default();

        let due_date = self.take_date(&mut rest, now.date());

        ParsedTask {
            title: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            due_date,
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

fn num<T: FromStr>(s: &str) -> Option<T> {
    s.parse().ok()
}

/// 연도 없는 월/일을 해석한다. 올해 날짜가 오늘보다 앞서면 내년으로 본다.
fn upcoming(today: NaiveDate, month: u32, day: u32) -> Option<NaiveDate> {
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this_year >= today {
        Some(this_year)
    } else {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)
    }
}

fn weekday_index(s: &str) -> Option<i64> {
    ["월", "화", "수", "목", "금", "토", "일"]
        .iter()
        .position(|d| *d == s)
        .map(|i| i as i64)
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

    fn date(input: &str) -> Option<NaiveDate> {
        parse(input).due_date
    }

    fn d(y: i32, m: u32, day: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(y, m, day)
    }

    #[test]
    fn relative_day_words() {
        assert_eq!(date("오늘 운동"), d(2026, 10, 28));
        assert_eq!(date("내일 병원"), d(2026, 10, 29));
        assert_eq!(date("모레 세미나"), d(2026, 10, 30));
        assert_eq!(date("글피 여행"), d(2026, 10, 31));
        assert_eq!(parse("내일 병원").title, "병원");
    }

    #[test]
    fn n_days_later() {
        assert_eq!(date("3일 후 반납"), d(2026, 10, 31));
        assert_eq!(date("5일 뒤 제출"), d(2026, 11, 2));
        assert!(date("99999일 후 먼 미래").is_none());
    }

    #[test]
    fn weekday_expressions() {
        assert_eq!(date("이번 주 금요일 발표"), d(2026, 10, 30));
        assert_eq!(date("이번 주 일요일 청소"), d(2026, 11, 1));
        assert_eq!(date("다음 주 월요일 회의"), d(2026, 11, 2));
        assert_eq!(date("다음 주 수요일 면접"), d(2026, 11, 4));
        assert_eq!(date("다음 주 과제"), d(2026, 11, 2));
        assert_eq!(parse("다음 주 월요일 회의").title, "회의");
    }

    #[test]
    fn absolute_dates() {
        assert_eq!(date("10월 30일 과제"), d(2026, 10, 30));
        assert_eq!(date("10/30 팀플"), d(2026, 10, 30));
        assert_eq!(date("2026-11-15 졸업 사진"), d(2026, 11, 15));
        assert_eq!(parse("10월 30일 과제").title, "과제");
    }

    #[test]
    fn past_month_day_rolls_over_to_next_year() {
        assert_eq!(date("1월 5일 신년 계획"), d(2027, 1, 5));
        // 오늘과 같은 날짜는 올해로 본다.
        assert_eq!(date("10월 28일 오늘 마감"), d(2026, 10, 28));
    }

    #[test]
    fn invalid_dates_are_left_in_the_title() {
        let p = parse("2월 31일 존재하지 않는 날");
        assert_eq!(p.due_date, None);
        assert_eq!(p.title, "2월 31일 존재하지 않는 날");
    }

    #[test]
    fn particles_after_dates_are_consumed() {
        let p = parse("회의록 정리 내일까지");
        assert_eq!(p.due_date, d(2026, 10, 29));
        assert_eq!(p.title, "회의록 정리");
    }
}

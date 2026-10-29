use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;

use super::{ParsedTask, TaskParser};
use crate::model::{normalize_tags, Priority};

/// 날짜·시각 표현 뒤에 붙는 조사. 함께 소비한다. ("내일까지", "3시에")
const SUFFIX: &str = "(?:까지|부터|에)?";

/// 표현 앞 경계. 공백 하나를 함께 소비한다. (regex 크레이트는 lookbehind가 없다)
const START: &str = r"(?:^|\s)";
/// 표현 뒤 경계. 공백/구두점 하나를 함께 소비한다. (regex 크레이트는 lookahead가 없다)
const END: &str = r"(?:\s|$|[,.;:!?)])";

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
    hour_minute: Regex,
    clock: Regex,
    time_word: Regex,
}

impl Default for RuleBasedParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleBasedParser {
    pub fn new() -> Self {
        RuleBasedParser {
            tag: re(r"#([^\s,.;:!?()]+)[,.;:!?)]*"),
            priority: re(r"!(?:(높음|중요|낮음)|!)"),
            iso_date: re(&format!(r"(\d{{4}})-(\d{{1,2}})-(\d{{1,2}}){SUFFIX}")),
            month_day: re(&format!(r"(\d{{1,2}})월\s*(\d{{1,2}})일{SUFFIX}")),
            // 알려진 한계: "우유 1/2 통"의 1/2는 1월 2일과 구별할 수 없어 날짜로 해석된다.
            slash_date: re(&format!(r"{START}(\d{{1,2}})/(\d{{1,2}}){SUFFIX}{END}")),
            days_later: re(&format!(r"{START}(\d+)일\s*(?:후|뒤){SUFFIX}{END}")),
            week_weekday: re(&format!(
                r"{START}(이번|다음)\s*주\s*(월|화|수|목|금|토|일)요일{SUFFIX}{END}"
            )),
            next_week: re(&format!(r"{START}다음\s*주{SUFFIX}{END}")),
            day_word: re(&format!(r"{START}(오늘|내일|모레|글피){SUFFIX}{END}")),
            hour_minute: re(&format!(r"(\d{{1,2}}):(\d{{2}}){SUFFIX}")),
            // 그룹: 1 오전/오후 접두어, 2 시, 3 "간"(시간 duration이면 거부), 4 "반", 5 분
            clock: re(&format!(
                r"(오전|오후|아침|저녁|밤|새벽)?\s*(\d{{1,2}})시(간)?\s*(?:(반)|(\d{{1,2}})분)?{SUFFIX}{END}"
            )),
            time_word: re(&format!(r"(아침|점심|저녁){SUFFIX}")),
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

impl RuleBasedParser {
    /// 시각 표현 하나를 찾아 지운다. 명시적 시각이 있으면 `점심` 같은 단어는 건드리지 않는다.
    fn take_time(&self, rest: &mut String) -> Option<NaiveTime> {
        if let Some(time) = extract(rest, &self.hour_minute, |g| {
            NaiveTime::from_hms_opt(num(&g[0])?, num(&g[1])?, 0)
        }) {
            return Some(time);
        }
        if let Some(time) = extract(rest, &self.clock, |g| {
            if !g[2].is_empty() {
                return None; // "2시간"은 시각이 아니다
            }
            let hour: u32 = num(&g[1])?;
            let minute: u32 = if !g[3].is_empty() {
                30
            } else if !g[4].is_empty() {
                num(&g[4])?
            } else {
                0
            };
            let hour = match g[0].as_str() {
                "오전" | "새벽" => {
                    if hour == 12 {
                        0
                    } else {
                        hour
                    }
                }
                "밤" if hour == 12 => 0,
                "오후" | "저녁" | "밤" => {
                    if hour < 12 {
                        hour + 12
                    } else {
                        hour
                    }
                }
                _ => hour,
            };
            NaiveTime::from_hms_opt(hour, minute, 0)
        }) {
            return Some(time);
        }
        extract(rest, &self.time_word, |g| match g[0].as_str() {
            "아침" => NaiveTime::from_hms_opt(9, 0, 0),
            "점심" => NaiveTime::from_hms_opt(12, 0, 0),
            "저녁" => NaiveTime::from_hms_opt(19, 0, 0),
            _ => None,
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
            Some(if g[0] == "낮음" {
                Priority::Low
            } else {
                Priority::High
            })
        })
        .unwrap_or_default();

        let today = now.date();
        let due_date = self.take_date(&mut rest, today);
        let due_time = self.take_time(&mut rest);
        // 날짜 없이 시각만 있으면 오늘로 본다. 이미 지난 시각이어도 내일로 넘기지 않는다.
        let due_date = due_date.or_else(|| due_time.map(|_| today));

        ParsedTask {
            title: rest.split_whitespace().collect::<Vec<_>>().join(" "),
            due_date,
            due_time,
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
    // 윤일(2/29)은 최대 8년 뒤(세기 윤년 예외)까지 찾아본다.
    (today.year()..=today.year() + 8)
        .filter_map(|year| NaiveDate::from_ymd_opt(year, month, day))
        .find(|date| *date >= today)
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

    fn time(input: &str) -> Option<NaiveTime> {
        parse(input).due_time
    }

    fn hm(h: u32, m: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(h, m, 0)
    }

    #[test]
    fn clock_expressions() {
        assert_eq!(time("오후 3시 회의"), hm(15, 0));
        assert_eq!(time("오전 9시 운동"), hm(9, 0));
        assert_eq!(time("오후 2시 30분 상담"), hm(14, 30));
        assert_eq!(time("오후 3시 반 미팅"), hm(15, 30));
        assert_eq!(time("15:00 스터디"), hm(15, 0));
        assert_eq!(time("9시 30분 출근"), hm(9, 30));
        assert_eq!(parse("오후 3시 회의").title, "회의");
    }

    #[test]
    fn meridiem_edge_cases() {
        assert_eq!(time("오전 12시 알람"), hm(0, 0));
        assert_eq!(time("오후 12시 점심"), hm(12, 0));
        assert_eq!(time("저녁 7시 영화"), hm(19, 0));
        assert_eq!(time("밤 12시 마감"), hm(0, 0));
    }

    #[test]
    fn hour_without_meridiem_is_24h_literal() {
        assert_eq!(time("3시 반 낮잠"), hm(3, 30));
        assert_eq!(time("18시 퇴근"), hm(18, 0));
    }

    #[test]
    fn time_words() {
        assert_eq!(time("아침 조깅"), hm(9, 0));
        assert_eq!(time("점심 약속"), hm(12, 0));
        assert_eq!(time("저녁 장보기"), hm(19, 0));
        assert_eq!(parse("저녁 장보기").title, "장보기");
    }

    #[test]
    fn explicit_time_wins_over_time_words() {
        let p = parse("오후 12시 점심");
        assert_eq!(p.due_time, hm(12, 0));
        assert_eq!(p.title, "점심");

        let p = parse("점심 12:30 약속");
        assert_eq!(p.due_time, hm(12, 30));
        assert_eq!(p.title, "점심 약속");
    }

    #[test]
    fn invalid_or_duration_times_stay_in_the_title() {
        let p = parse("25시 이상한 일정");
        assert_eq!((p.due_time, p.title.as_str()), (None, "25시 이상한 일정"));

        let p = parse("2시간 공부");
        assert_eq!((p.due_time, p.title.as_str()), (None, "2시간 공부"));
    }

    #[test]
    fn time_without_date_means_today_even_if_already_past() {
        let p = parse("오전 9시 운동"); // 지금은 15:00
        assert_eq!(p.due_date, d(2026, 10, 28));
        assert_eq!(p.due_time, hm(9, 0));
    }

    #[test]
    fn particle_after_time_is_consumed() {
        let p = parse("오후 3시에 치과");
        assert_eq!(p.due_time, hm(15, 0));
        assert_eq!(p.title, "치과");
    }

    #[test]
    fn date_and_time_and_tags_and_priority_together() {
        let p = parse("내일 오후 3시 팀플 회의 #학교 !중요");
        assert_eq!(p.title, "팀플 회의");
        assert_eq!(p.due_date, d(2026, 10, 29));
        assert_eq!(p.due_time, hm(15, 0));
        assert_eq!(p.priority, Priority::High);
        assert_eq!(p.tags, ["학교"]);
    }

    #[test]
    fn ban_requires_a_word_boundary() {
        let p = parse("오후 3시 반납");
        assert_eq!((p.due_time, p.title.as_str()), (hm(15, 0), "반납"));
        let p = parse("3시 반찬 사기");
        assert_eq!((p.due_time, p.title.as_str()), (hm(3, 0), "반찬 사기"));
        let p = parse("오후 3시 반 미팅");
        assert_eq!((p.due_time, p.title.as_str()), (hm(15, 30), "미팅"));
        let p = parse("오후 3시에 치과");
        assert_eq!((p.due_time, p.title.as_str()), (hm(15, 0), "치과"));
        let p = parse("오후 3시 반에 만나");
        assert_eq!((p.due_time, p.title.as_str()), (hm(15, 30), "만나"));
        assert_eq!(time("오후 2시 30분 상담"), hm(14, 30));
        assert_eq!(time("9시 30분 출근"), hm(9, 30));
        let p = parse("25시 이상한 일정");
        assert_eq!((p.due_time, p.title.as_str()), (None, "25시 이상한 일정"));
        let p = parse("2시간 공부");
        assert_eq!((p.due_time, p.title.as_str()), (None, "2시간 공부"));
    }

    #[test]
    fn slash_dates_need_surrounding_boundaries() {
        for input in ["https://example.com/1/2 확인", "1/2/3 메모", "v1/2x 점검"] {
            let p = parse(input);
            assert_eq!(p.due_date, None, "{input}");
            assert_eq!(p.title, input);
        }
        assert_eq!(date("10/30 팀플"), d(2026, 10, 30));
        let p = parse("마감 10/30");
        assert_eq!((p.due_date, p.title.as_str()), (d(2026, 10, 30), "마감"));
        let p = parse("10/30에 제출");
        assert_eq!((p.due_date, p.title.as_str()), (d(2026, 10, 30), "제출"));
    }

    #[test]
    fn day_words_need_boundaries() {
        for input in ["내일배움카드 신청", "오늘의 할 일"] {
            let p = parse(input);
            assert_eq!(p.due_date, None, "{input}");
            assert_eq!(p.title, input);
        }
        let p = parse("내일, 병원");
        assert_eq!((p.due_date, p.title.as_str()), (d(2026, 10, 29), "병원"));
        let p = parse("회의는 내일");
        assert_eq!((p.due_date, p.title.as_str()), (d(2026, 10, 29), "회의는"));
        let p = parse("회의록 정리 내일까지");
        assert_eq!(
            (p.due_date, p.title.as_str()),
            (d(2026, 10, 29), "회의록 정리")
        );
        let p = parse("내일");
        assert_eq!((p.due_date, p.title.as_str()), (d(2026, 10, 29), ""));
        let p = parse("  내일   공백   많음  ");
        assert_eq!(
            (p.due_date, p.title.as_str()),
            (d(2026, 10, 29), "공백 많음")
        );
    }

    #[test]
    fn feb_29_rolls_to_the_next_leap_year() {
        let p = parse("2월 29일 생일");
        assert_eq!((p.due_date, p.title.as_str()), (d(2028, 2, 29), "생일"));
        assert_eq!(date("1월 5일 신년 계획"), d(2027, 1, 5));
        assert_eq!(date("10월 28일 오늘 마감"), d(2026, 10, 28));
        for input in ["2월 31일 없는 날", "13월 1일 없는 날"] {
            let p = parse(input);
            assert_eq!(p.due_date, None, "{input}");
            assert_eq!(p.title, input);
        }
    }

    #[test]
    fn tag_trailing_punctuation_is_not_part_of_the_tag() {
        for (input, title, tag) in [
            ("메모 #학교, 확인", "메모 확인", "학교"),
            ("#학교. 확인", "확인", "학교"),
            ("끝 #집)", "끝", "집"),
        ] {
            let p = parse(input);
            assert_eq!(p.tags, [tag], "{input}");
            assert_eq!(p.title, title, "{input}");
        }
    }

    type Case = (
        &'static str,         // 입력
        &'static str,         // 기대 제목
        Option<&'static str>, // 기대 날짜 YYYY-MM-DD
        Option<&'static str>, // 기대 시각 HH:MM
        Priority,
        &'static [&'static str], // 기대 태그
    );

    const N: Priority = Priority::Normal;
    const H: Priority = Priority::High;
    const L: Priority = Priority::Low;

    const CORPUS: &[Case] = &[
        ("장보기", "장보기", None, None, N, &[]),
        ("오늘 운동", "운동", Some("2026-10-28"), None, N, &[]),
        ("내일 병원", "병원", Some("2026-10-29"), None, N, &[]),
        ("모레 세미나", "세미나", Some("2026-10-30"), None, N, &[]),
        (
            "글피 여행 준비",
            "여행 준비",
            Some("2026-10-31"),
            None,
            N,
            &[],
        ),
        ("3일 후 반납", "반납", Some("2026-10-31"), None, N, &[]),
        ("5일 뒤 제출", "제출", Some("2026-11-02"), None, N, &[]),
        (
            "다음 주 월요일 회의",
            "회의",
            Some("2026-11-02"),
            None,
            N,
            &[],
        ),
        (
            "이번 주 금요일 발표",
            "발표",
            Some("2026-10-30"),
            None,
            N,
            &[],
        ),
        (
            "다음 주 수요일 면접",
            "면접",
            Some("2026-11-04"),
            None,
            N,
            &[],
        ),
        (
            "이번 주 일요일 청소",
            "청소",
            Some("2026-11-01"),
            None,
            N,
            &[],
        ),
        ("다음 주 과제", "과제", Some("2026-11-02"), None, N, &[]),
        (
            "10월 30일 과제 제출",
            "과제 제출",
            Some("2026-10-30"),
            None,
            N,
            &[],
        ),
        ("11월 3일 시험", "시험", Some("2026-11-03"), None, N, &[]),
        (
            "1월 5일 신년 계획",
            "신년 계획",
            Some("2027-01-05"),
            None,
            N,
            &[],
        ),
        ("10/30 팀플", "팀플", Some("2026-10-30"), None, N, &[]),
        (
            "2026-11-15 졸업 사진",
            "졸업 사진",
            Some("2026-11-15"),
            None,
            N,
            &[],
        ),
        (
            "12월 25일 선물 사기",
            "선물 사기",
            Some("2026-12-25"),
            None,
            N,
            &[],
        ),
        (
            "오후 3시 회의",
            "회의",
            Some("2026-10-28"),
            Some("15:00"),
            N,
            &[],
        ),
        (
            "오전 9시 운동",
            "운동",
            Some("2026-10-28"),
            Some("09:00"),
            N,
            &[],
        ),
        (
            "15:00 스터디",
            "스터디",
            Some("2026-10-28"),
            Some("15:00"),
            N,
            &[],
        ),
        (
            "오후 2시 30분 상담",
            "상담",
            Some("2026-10-28"),
            Some("14:30"),
            N,
            &[],
        ),
        (
            "오후 3시 반 미팅",
            "미팅",
            Some("2026-10-28"),
            Some("15:30"),
            N,
            &[],
        ),
        (
            "점심 약속",
            "약속",
            Some("2026-10-28"),
            Some("12:00"),
            N,
            &[],
        ),
        (
            "저녁 장보기",
            "장보기",
            Some("2026-10-28"),
            Some("19:00"),
            N,
            &[],
        ),
        (
            "아침 조깅",
            "조깅",
            Some("2026-10-28"),
            Some("09:00"),
            N,
            &[],
        ),
        (
            "저녁 7시 영화",
            "영화",
            Some("2026-10-28"),
            Some("19:00"),
            N,
            &[],
        ),
        (
            "오전 12시 알람",
            "알람",
            Some("2026-10-28"),
            Some("00:00"),
            N,
            &[],
        ),
        (
            "오후 12시 점심",
            "점심",
            Some("2026-10-28"),
            Some("12:00"),
            N,
            &[],
        ),
        (
            "내일 오후 3시 팀플 회의 #학교 !중요",
            "팀플 회의",
            Some("2026-10-29"),
            Some("15:00"),
            H,
            &["학교"],
        ),
        (
            "내일 저녁 친구 만나기 #개인",
            "친구 만나기",
            Some("2026-10-29"),
            Some("19:00"),
            N,
            &["개인"],
        ),
        (
            "모레 오전 10시 치과 !!",
            "치과",
            Some("2026-10-30"),
            Some("10:00"),
            H,
            &[],
        ),
        (
            "다음 주 금요일 14:30 발표 리허설 #학교 #팀플",
            "발표 리허설",
            Some("2026-11-06"),
            Some("14:30"),
            N,
            &["학교", "팀플"],
        ),
        ("책 읽기 !낮음", "책 읽기", None, None, L, &[]),
        (
            "보고서 작성 !높음 #회사",
            "보고서 작성",
            None,
            None,
            H,
            &["회사"],
        ),
        (
            "!중요 세금 납부 10월 31일",
            "세금 납부",
            Some("2026-10-31"),
            None,
            H,
            &[],
        ),
        ("#집 빨래", "빨래", None, None, N, &["집"]),
        ("장보기 #집 #마트", "장보기", None, None, N, &["집", "마트"]),
        (
            "오늘 오후 6시 퇴근 후 운동 #건강",
            "퇴근 후 운동",
            Some("2026-10-28"),
            Some("18:00"),
            N,
            &["건강"],
        ),
        (
            "7일 뒤 보험 갱신 !!",
            "보험 갱신",
            Some("2026-11-04"),
            None,
            H,
            &[],
        ),
        (
            "10월 29일 오후 1시 면접",
            "면접",
            Some("2026-10-29"),
            Some("13:00"),
            N,
            &[],
        ),
        (
            "9시 30분 출근",
            "출근",
            Some("2026-10-28"),
            Some("09:30"),
            N,
            &[],
        ),
        ("25시 이상한 일정", "25시 이상한 일정", None, None, N, &[]),
        (
            "2월 31일 존재하지 않는 날",
            "2월 31일 존재하지 않는 날",
            None,
            None,
            N,
            &[],
        ),
        (
            "  내일   공백   많음  ",
            "공백 많음",
            Some("2026-10-29"),
            None,
            N,
            &[],
        ),
        ("내일", "", Some("2026-10-29"), None, N, &[]),
        ("2시간 공부", "2시간 공부", None, None, N, &[]),
        (
            "점심 12:30 약속",
            "점심 약속",
            Some("2026-10-28"),
            Some("12:30"),
            N,
            &[],
        ),
        (
            "!낮음 #기타 오늘 정리",
            "정리",
            Some("2026-10-28"),
            None,
            L,
            &["기타"],
        ),
        (
            "회의록 정리 내일까지",
            "회의록 정리",
            Some("2026-10-29"),
            None,
            N,
            &[],
        ),
        (
            "오후 3시에 치과",
            "치과",
            Some("2026-10-28"),
            Some("15:00"),
            N,
            &[],
        ),
        (
            "3시 반 낮잠",
            "낮잠",
            Some("2026-10-28"),
            Some("03:30"),
            N,
            &[],
        ),
        (
            "10일 후 이사 준비",
            "이사 준비",
            Some("2026-11-07"),
            None,
            N,
            &[],
        ),
        (
            "오후 3시 반납",
            "반납",
            Some("2026-10-28"),
            Some("15:00"),
            N,
            &[],
        ),
        (
            "3시 반찬 사기",
            "반찬 사기",
            Some("2026-10-28"),
            Some("03:00"),
            N,
            &[],
        ),
        (
            "오후 3시 반에 만나",
            "만나",
            Some("2026-10-28"),
            Some("15:30"),
            N,
            &[],
        ),
        (
            "https://example.com/1/2 확인",
            "https://example.com/1/2 확인",
            None,
            None,
            N,
            &[],
        ),
        ("1/2/3 메모", "1/2/3 메모", None, None, N, &[]),
        ("내일배움카드 신청", "내일배움카드 신청", None, None, N, &[]),
        ("오늘의 할 일", "오늘의 할 일", None, None, N, &[]),
        ("내일, 병원", "병원", Some("2026-10-29"), None, N, &[]),
        ("회의는 내일", "회의는", Some("2026-10-29"), None, N, &[]),
        ("2월 29일 생일", "생일", Some("2028-02-29"), None, N, &[]),
        ("메모 #학교, 확인", "메모 확인", None, None, N, &["학교"]),
        ("끝 #집)", "끝", None, None, N, &["집"]),
    ];

    #[test]
    fn corpus_has_at_least_fifty_cases() {
        assert!(CORPUS.len() >= 50, "only {} cases", CORPUS.len());
    }

    #[test]
    fn corpus_all_cases_parse_as_expected() {
        let parser = RuleBasedParser::new();
        let mut failures = Vec::new();
        for &(input, title, date, time, priority, tags) in CORPUS {
            let want = ParsedTask {
                title: title.to_string(),
                due_date: date.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()),
                due_time: time.map(|s| NaiveTime::parse_from_str(s, "%H:%M").unwrap()),
                priority,
                tags: tags.iter().map(|s| s.to_string()).collect(),
            };
            let got = parser.parse(input, now());
            if got != want {
                failures.push(format!(
                    "{input:?}
  want {want:?}
  got  {got:?}"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} / {} cases failed:
{}",
            failures.len(),
            CORPUS.len(),
            failures.join(
                "
"
            )
        );
    }
}

mod rule_based;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::model::Priority;

pub use rule_based::RuleBasedParser;

/// 자연어 입력을 해석한 결과. 제목이 빈 문자열일 수 있다 (저장 여부는 호출한 쪽이 판단).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTask {
    pub title: String,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

/// 자연어 파서의 확장 지점. 규칙 기반 구현 외에 LLM 기반 구현을 추가할 수 있다.
pub trait TaskParser {
    /// `now`는 사용자의 현지 시각. "내일", "오후 3시" 같은 상대 표현의 기준이다.
    fn parse(&self, input: &str, now: NaiveDateTime) -> ParsedTask;
}

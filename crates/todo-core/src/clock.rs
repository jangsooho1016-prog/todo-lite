use chrono::{DateTime, Local, NaiveDateTime, Utc};

/// 같은 순간을 UTC(저장용)와 현지 시각(파싱·뷰 판정용)으로 함께 들고 다닌다.
#[derive(Debug, Clone, Copy)]
pub struct Now {
    pub utc: DateTime<Utc>,
    pub local: NaiveDateTime,
}

impl Now {
    pub fn system() -> Self {
        let now = Local::now();
        Now {
            utc: now.with_timezone(&Utc),
            local: now.naive_local(),
        }
    }

    /// 테스트용 고정 시계. 현지 시각을 UTC로 그대로 간주한다.
    pub fn fixed(local: NaiveDateTime) -> Self {
        Now {
            utc: local.and_utc(),
            local,
        }
    }
}

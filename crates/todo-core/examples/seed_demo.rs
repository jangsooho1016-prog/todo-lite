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

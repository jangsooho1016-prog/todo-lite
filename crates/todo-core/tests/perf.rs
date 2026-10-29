use std::time::Instant;

use chrono::NaiveDate;
use todo_core::clock::Now;
use todo_core::parser::RuleBasedParser;
use todo_core::query::{select, Filter, View};
use todo_core::service::TodoService;
use todo_core::store::{SqliteStore, TaskStore};

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "timing is only meaningful in release builds"
)]
fn list_and_search_10k_tasks_under_100ms() {
    let svc = TodoService::new(
        SqliteStore::open_in_memory().unwrap(),
        RuleBasedParser::new(),
        "pc".to_string(),
    );
    let now = Now::fixed(
        NaiveDate::from_ymd_opt(2026, 10, 28)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap(),
    );
    for i in 0..10_000 {
        svc.add_from_input(&format!("내일 오후 3시 할 일 {i} #태그{}", i % 20), now)
            .unwrap();
    }

    let start = Instant::now();
    let tasks = svc.store().all().unwrap();
    let filter = Filter {
        view: View::All,
        tag: None,
        search: Some("할 일 99"),
    };
    let hits = select(&tasks, &filter, now.local);
    let elapsed = start.elapsed();

    assert!(!hits.is_empty());
    assert!(
        elapsed.as_millis() < 100,
        "load + search over 10k tasks took {elapsed:?}"
    );
}

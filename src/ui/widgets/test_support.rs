use crate::models::hotspot::{Hotspot, HotspotTestData};

pub(crate) fn hotspot_with_counts(
    location: &str,
    touches: u64,
    lines_touched: u64,
    most_recent_days: i64,
    author_count: usize,
) -> Hotspot {
    let authors = (0..author_count)
        .map(|i| format!("author{i}@example.com"))
        .collect::<Vec<_>>();
    let author_refs = authors.iter().map(String::as_str).collect::<Vec<_>>();

    Hotspot::test_fixture(HotspotTestData {
        location,
        touches,
        lines_touched,
        recent_points: 2,
        most_recent_days,
        authors: &author_refs,
        recent_commits: vec![],
        co_changes: vec![],
        author_touches: vec![],
        default_rank_multiplier_percent: 100,
    })
}

pub(crate) fn hotspot_payment() -> Hotspot {
    Hotspot::test_fixture(HotspotTestData {
        location: "src/service/payment.rs",
        touches: 10,
        lines_touched: 220,
        recent_points: 7,
        most_recent_days: 2,
        authors: &[
            "alice@example.com",
            "bob@example.com",
            "carol@example.com",
            "dana@example.com",
        ],
        recent_commits: vec![
            (
                "abcdef0123456789",
                "Alice",
                "2026-03-01",
                "Refine payment retries",
                19,
            ),
            (
                "1234567deadbeef0",
                "Bob",
                "2026-02-27",
                "Split gateway adapter",
                12,
            ),
        ],
        co_changes: vec![("src/lib.rs", 8), ("src/main.rs", 3)],
        author_touches: vec![
            ("Alice <alice@example.com>", 6),
            ("Bob <bob@example.com>", 4),
        ],
        default_rank_multiplier_percent: 100,
    })
}

pub(crate) fn hotspot_ui() -> Hotspot {
    Hotspot::test_fixture(HotspotTestData {
        location: "src/ui/render.rs",
        touches: 5,
        lines_touched: 44,
        recent_points: 3,
        most_recent_days: 20,
        authors: &["eve@example.com"],
        recent_commits: vec![(
            "fedcba9876543210",
            "Eve",
            "2026-01-15",
            "Tune viewport math",
            7,
        )],
        co_changes: vec![("src/ui/layout.rs", 2)],
        author_touches: vec![("Eve <eve@example.com>", 5)],
        default_rank_multiplier_percent: 100,
    })
}

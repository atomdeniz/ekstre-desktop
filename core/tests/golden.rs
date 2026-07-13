//! Golden tests ported verbatim from the Python `tests/test_parser.py`.
//! Fixtures in `tests/fixtures/` are the same synthetic statement texts, and the
//! expected values pin exact parse output — this is the port-parity guarantee.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use ekstre_core::{
    builtin_banks, parse_amount, parse_amount_us, parse_date, parse_statement, Bank, Database,
    Statement,
};

fn banks() -> HashMap<String, Bank> {
    builtin_banks().into_iter().map(|b| (b.name.clone(), b)).collect()
}

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(path).unwrap()
}

#[test]
fn test_parse_amount() {
    assert_eq!(parse_amount("12.345,67"), 12345.67);
    assert_eq!(parse_amount("987,65"), 987.65);
    assert_eq!(parse_amount("5.000,00"), 5000.0);
    // Turkish round-amount shorthand ",-" means ",00" (seen in TEB PDFs).
    assert_eq!(parse_amount("24.396,-"), 24396.0);
    assert_eq!(parse_amount("1.234,-"), 1234.0);
}

#[test]
fn test_parse_amount_us() {
    // Akbank business statements: comma thousands, dot decimal.
    assert_eq!(parse_amount_us("10,964.11"), 10964.11);
    assert_eq!(parse_amount_us("2,741.03"), 2741.03);
    assert_eq!(parse_amount_us("125.00"), 125.0);
}

#[test]
fn test_parse_date() {
    assert_eq!(parse_date("25", "07", "2026"), "2026-07-25");
    assert_eq!(parse_date("5", "9", "2026"), "2026-09-05");
    assert_eq!(parse_date("7", "Mayıs", "2026"), "2026-05-07");
}

#[test]
fn test_teb() {
    let b = banks();
    let s = parse_statement(&b["TEB"], &fixture("teb.txt")).unwrap();
    assert_eq!(s.bank, "TEB");
    assert_eq!(s.card_last4.as_deref(), Some("5678"));
    assert_eq!(s.card_masked.as_deref(), Some("1234 **** **** 5678"));
    assert_eq!(s.total_due, 12345.67);
    assert_eq!(s.min_due, Some(1234.0)); // "TL.1.234,-" -> 1234.0
    assert_eq!(s.due_date, "2026-07-10");
    assert_eq!(s.statement_date.as_deref(), Some("2026-06-30")); // Hesap Kesim Tarihi
}

#[test]
fn test_enpara() {
    let b = banks();
    let s = parse_statement(&b["ENPARA"], &fixture("enpara.txt")).unwrap();
    assert_eq!(s.bank, "ENPARA");
    assert_eq!(s.card_last4.as_deref(), Some("9012"));
    assert_eq!(s.total_due, 9876.54);
    assert_eq!(s.min_due, Some(987.65));
    assert_eq!(s.due_date, "2026-08-20");
    assert_eq!(s.statement_date.as_deref(), Some("2026-08-05"));
}

#[test]
fn test_enpara_fallback_masked() {
    let b = banks();
    let s = parse_statement(&b["ENPARA"], &fixture("enpara_nocard.txt")).unwrap();
    assert_eq!(s.card_last4, None);
    assert_eq!(s.card_masked.as_deref(), Some("Enpara.com"));
    assert_eq!(s.total_due, 3000.0);
    assert_eq!(s.due_date, "2026-09-05");
}

#[test]
fn test_is() {
    let b = banks();
    let s = parse_statement(&b["İş Bankası"], &fixture("is.txt")).unwrap();
    assert_eq!(s.bank, "İş Bankası");
    assert_eq!(s.card_last4.as_deref(), Some("8765"));
    assert_eq!(s.card_masked.as_deref(), Some("4321 **** **** 8765"));
    assert_eq!(s.total_due, 5000.0);
    assert_eq!(s.min_due, Some(500.0));
    assert_eq!(s.due_date, "2026-09-15");
    assert_eq!(s.statement_date.as_deref(), Some("2026-09-03"));
}

#[test]
fn test_yapikredi() {
    let b = banks();
    let s = parse_statement(&b["Yapı Kredi"], &fixture("yapikredi.txt")).unwrap();
    assert_eq!(s.bank, "Yapı Kredi");
    assert_eq!(s.card_last4.as_deref(), Some("9803"));
    assert_eq!(s.card_masked.as_deref(), Some("4506 **** **** 9803"));
    assert_eq!(s.total_due, 2952.54);
    assert_eq!(s.min_due, Some(1181.02));
    assert_eq!(s.due_date, "2026-05-07");
    assert_eq!(s.statement_date.as_deref(), Some("2026-04-27"));
}

#[test]
fn test_garanti() {
    let b = banks();
    let s = parse_statement(&b["Garanti BBVA"], &fixture("garanti.txt")).unwrap();
    assert_eq!(s.bank, "Garanti BBVA");
    assert_eq!(s.card_last4.as_deref(), Some("3210"));
    assert_eq!(s.card_masked.as_deref(), Some("4444 **** **** 3210"));
    assert_eq!(s.total_due, 1234.56);
    assert_eq!(s.min_due, Some(370.0));
    assert_eq!(s.due_date, "2026-06-27");
    assert_eq!(s.statement_date.as_deref(), Some("2026-06-17"));
}

#[test]
fn test_axess_business() {
    let b = banks();
    let s = parse_statement(&b["Axess Business"], &fixture("akbank.txt")).unwrap();
    assert_eq!(s.bank, "Axess Business");
    // Business statements mask the whole card number -> fallback label.
    assert_eq!(s.card_last4, None);
    assert_eq!(s.card_masked.as_deref(), Some("Axess Business"));
    // US-formatted amounts (1,234.56).
    assert_eq!(s.total_due, 12345.67);
    assert_eq!(s.min_due, Some(3086.42));
    // "Son Ödeme Tarihi" must win over the later "Bir Sonraki Son Ödeme Tarihi".
    assert_eq!(s.due_date, "2026-12-05");
    assert_eq!(s.statement_date.as_deref(), Some("2026-11-30"));
}

#[test]
fn test_missing_required_returns_none() {
    let b = banks();
    assert!(parse_statement(&b["TEB"], "hiçbir şey yok").is_none());
}

#[test]
fn test_past_due_captured_silently() {
    // A past-due statement is pre-marked reminded; an upcoming one stays pending.
    let db = Database::open_in_memory().unwrap();
    let today = today_iso();
    let past = Statement {
        bank: "TEB".into(),
        card_last4: Some("1111".into()),
        card_masked: Some("m".into()),
        total_due: 10.0,
        min_due: Some(1.0),
        due_date: shift_days(&today, -5),
        statement_date: None,
    };
    let upcoming = Statement {
        card_last4: Some("2222".into()),
        due_date: shift_days(&today, 5),
        ..past.clone()
    };
    db.insert_statement(&past).unwrap();
    db.insert_statement(&upcoming).unwrap();

    let rows = db.latest_per_card().unwrap();
    let by_card: HashMap<_, _> = rows
        .into_iter()
        .map(|r| (r.card_last4.clone().unwrap(), r.reminded_at))
        .collect();
    assert!(by_card["1111"].is_some(), "already past due -> won't remind");
    assert!(by_card["2222"].is_none(), "upcoming -> will remind on due day");
}

#[test]
fn test_db_dedup() {
    let db = Database::open_in_memory().unwrap();
    let b = banks();
    let s = parse_statement(&b["TEB"], &fixture("teb.txt")).unwrap();
    assert!(db.insert_statement(&s).unwrap()); // first insert
    assert!(!db.insert_statement(&s).unwrap()); // ON CONFLICT DO NOTHING
    assert_eq!(db.latest_per_card().unwrap().len(), 1);
}

// --- date helpers (avoid a chrono dependency in tests) ---

/// Local date as `YYYY-MM-DD` via SQLite, matching the DB's own clock.
fn today_iso() -> String {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.query_row("SELECT date('now','localtime')", [], |r| r.get(0))
        .unwrap()
}

/// Shift an ISO date by `days` (may be negative) using SQLite date math.
fn shift_days(iso: &str, days: i64) -> String {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.query_row(
        "SELECT date(?1, ?2)",
        rusqlite::params![iso, format!("{days} days")],
        |r| r.get(0),
    )
    .unwrap()
}

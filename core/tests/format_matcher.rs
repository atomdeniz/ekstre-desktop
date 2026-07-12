//! Tests for the presentation and matching helpers, pinned to Python behavior
//! (`_fmt_amount`, `reminder_text`, `_matches`, `_strip_html`).

use ekstre_core::{
    body_text, days_left, format_amount_tr, header_matches, matches, reminder_body, reminder_text,
    reminder_title, strip_html, StatementRow,
};

fn row(min_due: Option<f64>, statement_date: Option<&str>) -> StatementRow {
    StatementRow {
        id: 1,
        bank: "TEB".into(),
        card_last4: Some("5678".into()),
        card_masked: Some("1234 **** **** 5678".into()),
        total_due: 12345.67,
        min_due,
        due_date: "2026-07-25".into(),
        statement_date: statement_date.map(|s| s.into()),
        reminded_at: None,
    }
}

#[test]
fn fmt_amount_turkish() {
    assert_eq!(format_amount_tr(1234.5), "1.234,50");
    assert_eq!(format_amount_tr(12345.67), "12.345,67");
    assert_eq!(format_amount_tr(987.65), "987,65");
    assert_eq!(format_amount_tr(5000.0), "5.000,00");
    assert_eq!(format_amount_tr(1234567.89), "1.234.567,89");
    assert_eq!(format_amount_tr(0.0), "0,00");
    assert_eq!(format_amount_tr(100.0), "100,00");
}

#[test]
fn reminder_text_full() {
    let t = reminder_text(&row(Some(1234.56), Some("2026-07-10")));
    assert_eq!(
        t,
        "💳 TEB kredi kartı — son ödeme günü!\n\
         Kart: 1234 **** **** 5678\n\
         Dönem borcu: 12.345,67 TL\n\
         Asgari: 1.234,56 TL\n\
         Hesap kesim: 2026-07-10\n\
         Son ödeme: 2026-07-25"
    );
}

#[test]
fn reminder_text_minimal() {
    // No min_due, no statement_date -> those lines are omitted.
    let t = reminder_text(&row(None, None));
    assert_eq!(
        t,
        "💳 TEB kredi kartı — son ödeme günü!\n\
         Kart: 1234 **** **** 5678\n\
         Dönem borcu: 12.345,67 TL\n\
         Son ödeme: 2026-07-25"
    );
}

#[test]
fn native_notification_text() {
    let r = row(Some(1234.56), Some("2026-07-10"));
    assert_eq!(reminder_title(&r), "💳 TEB — son ödeme günü");
    assert_eq!(
        reminder_body(&r),
        "1234 **** **** 5678 · Dönem borcu: 12.345,67 TL · Asgari: 1.234,56 TL · Son ödeme: 2026-07-25"
    );
    // Without min_due, the Asgari segment is dropped.
    assert_eq!(
        reminder_body(&row(None, None)),
        "1234 **** **** 5678 · Dönem borcu: 12.345,67 TL · Son ödeme: 2026-07-25"
    );
}

#[test]
fn days_left_math() {
    assert_eq!(days_left("2026-07-25", "2026-07-25"), Some(0));
    assert_eq!(days_left("2026-07-25", "2026-07-20"), Some(5));
    assert_eq!(days_left("2026-07-20", "2026-07-25"), Some(-5));
    assert_eq!(days_left("2026-01-01", "2025-12-31"), Some(1)); // year boundary
    assert_eq!(days_left("garbage", "2026-07-25"), None);
}

#[test]
fn header_matching() {
    // None / empty needle always matches (skip the check).
    assert!(header_matches(None, "anything"));
    assert!(header_matches(Some(""), "anything"));
    // Case-insensitive substring.
    assert!(header_matches(Some("teb.com.tr"), "TEB <noreply@TEB.COM.TR>"));
    assert!(!header_matches(Some("garanti"), "TEB <noreply@teb.com.tr>"));
    assert!(matches(
        Some("teb.com.tr"),
        Some("Ekstreniz"),
        "TEB <noreply@teb.com.tr>",
        "Kredi Kartı Ekstreniz hazır"
    ));
    assert!(!matches(
        Some("teb.com.tr"),
        Some("Ekstreniz"),
        "TEB <noreply@teb.com.tr>",
        "Kampanya"
    ));
}

#[test]
fn html_stripping() {
    assert_eq!(
        strip_html("<p>Dönem <b>Borcunuz</b>:  12.345,67</p>"),
        "Dönem Borcunuz : 12.345,67"
    );
    // Plain text preferred over HTML when present.
    assert_eq!(body_text("  plain body  ", "<p>html</p>"), "plain body");
    assert_eq!(body_text("   ", "<p>html body</p>"), "html body");
}

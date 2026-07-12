//! Presentation helpers: Turkish amount formatting, reminder text, days-left.

use crate::db::StatementRow;

/// Format a number as Turkish currency: thousands `.`, decimal `,`, 2 decimals.
/// `1234.5` -> `"1.234,50"`.
pub fn format_amount_tr(value: f64) -> String {
    let s = format!("{:.2}", value.abs());
    let (int_part, frac_part) = s.split_once('.').unwrap_or((&s, "00"));

    let n = int_part.len();
    let first = n % 3;
    let mut grouped = String::with_capacity(n + n / 3);
    if first > 0 {
        grouped.push_str(&int_part[..first]);
    }
    let mut i = first;
    while i < n {
        if !grouped.is_empty() {
            grouped.push('.');
        }
        grouped.push_str(&int_part[i..i + 3]);
        i += 3;
    }

    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{grouped},{frac_part}")
}

/// The Turkish reminder message for a due statement.
pub fn reminder_text(row: &StatementRow) -> String {
    let mut lines = vec![
        format!("💳 {} kredi kartı — son ödeme günü!", row.bank),
        format!("Kart: {}", row.card_masked.as_deref().unwrap_or("-")),
        format!("Dönem borcu: {} TL", format_amount_tr(row.total_due)),
    ];
    if let Some(min) = row.min_due {
        lines.push(format!("Asgari: {} TL", format_amount_tr(min)));
    }
    if let Some(sd) = &row.statement_date {
        if !sd.is_empty() {
            lines.push(format!("Hesap kesim: {sd}"));
        }
    }
    lines.push(format!("Son ödeme: {}", row.due_date));
    lines.join("\n")
}

/// Native-notification title for a due statement (short, single line).
pub fn reminder_title(row: &StatementRow) -> String {
    format!("💳 {} — son ödeme günü", row.bank)
}

pub fn reminder_body(row: &StatementRow) -> String {
    let mut parts = vec![
        format!("{}", row.card_masked.as_deref().unwrap_or("-")),
        format!("Dönem borcu: {} TL", format_amount_tr(row.total_due)),
    ];
    if let Some(min) = row.min_due {
        parts.push(format!("Asgari: {} TL", format_amount_tr(min)));
    }
    parts.push(format!("Son ödeme: {}", row.due_date));
    parts.join(" · ")
}

/// Days between `today` and an ISO `due_date` (`due - today`); `None` if unparseable.
/// Both args are ISO `YYYY-MM-DD`. Negative = overdue.
pub fn days_left(due_date: &str, today: &str) -> Option<i64> {
    Some(iso_to_ordinal(due_date)? - iso_to_ordinal(today)?)
}

/// Convert an ISO date to a day count (proleptic Gregorian ordinal) for diffing.
fn iso_to_ordinal(iso: &str) -> Option<i64> {
    let mut parts = iso.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Howard Hinnant's days-from-civil algorithm.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

//! IMAP polling: a read-only rolling-window scan (never sets `\Seen`), per-bank
//! From/Subject matching, and per-source text extraction (first PDF attachment,
//! or plain/HTML body).

use crate::banks::Bank;
use crate::matcher::{body_text, matches};
use crate::parser::{parse_statement, Statement};
use crate::pdf;
use mail_parser::MimeHeaders;
use pdfium_render::prelude::Pdfium;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub mailbox: String,
}

/// Build a day-granular IMAP `SINCE` token (`DD-Mon-YYYY`) `days` before `today`,
/// where `today` is `(year, month, day)`. English month names avoid locale issues.
/// Kept pure (no clock) so it is unit-testable; the caller passes today's date.
pub fn imap_since(today: (i32, u32, u32), days: i64) -> String {
    let (y, m, d) = shift_back(today, days);
    format!("{:02}-{}-{}", d, MONTHS[(m - 1) as usize], y)
}

/// Connect, scan every bank over the lookback window, and return parsed statements.
/// `pdfium` is required for banks whose `source` is `pdf`. `today` seeds the SINCE
/// date. Never marks mail read (uses `BODY.PEEK[]`).
pub fn scan(
    cfg: &ImapConfig,
    banks: &[Bank],
    lookback_days: i64,
    today: (i32, u32, u32),
    pdfium: Option<&Pdfium>,
) -> Result<Vec<Statement>, Box<dyn std::error::Error>> {
    let tls = native_tls::TlsConnector::builder().build()?;
    let client = imap::connect((cfg.host.as_str(), cfg.port), cfg.host.as_str(), &tls)?;
    let mut session = client.login(&cfg.user, &cfg.password).map_err(|(e, _)| e)?;
    session.select(&cfg.mailbox)?;

    let since = imap_since(today, lookback_days);
    let mut out = Vec::new();

    for bank in banks {
        let mut query = format!("SINCE {since}");
        // Only send the FROM filter server-side when ASCII (non-ASCII IMAP SEARCH
        // is unreliable); we always re-filter headers in Rust below.
        if let Some(f) = &bank.match_from {
            if f.is_ascii() {
                query.push_str(&format!(" FROM \"{f}\""));
            }
        }

        let uids = session.uid_search(&query)?;
        for uid in uids {
            let fetches = session.uid_fetch(uid.to_string(), "BODY.PEEK[]")?;
            for fetch in fetches.iter() {
                if let Some(body) = fetch.body() {
                    if let Some(stmt) = parse_message(bank, body, pdfium) {
                        out.push(stmt);
                    }
                }
            }
        }
    }

    let _ = session.logout();
    Ok(out)
}

/// Match one raw RFC822 message against a bank and parse it, or `None`.
fn parse_message(bank: &Bank, raw: &[u8], pdfium: Option<&Pdfium>) -> Option<Statement> {
    let msg = mail_parser::MessageParser::default().parse(raw)?;

    let from = header_text(msg.from());
    let subject = msg.subject().unwrap_or("");
    if !matches(
        bank.match_from.as_deref(),
        bank.match_subject.as_deref(),
        &from,
        subject,
    ) {
        return None;
    }

    let text = if bank.source == "pdf" {
        let pdfium = pdfium?;
        first_pdf_text(&msg, pdfium)?
    } else {
        let plain = msg.body_text(0).unwrap_or_default();
        let html = msg.body_html(0).unwrap_or_default();
        body_text(&plain, &html)
    };

    parse_statement(bank, &text)
}

fn first_pdf_text(msg: &mail_parser::Message, pdfium: &Pdfium) -> Option<String> {
    for att in msg.attachments() {
        let name_is_pdf = att
            .attachment_name()
            .map(|n| n.to_lowercase().ends_with(".pdf"))
            .unwrap_or(false);
        if att.is_content_type("application", "pdf") || name_is_pdf {
            if let Ok(text) = pdf::extract_text(pdfium, att.contents()) {
                return Some(text);
            }
        }
    }
    None
}

/// Flatten an address header into a searchable "Name <addr>" string.
fn header_text(addr: Option<&mail_parser::Address>) -> String {
    let mut s = String::new();
    if let Some(addr) = addr {
        for a in addr.iter() {
            if let Some(name) = a.name() {
                s.push_str(name);
                s.push(' ');
            }
            if let Some(email) = a.address() {
                s.push_str(email);
                s.push(' ');
            }
        }
    }
    s
}

/// Subtract `days` from a `(year, month, day)` date. Small helper to avoid a
/// chrono dependency in core; correct across month/year boundaries.
fn shift_back(today: (i32, u32, u32), days: i64) -> (i32, u32, u32) {
    let mut n = civil_to_days(today) - days;
    // days_to_civil handles any integer day count.
    let _ = &mut n;
    days_to_civil(n)
}

fn civil_to_days((y, m, d): (i32, u32, u32)) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let m = m as i64;
    let d = d as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn days_to_civil(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        (if m <= 2 { y + 1 } else { y }) as i32,
        m as u32,
        d as u32,
    )
}

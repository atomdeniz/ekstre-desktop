//! Header matching and HTML stripping used by the IMAP scan. Pure helpers so
//! they can be tested without a mail server.

use regex::Regex;
use std::sync::OnceLock;

/// Case-insensitive substring test. `None`/empty needle always matches (skip the
/// check).
pub fn header_matches(needle: Option<&str>, haystack: &str) -> bool {
    match needle {
        None => true,
        Some(n) if n.is_empty() => true,
        Some(n) => haystack.to_lowercase().contains(&n.to_lowercase()),
    }
}

pub fn matches(
    match_from: Option<&str>,
    match_subject: Option<&str>,
    from_header: &str,
    subject_header: &str,
) -> bool {
    header_matches(match_from, from_header) && header_matches(match_subject, subject_header)
}

/// Strip HTML tags and collapse whitespace to single spaces, then trim.
pub fn strip_html(html: &str) -> String {
    let tag = tag_re().replace_all(html, " ");
    ws_re().replace_all(&tag, " ").trim().to_string()
}

/// Pick body text: prefer non-empty plain text, else strip the HTML part.
pub fn body_text(plain: &str, html: &str) -> String {
    let p = plain.trim();
    if !p.is_empty() {
        p.to_string()
    } else {
        strip_html(html)
    }
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn ws_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

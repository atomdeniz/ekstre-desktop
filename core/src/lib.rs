//! Ekstre Desktop core library: per-bank regex parsing of Turkish credit-card
//! statements, SQLite storage with dedup, and reminder-eligibility rules. The
//! Tauri shell depends on this crate.

pub mod banks;
pub mod db;
pub mod format;
pub mod imap;
pub mod matcher;
pub mod parser;
pub mod pdf;

pub use banks::{builtin_banks, load_banks_str, Bank};
pub use db::{Database, StatementRow};
pub use format::{
    days_left, format_amount_tr, reminder_body, reminder_text, reminder_title, reminder_title_lead,
};
pub use matcher::{body_text, header_matches, matches, strip_html};
pub use parser::{parse_amount, parse_amount_us, parse_date, parse_statement, Statement};

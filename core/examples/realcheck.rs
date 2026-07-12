//! Dev utility: strip an HTML email body from a file and try to parse it with a
//! named built-in bank (forced through the body/HTML path). Not a test; used to
//! validate the parser against real emails without committing real data.
//!
//!   cargo run -p ekstre-core --example realcheck -- <bank-name> <html-file>

use std::env;
use std::fs;

use ekstre_core::{builtin_banks, parse_statement, strip_html};

fn main() {
    let mut args = env::args().skip(1);
    let bank_name = args.next().expect("usage: realcheck <bank-name> <html-file>");
    let path = args.next().expect("usage: realcheck <bank-name> <html-file>");

    let raw = fs::read_to_string(&path).expect("read html file");
    let text = strip_html(&raw);
    println!("--- stripped text ---\n{text}\n---------------------");

    let banks = builtin_banks();
    let bank = banks
        .iter()
        .find(|b| b.name == bank_name)
        .unwrap_or_else(|| panic!("no built-in bank named {bank_name:?}"));

    match parse_statement(bank, &text) {
        Some(s) => println!("PARSED: {s:#?}"),
        None => println!("PARSE FAILED (total_due and/or due_date did not match)"),
    }
}

//! Dev utility: extract text from a PDF via pdfium, then optionally try to parse
//! it with a named built-in bank. Used to validate real bank PDFs against the
//! field regexes without committing real data.
//!
//!   cargo run -p ekstre-core --example pdfcheck -- <pdf-file> [bank-name]
//!
//! Set PDFIUM_LIB_DIR to the folder holding libpdfium (default: vendor/pdfium/lib).

use std::env;
use std::fs;

use ekstre_core::pdf;
use ekstre_core::{builtin_banks, parse_statement};

fn main() {
    let mut args = env::args().skip(1);
    let pdf_path = args.next().expect("usage: pdfcheck <pdf-file> [bank-name]");
    let bank_name = args.next();

    let lib_dir = env::var("PDFIUM_LIB_DIR").unwrap_or_else(|_| "vendor/pdfium/lib".into());
    let pdfium = pdf::bind_pdfium(&lib_dir).expect("bind pdfium (check PDFIUM_LIB_DIR)");

    let bytes = fs::read(&pdf_path).expect("read pdf");
    let text = pdf::extract_text(&pdfium, &bytes).expect("extract text");
    println!("--- extracted text ---\n{text}\n----------------------");

    if let Some(name) = bank_name {
        let banks = builtin_banks();
        let bank = banks
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("no built-in bank named {name:?}"));
        match parse_statement(bank, &text) {
            Some(s) => println!("PARSED: {s:#?}"),
            None => println!("PARSE FAILED (total_due and/or due_date did not match)"),
        }
    }
}

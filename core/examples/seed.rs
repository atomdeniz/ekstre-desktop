//! Dev utility: parse every PDF in a directory against the built-in banks,
//! insert into a DB, then print the dashboard view-model (what `get_statements`
//! would return). Verifies the full pipeline end-to-end without the GUI.
//!
//!   cargo run -p ekstre-core --example seed -- <db-path> <samples-dir>

use std::{env, fs};

use ekstre_core::{
    builtin_banks, days_left, format_amount_tr, parse_statement, pdf, Database,
};

fn main() {
    let mut args = env::args().skip(1);
    let db_path = args.next().expect("usage: seed <db-path> <samples-dir>");
    let dir = args.next().expect("usage: seed <db-path> <samples-dir>");

    let db = Database::open(&db_path).expect("open db");
    let banks = builtin_banks();
    let lib_dir = env::var("PDFIUM_LIB_DIR").unwrap_or_else(|_| "vendor/pdfium/lib".into());
    let pdfium = pdf::bind_pdfium(&lib_dir).expect("bind pdfium");

    let mut added = 0;
    for entry in fs::read_dir(&dir).expect("read dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("pdf")) != Some(true) {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        let text = match pdf::extract_text(&pdfium, &bytes) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for bank in &banks {
            if let Some(stmt) = parse_statement(bank, &text) {
                if db.insert_statement(&stmt).unwrap() {
                    added += 1;
                }
                break;
            }
        }
    }
    println!("inserted {added} statement(s)\n");

    let today = db.today_local().unwrap();
    let colors: std::collections::HashMap<_, _> =
        banks.iter().map(|b| (b.name.clone(), b.color.clone())).collect();
    for r in db.latest_per_card().unwrap() {
        let dl = days_left(&r.due_date, &today);
        println!(
            "{:<12} {:<22} borç {:>12} TL  asgari {:>10}  son ödeme {}  ({} gün)  {}",
            r.bank,
            r.card_masked.as_deref().unwrap_or("-"),
            format_amount_tr(r.total_due),
            r.min_due.map(format_amount_tr).unwrap_or_else(|| "-".into()),
            r.due_date,
            dl.map(|d| d.to_string()).unwrap_or_else(|| "?".into()),
            colors.get(&r.bank).cloned().unwrap_or_default(),
        );
    }
}

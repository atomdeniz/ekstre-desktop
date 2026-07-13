use crate::banks::{AmountFormat, Bank};

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub bank: String,
    pub card_last4: Option<String>,
    pub card_masked: Option<String>,
    pub total_due: f64,
    pub min_due: Option<f64>,
    /// ISO `YYYY-MM-DD`.
    pub due_date: String,
    /// ISO `YYYY-MM-DD`; the "hesap kesim tarihi". Not all banks send it.
    pub statement_date: Option<String>,
}

/// Turkish-formatted amount to `f64`: strip thousands `.`, drop the `",-"`
/// round-amount shorthand (means `",00"`, e.g. `"24.396,-"`), swap decimal
/// `,` -> `.`. `"12.345,67"` -> `12345.67`, `"24.396,-"` -> `24396.0`. The caller
/// only feeds regex-captured digit groups, so parsing always succeeds.
pub fn parse_amount(raw: &str) -> f64 {
    raw.replace('.', "")
        .replace(",-", "")
        .replace(',', ".")
        .parse()
        .unwrap()
}

/// US-formatted amount to `f64`: strip thousands `,`, keep the decimal `.`.
/// `"10,964.11"` -> `10964.11`. Akbank's business statements use this format.
pub fn parse_amount_us(raw: &str) -> f64 {
    raw.replace(',', "").parse().unwrap()
}

/// Three groups (day, month, year) to zero-padded ISO `YYYY-MM-DD`. `month` is a
/// number or a Turkish month name (see `month_to_num`).
pub fn parse_date(day: &str, month: &str, year: &str) -> String {
    let d: u32 = day.parse().unwrap();
    let m: u32 = month_to_num(month);
    let y: i32 = year.parse().unwrap();
    format!("{y:04}-{m:02}-{d:02}")
}

/// Numeric month string passes through; a Turkish month name is mapped to its
/// number (Yapı Kredi writes dates as "7 Mayıs 2026").
fn month_to_num(month: &str) -> u32 {
    if let Ok(n) = month.parse::<u32>() {
        return n;
    }
    match month.to_lowercase().as_str() {
        "ocak" => 1, "şubat" => 2, "mart" => 3, "nisan" => 4,
        "mayıs" => 5, "haziran" => 6, "temmuz" => 7, "ağustos" => 8,
        "eylül" => 9, "ekim" => 10, "kasım" => 11, "aralık" => 12,
        other => panic!("bilinmeyen Türkçe ay: {other}"),
    }
}

/// Parse `text` against a bank's field regexes.
///
/// A statement is valid **iff both `total_due` and `due_date` match**; otherwise
/// `None`. All other fields are optional. Group contract per field:
/// `card` -> (first4, last4); `total_due`/`min_due` -> 1 amount group;
/// `due_date`/`statement_date` -> 3 date groups.
pub fn parse_statement(bank: &Bank, text: &str) -> Option<Statement> {
    let total_m = bank.fields.get("total_due").and_then(|r| r.captures(text))?;
    let due_m = bank.fields.get("due_date").and_then(|r| r.captures(text))?;

    let min_m = bank.fields.get("min_due").and_then(|r| r.captures(text));
    let card_m = bank.fields.get("card").and_then(|r| r.captures(text));
    let stmt_m = bank.fields.get("statement_date").and_then(|r| r.captures(text));

    let amount = |raw: &str| match bank.amount_format {
        AmountFormat::Us => parse_amount_us(raw),
        AmountFormat::Tr => parse_amount(raw),
    };

    let (card_last4, card_masked) = match card_m {
        Some(c) => {
            let first4 = c.get(1).unwrap().as_str();
            let last4 = c.get(2).unwrap().as_str();
            (
                Some(last4.to_string()),
                Some(format!("{first4} **** **** {last4}")),
            )
        }
        None => (None, bank.fallback_masked.clone()),
    };

    Some(Statement {
        bank: bank.name.clone(),
        card_last4,
        card_masked,
        total_due: amount(total_m.get(1).unwrap().as_str()),
        min_due: min_m.map(|m| amount(m.get(1).unwrap().as_str())),
        due_date: parse_date(
            due_m.get(1).unwrap().as_str(),
            due_m.get(2).unwrap().as_str(),
            due_m.get(3).unwrap().as_str(),
        ),
        statement_date: stmt_m.map(|m| {
            parse_date(
                m.get(1).unwrap().as_str(),
                m.get(2).unwrap().as_str(),
                m.get(3).unwrap().as_str(),
            )
        }),
    })
}

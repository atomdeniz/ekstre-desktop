use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use std::collections::HashMap;

/// The built-in bank pack, embedded at compile time. Users select from these in
/// the setup wizard; new banks are added here via community PRs.
const BUILTIN_YML: &str = include_str!("../banks/banks.yml");

/// A bank definition with compiled, case-insensitive field regexes.
#[derive(Debug, Clone)]
pub struct Bank {
    pub name: String,
    pub color: String,
    /// `"body"` or `"pdf"`.
    pub source: String,
    pub match_from: Option<String>,
    pub match_subject: Option<String>,
    pub fallback_masked: Option<String>,
    pub fields: HashMap<String, Regex>,
}

#[derive(Deserialize)]
struct RawBanks {
    #[serde(default)]
    banks: Vec<RawBank>,
}

#[derive(Deserialize)]
struct RawMatch {
    from: Option<String>,
    subject: Option<String>,
}

#[derive(Deserialize)]
struct RawBank {
    name: String,
    color: Option<String>,
    source: Option<String>,
    #[serde(rename = "match")]
    match_field: Option<RawMatch>,
    fallback_masked: Option<String>,
    #[serde(default)]
    fields: HashMap<String, String>,
}

/// Parse a `banks.yml` document. Each field regex is compiled case-insensitively.
/// Defaults: `color=#666666`, `source=body`.
pub fn load_banks_str(yaml: &str) -> Result<Vec<Bank>, Box<dyn std::error::Error>> {
    let raw: RawBanks = serde_yaml::from_str(yaml)?;
    let mut banks = Vec::with_capacity(raw.banks.len());
    for b in raw.banks {
        let mut fields = HashMap::with_capacity(b.fields.len());
        for (k, v) in b.fields {
            let re = RegexBuilder::new(&v).case_insensitive(true).build()?;
            fields.insert(k, re);
        }
        let (match_from, match_subject) = match b.match_field {
            Some(m) => (m.from, m.subject),
            None => (None, None),
        };
        banks.push(Bank {
            name: b.name,
            color: b.color.unwrap_or_else(|| "#666666".to_string()),
            source: b.source.unwrap_or_else(|| "body".to_string()),
            match_from,
            match_subject,
            fallback_masked: b.fallback_masked,
            fields,
        });
    }
    Ok(banks)
}

/// Load the compiled-in built-in bank pack. Panics only if the embedded pack is
/// malformed, which a test guards against.
pub fn builtin_banks() -> Vec<Bank> {
    load_banks_str(BUILTIN_YML).expect("built-in banks.yml is valid")
}

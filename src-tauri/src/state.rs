//! Application state: the database, bank pack, pdfium binding, and the poll
//! orchestration (IMAP scan -> parse -> store). Held in Tauri managed state.

use std::collections::HashMap;
use std::path::PathBuf;

use ekstre_core::banks::Bank;
use ekstre_core::imap::{scan, ImapConfig, Scanned};
use ekstre_core::{builtin_banks, Database};
use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "com.denizozogul.ekstre";

/// One IMAP account (non-secret part; the password lives in the OS keychain,
/// keyed by `user`). Stored as a JSON array under the `accounts` settings key.
#[derive(Serialize, Deserialize, Clone)]
pub struct ImapAccount {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub mailbox: String,
}

/// Non-secret runtime settings, persisted in the DB `settings` table.
pub struct Config {
    pub accounts: Vec<ImapAccount>,
    pub lookback_days: i64,
    pub poll_interval_min: u64,
    pub reminder_hour: u32,
    /// How many days before the due date to fire the reminder (0 = on the due day).
    pub reminder_days_before: i64,
    /// Selected bank names; empty means "all built-in banks".
    pub selected_banks: Vec<String>,
    /// Card keys (`bank|last4`) hidden from the dashboard and reminders.
    /// An opt-out list, so newly discovered cards default to enabled.
    pub disabled_cards: Vec<String>,
    /// Start the app automatically on login. Defaults on.
    pub launch_at_login: bool,
}

impl Config {
    fn get(db: &Database, key: &str, default: &str) -> String {
        db.get_setting(key).ok().flatten().unwrap_or_else(|| default.to_string())
    }

    pub fn load(db: &Database) -> Self {
        let selected = Self::get(db, "selected_banks", "");
        let selected_banks = selected
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Config {
            accounts: Self::load_accounts(db),
            disabled_cards: serde_json::from_str(&Self::get(db, "disabled_cards", "[]"))
                .unwrap_or_default(),
            lookback_days: Self::get(db, "lookback_days", "45").parse().unwrap_or(45),
            poll_interval_min: Self::get(db, "poll_interval_min", "15").parse().unwrap_or(15),
            reminder_hour: Self::get(db, "reminder_hour", "9").parse().unwrap_or(9),
            reminder_days_before: Self::get(db, "reminder_days_before", "3").parse().unwrap_or(3),
            selected_banks,
            launch_at_login: Self::get(db, "launch_at_login", "true") != "false",
        }
    }

    /// Accounts from the `accounts` JSON key. The single-account `imap_*` keys
    /// written by pre-multi-account versions are only consulted while the
    /// `accounts` key has never been written — once it exists it is the truth,
    /// even as an empty list (all accounts removed).
    fn load_accounts(db: &Database) -> Vec<ImapAccount> {
        if let Some(raw) = db.get_setting("accounts").ok().flatten() {
            return serde_json::from_str(&raw).unwrap_or_default();
        }
        let user = Self::get(db, "imap_user", "");
        if user.is_empty() {
            return Vec::new();
        }
        vec![ImapAccount {
            host: Self::get(db, "imap_host", "imap.gmail.com"),
            port: Self::get(db, "imap_port", "993").parse().unwrap_or(993),
            user,
            mailbox: Self::get(db, "imap_mailbox", "INBOX"),
        }]
    }

    pub fn is_configured(&self) -> bool {
        !self.accounts.is_empty()
    }

    pub fn card_key(bank: &str, last4: Option<&str>) -> String {
        format!("{bank}|{}", last4.unwrap_or(""))
    }

    pub fn is_card_enabled(&self, bank: &str, last4: Option<&str>) -> bool {
        let key = Self::card_key(bank, last4);
        !self.disabled_cards.iter().any(|k| k == &key)
    }
}

pub struct AppState {
    pub db: Database,
    pub banks: Vec<Bank>,
    /// Directory holding libpdfium; bound fresh per poll (pdfium is not Send).
    pub pdfium_lib_dir: Option<String>,
    /// OS app-data dir; holds `ekstre.db` and the `statements/` PDF store.
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf, resource_dir: Option<PathBuf>) -> Self {
        let db_path = data_dir.join("ekstre.db");
        let db = Database::open(db_path.to_string_lossy().as_ref())
            .expect("open database");
        AppState {
            db,
            banks: builtin_banks(),
            pdfium_lib_dir: find_pdfium_dir(resource_dir),
            data_dir,
        }
    }

    /// Directory where scanned statement PDFs are stored, one file per row id
    /// (`<id>.pdf`). Created on demand.
    pub fn statements_dir(&self) -> PathBuf {
        let dir = self.data_dir.join("statements");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn statement_pdf_path(&self, id: i64) -> PathBuf {
        self.statements_dir().join(format!("{id}.pdf"))
    }

    /// Map of bank name -> dashboard color.
    pub fn colors(&self) -> HashMap<String, String> {
        self.banks.iter().map(|b| (b.name.clone(), b.color.clone())).collect()
    }

    /// The banks to scan given the config selection (empty selection = all).
    fn active_banks(&self, cfg: &Config) -> Vec<Bank> {
        if cfg.selected_banks.is_empty() {
            self.banks.clone()
        } else {
            self.banks
                .iter()
                .filter(|b| cfg.selected_banks.iter().any(|s| s == &b.name))
                .cloned()
                .collect()
        }
    }

    /// The IMAP password for an account. The dev env var wins over the keychain:
    /// dev builds are re-signed on every compile, so a keychain read would prompt
    /// for the login password each time. Applies to every account when set.
    fn imap_password(&self, user: &str) -> Option<String> {
        if let Ok(pw) = std::env::var("EKSTRE_IMAP_PASSWORD") {
            if !pw.is_empty() {
                return Some(pw);
            }
        }
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, user) {
            if let Ok(pw) = entry.get_password() {
                if !pw.is_empty() {
                    return Some(pw);
                }
            }
        }
        None
    }

    /// Store the IMAP password in the OS keychain, keyed by the account user.
    pub fn set_imap_password(&self, user: &str, password: &str) -> Result<(), String> {
        keyring::Entry::new(KEYRING_SERVICE, user)
            .and_then(|e| e.set_password(password))
            .map_err(|e| format!("parola kaydedilemedi: {e}"))
    }

    /// Remove an account's keychain entry. Best-effort: a missing entry is fine.
    pub fn delete_imap_password(&self, user: &str) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, user) {
            let _ = entry.delete_credential();
        }
    }

    /// Persist the account list as JSON under the `accounts` settings key.
    pub fn save_accounts(&self, accounts: &[ImapAccount]) -> Result<(), String> {
        let json = serde_json::to_string(accounts).map_err(|e| e.to_string())?;
        self.db.set_setting("accounts", &json).map_err(|e| e.to_string())
    }

    /// Connect with the given settings and count parseable statements over the
    /// last 90 days. Powers the wizard's "test connection" button. Does not store.
    pub fn test_scan(&self, cfg: &ImapConfig) -> Result<usize, String> {
        let today = parse_iso(&self.db.today_local().map_err(|e| e.to_string())?)
            .ok_or("bugünün tarihi okunamadı")?;
        let pdfium = self
            .pdfium_lib_dir
            .as_ref()
            .and_then(|dir| ekstre_core::pdf::bind_pdfium(dir).ok());
        let found = scan(cfg, &self.banks, 90, today, pdfium.as_ref())
            .map_err(|e| format!("Bağlantı başarısız: {e}"))?;
        Ok(found.len())
    }

    /// Run one poll: scan every account's mailbox, parse, and store. Returns rows
    /// added. A failing account is skipped (logged) as long as at least one
    /// account succeeds; errors only surface when every account fails.
    pub fn run_poll(&self) -> Result<usize, String> {
        self.run_poll_with_lookback(None)
    }

    /// Like `run_poll`, but `lookback_days` overrides the configured window.
    /// Backfilled past-due statements are pre-marked reminded on insert, so a
    /// deep scan never floods notifications.
    pub fn run_poll_with_lookback(&self, lookback_days: Option<i64>) -> Result<usize, String> {
        let cfg = Config::load(&self.db);
        if !cfg.is_configured() {
            return Err("E-posta hesabı henüz ayarlanmadı.".into());
        }
        let lookback = lookback_days.unwrap_or(cfg.lookback_days);
        let today = parse_iso(&self.db.today_local().map_err(|e| e.to_string())?)
            .ok_or("bugünün tarihi okunamadı")?;

        let banks = self.active_banks(&cfg);
        // Bind pdfium locally on this (poll) thread — it is not Send/Sync.
        let pdfium = self
            .pdfium_lib_dir
            .as_ref()
            .and_then(|dir| ekstre_core::pdf::bind_pdfium(dir).ok());

        let mut added = 0;
        let mut succeeded = 0;
        let mut errors: Vec<String> = Vec::new();
        for acct in &cfg.accounts {
            let Some(password) = self.imap_password(&acct.user) else {
                errors.push(format!("{}: IMAP parolası ayarlı değil", acct.user));
                continue;
            };
            let imap_cfg = ImapConfig {
                host: acct.host.clone(),
                port: acct.port,
                user: acct.user.clone(),
                password,
                mailbox: acct.mailbox.clone(),
            };
            match scan(&imap_cfg, &banks, lookback, today, pdfium.as_ref()) {
                Ok(scanned) => {
                    succeeded += 1;
                    for item in &scanned {
                        if self.db.insert_statement(&item.statement).map_err(|e| e.to_string())? {
                            added += 1;
                        }
                        self.store_pdf(&item);
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", acct.user)),
            }
        }
        for e in &errors {
            log::warn!("account scan failed: {e}");
        }
        if succeeded == 0 {
            return Err(format!("IMAP taraması başarısız: {}", errors.join(" · ")));
        }
        Ok(added)
    }

    /// Persist a scanned statement's PDF as `statements/<id>.pdf`, if we have the
    /// bytes and no file yet. Runs whether the row was newly inserted or deduped,
    /// so re-scanning backfills PDFs for statements captured before this feature.
    /// Best-effort: a write failure is logged, never fatal to the poll.
    fn store_pdf(&self, item: &Scanned) {
        let Some(bytes) = &item.pdf else { return };
        let id = match self.db.statement_id(&item.statement) {
            Ok(Some(id)) => id,
            Ok(None) => return,
            Err(e) => {
                log::warn!("statement_id lookup failed: {e}");
                return;
            }
        };
        let path = self.statement_pdf_path(id);
        if !path.exists() {
            if let Err(e) = std::fs::write(&path, bytes) {
                log::warn!("failed to store statement PDF {}: {e}", path.display());
            }
        }
    }
}

/// Find a directory containing libpdfium (bundled resources in a shipped app, or
/// the dev vendor dir). Returns the first candidate that binds successfully.
fn find_pdfium_dir(resource_dir: Option<PathBuf>) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(env) = std::env::var("PDFIUM_LIB_DIR") {
        if !env.is_empty() {
            candidates.push(env);
        }
    }
    if let Some(res) = resource_dir {
        candidates.push(res.join("pdfium").to_string_lossy().into_owned());
    }
    // Bundled release: libpdfium is signed into Contents/Frameworks (../Frameworks
    // relative to the executable in Contents/MacOS).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(contents.join("Frameworks").to_string_lossy().into_owned());
        }
    }
    candidates.push("vendor/pdfium/lib".into());
    candidates.push("../vendor/pdfium/lib".into());

    for dir in &candidates {
        if ekstre_core::pdf::bind_pdfium(dir).is_ok() {
            return Some(dir.clone());
        }
    }
    log::warn!("libpdfium not found; PDF-source banks will not parse");
    None
}

fn parse_iso(iso: &str) -> Option<(i32, u32, u32)> {
    let mut p = iso.split('-');
    let y = p.next()?.parse().ok()?;
    let m = p.next()?.parse().ok()?;
    let d = p.next()?.parse().ok()?;
    Some((y, m, d))
}

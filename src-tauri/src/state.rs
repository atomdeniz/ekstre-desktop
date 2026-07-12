//! Application state: the database, bank pack, pdfium binding, and the poll
//! orchestration (IMAP scan -> parse -> store). Held in Tauri managed state.

use std::collections::HashMap;
use std::path::PathBuf;

use ekstre_core::banks::Bank;
use ekstre_core::imap::{scan, ImapConfig};
use ekstre_core::{builtin_banks, Database};

const KEYRING_SERVICE: &str = "com.denizozogul.ekstre";

/// Non-secret runtime settings, persisted in the DB `settings` table.
pub struct Config {
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_user: String,
    pub imap_mailbox: String,
    pub lookback_days: i64,
    pub poll_interval_min: u64,
    pub reminder_hour: u32,
    /// How many days before the due date to fire the reminder (0 = on the due day).
    pub reminder_days_before: i64,
    /// Selected bank names; empty means "all built-in banks".
    pub selected_banks: Vec<String>,
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
            imap_host: Self::get(db, "imap_host", "imap.gmail.com"),
            imap_port: Self::get(db, "imap_port", "993").parse().unwrap_or(993),
            imap_user: Self::get(db, "imap_user", ""),
            imap_mailbox: Self::get(db, "imap_mailbox", "INBOX"),
            lookback_days: Self::get(db, "lookback_days", "45").parse().unwrap_or(45),
            poll_interval_min: Self::get(db, "poll_interval_min", "15").parse().unwrap_or(15),
            reminder_hour: Self::get(db, "reminder_hour", "9").parse().unwrap_or(9),
            reminder_days_before: Self::get(db, "reminder_days_before", "3").parse().unwrap_or(3),
            selected_banks,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.imap_user.is_empty()
    }
}

pub struct AppState {
    pub db: Database,
    pub banks: Vec<Bank>,
    /// Directory holding libpdfium; bound fresh per poll (pdfium is not Send).
    pub pdfium_lib_dir: Option<String>,
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
        }
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

    /// The IMAP password from the OS keychain (falls back to an env var for dev).
    fn imap_password(&self, user: &str) -> Option<String> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, user) {
            if let Ok(pw) = entry.get_password() {
                if !pw.is_empty() {
                    return Some(pw);
                }
            }
        }
        std::env::var("EKSTRE_IMAP_PASSWORD").ok().filter(|s| !s.is_empty())
    }

    /// Store the IMAP password in the OS keychain, keyed by the account user.
    pub fn set_imap_password(&self, user: &str, password: &str) -> Result<(), String> {
        keyring::Entry::new(KEYRING_SERVICE, user)
            .and_then(|e| e.set_password(password))
            .map_err(|e| format!("parola kaydedilemedi: {e}"))
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

    /// Run one poll: scan the mailbox, parse, and store. Returns rows added.
    pub fn run_poll(&self) -> Result<usize, String> {
        let cfg = Config::load(&self.db);
        if !cfg.is_configured() {
            return Err("E-posta hesabı henüz ayarlanmadı.".into());
        }
        let password = self
            .imap_password(&cfg.imap_user)
            .ok_or("IMAP parolası ayarlı değil.")?;
        let today = parse_iso(&self.db.today_local().map_err(|e| e.to_string())?)
            .ok_or("bugünün tarihi okunamadı")?;

        let imap_cfg = ImapConfig {
            host: cfg.imap_host.clone(),
            port: cfg.imap_port,
            user: cfg.imap_user.clone(),
            password,
            mailbox: cfg.imap_mailbox.clone(),
        };
        let banks = self.active_banks(&cfg);
        // Bind pdfium locally on this (poll) thread — it is not Send/Sync.
        let pdfium = self
            .pdfium_lib_dir
            .as_ref()
            .and_then(|dir| ekstre_core::pdf::bind_pdfium(dir).ok());
        let statements = scan(
            &imap_cfg,
            &banks,
            cfg.lookback_days,
            today,
            pdfium.as_ref(),
        )
        .map_err(|e| format!("IMAP taraması başarısız: {e}"))?;

        let mut added = 0;
        for s in &statements {
            if self.db.insert_statement(s).map_err(|e| e.to_string())? {
                added += 1;
            }
        }
        Ok(added)
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

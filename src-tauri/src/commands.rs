//! Tauri IPC commands invoked from the webview. These build the dashboard
//! view-model and expose settings + actions.

use ekstre_core::imap::ImapConfig;
use ekstre_core::{days_left, format_amount_tr};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::state::{AppState, Config};

/// IMAP form fields sent from the setup wizard.
#[derive(Deserialize, Clone)]
pub struct ImapForm {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub mailbox: String,
}

impl From<ImapForm> for ImapConfig {
    fn from(f: ImapForm) -> Self {
        ImapConfig {
            host: f.host,
            port: f.port,
            user: f.user,
            password: f.password,
            mailbox: f.mailbox,
        }
    }
}

/// One dashboard card.
#[derive(Serialize)]
pub struct CardView {
    pub id: i64,
    pub bank: String,
    pub card_masked: Option<String>,
    pub total_due_fmt: String,
    pub min_due_fmt: Option<String>,
    pub due_date: String,
    pub statement_date: Option<String>,
    pub days_left: Option<i64>,
    pub color: String,
}

fn to_card_view(
    r: ekstre_core::db::StatementRow,
    today: &str,
    colors: &std::collections::HashMap<String, String>,
) -> CardView {
    CardView {
        id: r.id,
        days_left: days_left(&r.due_date, today),
        color: colors.get(&r.bank).cloned().unwrap_or_else(|| "#666666".into()),
        total_due_fmt: format_amount_tr(r.total_due),
        min_due_fmt: r.min_due.map(format_amount_tr),
        bank: r.bank,
        card_masked: r.card_masked,
        due_date: r.due_date,
        statement_date: r.statement_date,
    }
}

/// Latest statement per card, shaped for the dashboard.
#[tauri::command]
pub fn get_statements(state: State<AppState>) -> Result<Vec<CardView>, String> {
    let today = state.db.today_local().map_err(|e| e.to_string())?;
    let colors = state.colors();
    let rows = state.db.latest_per_card().map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| to_card_view(r, &today, &colors)).collect())
}

/// Every stored statement, shaped like dashboard cards. Backs the calendar view.
#[tauri::command]
pub fn get_calendar(state: State<AppState>) -> Result<Vec<CardView>, String> {
    let today = state.db.today_local().map_err(|e| e.to_string())?;
    let colors = state.colors();
    let rows = state.db.all_statements().map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| to_card_view(r, &today, &colors)).collect())
}

/// Whether the app has been configured (an email account is set).
#[tauri::command]
pub fn is_configured(state: State<AppState>) -> bool {
    Config::load(&state.db).is_configured()
}

/// The built-in bank list, for the wizard's checkboxes.
#[derive(Serialize)]
pub struct BankInfo {
    pub name: String,
    pub color: String,
}

#[tauri::command]
pub fn list_banks(state: State<AppState>) -> Vec<BankInfo> {
    state
        .banks
        .iter()
        .map(|b| BankInfo {
            name: b.name.clone(),
            color: b.color.clone(),
        })
        .collect()
}

/// Run a poll now, returning the number of newly stored statements. The blocking
/// IMAP work runs off the UI thread.
#[tauri::command]
pub async fn poll_now(app: AppHandle) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().run_poll())
        .await
        .map_err(|e| e.to_string())?
}

/// Test an IMAP connection and count statements found over the last 90 days.
/// Powers the wizard's "test connection" button.
#[tauri::command]
pub async fn test_imap(app: AppHandle, form: ImapForm) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().test_scan(&form.into())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn complete_setup(
    state: State<AppState>,
    form: ImapForm,
    selected_banks: Vec<String>,
) -> Result<(), String> {
    state.set_imap_password(&form.user, &form.password)?;
    let settings = [
        ("imap_host", form.host.clone()),
        ("imap_port", form.port.to_string()),
        ("imap_user", form.user.clone()),
        ("imap_mailbox", form.mailbox.clone()),
        ("selected_banks", selected_banks.join(",")),
    ];
    for (k, v) in settings {
        state.db.set_setting(k, &v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Update settings from the settings screen. A blank password keeps the existing
/// keychain entry untouched; only a typed password overwrites it.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    form: ImapForm,
    selected_banks: Vec<String>,
    reminder_days_before: i64,
    launch_at_login: bool,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    if !form.password.is_empty() {
        state.set_imap_password(&form.user, &form.password)?;
    }
    let settings = [
        ("imap_host", form.host.clone()),
        ("imap_port", form.port.to_string()),
        ("imap_user", form.user.clone()),
        ("imap_mailbox", form.mailbox.clone()),
        ("selected_banks", selected_banks.join(",")),
        ("reminder_days_before", reminder_days_before.max(0).to_string()),
        ("launch_at_login", launch_at_login.to_string()),
    ];
    for (k, v) in settings {
        state.db.set_setting(k, &v).map_err(|e| e.to_string())?;
    }
    set_autolaunch(&app, launch_at_login);
    Ok(())
}

/// Apply the login-item setting to the OS. No-op in dev builds, which never
/// register a login item (matches the startup sync in `run()`).
fn set_autolaunch(_app: &AppHandle, _enabled: bool) {
    #[cfg(not(debug_assertions))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = _app.autolaunch();
        let _ = if _enabled { launcher.enable() } else { launcher.disable() };
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<std::collections::HashMap<String, String>, String> {
    let pairs = state.db.all_settings().map_err(|e| e.to_string())?;
    Ok(pairs.into_iter().collect())
}

/// Persist a batch of settings (non-secret).
#[tauri::command]
pub fn save_settings(
    state: State<AppState>,
    settings: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for (k, v) in settings {
        state.db.set_setting(&k, &v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Copy a stored statement PDF into the user's Downloads folder under a readable
/// name (`Bank_YYYY-MM-DD.pdf`), then reveal/open it. Returns the saved path.
/// Errors if the PDF was never captured (statement predates the feature) — the
/// message tells the user to re-scan.
#[tauri::command]
pub fn download_statement(app: AppHandle, id: i64) -> Result<String, String> {
    let state = app.state::<AppState>();
    let src = state.statement_pdf_path(id);
    if !src.exists() {
        return Err("Bu ekstrenin PDF'i kayıtlı değil. 'Tara'ya basıp tekrar deneyin.".into());
    }
    let row = state
        .db
        .get_statement(id)
        .map_err(|e| e.to_string())?
        .ok_or("Ekstre bulunamadı.")?;

    let downloads = app
        .path()
        .download_dir()
        .map_err(|e| format!("İndirilenler klasörü bulunamadı: {e}"))?;
    let name = format!("{}_{}.pdf", sanitize_filename(&row.bank), row.due_date);
    let dest = unique_path(downloads.join(name));

    std::fs::copy(&src, &dest).map_err(|e| format!("PDF kaydedilemedi: {e}"))?;
    reveal(&dest);
    Ok(dest.to_string_lossy().into_owned())
}

/// Replace path-hostile characters (separators, Turkish quirks aside) with `_`.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// First free `name.pdf`, `name (1).pdf`, ... so repeated downloads never clobber.
fn unique_path(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("ekstre").to_string();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("pdf").to_string();
    for n in 1.. {
        let candidate = dir.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

/// Open the saved file with the OS default handler (macOS/Windows). Best-effort:
/// the file is already saved, so a failure to open is non-fatal.
fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

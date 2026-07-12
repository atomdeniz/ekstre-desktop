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
    pub bank: String,
    pub card_masked: Option<String>,
    pub total_due_fmt: String,
    pub min_due_fmt: Option<String>,
    pub due_date: String,
    pub statement_date: Option<String>,
    pub days_left: Option<i64>,
    pub color: String,
}

/// Latest statement per card, shaped for the dashboard.
#[tauri::command]
pub fn get_statements(state: State<AppState>) -> Result<Vec<CardView>, String> {
    let today = state.db.today_local().map_err(|e| e.to_string())?;
    let colors = state.colors();
    let rows = state.db.latest_per_card().map_err(|e| e.to_string())?;
    let cards = rows
        .into_iter()
        .map(|r| CardView {
            days_left: days_left(&r.due_date, &today),
            color: colors.get(&r.bank).cloned().unwrap_or_else(|| "#666666".into()),
            total_due_fmt: format_amount_tr(r.total_due),
            min_due_fmt: r.min_due.map(format_amount_tr),
            bank: r.bank,
            card_masked: r.card_masked,
            due_date: r.due_date,
            statement_date: r.statement_date,
        })
        .collect();
    Ok(cards)
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
    state: State<AppState>,
    form: ImapForm,
    selected_banks: Vec<String>,
) -> Result<(), String> {
    if !form.password.is_empty() {
        state.set_imap_password(&form.user, &form.password)?;
    }
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

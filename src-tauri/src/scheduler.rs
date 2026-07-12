//! Background scheduler: periodic IMAP poll + daily reminder pass with native
//! notifications. Desktop-specific concern — a laptop that was asleep at the
//! reminder hour still gets notified, because `due_unreminded` returns anything
//! due-and-unreminded and we re-check it every cycle (so a missed day is caught
//! at the next wake, not lost).

use std::time::Duration;

use ekstre_core::{days_left, reminder_body, reminder_title_lead};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::{AppState, Config};

/// Spawn the scheduler thread. Runs one poll + reminder pass immediately, then
/// repeats every `poll_interval_min`.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || loop {
        let interval_min = {
            let state = app.state::<AppState>();
            let cfg = Config::load(&state.db);

            if cfg.is_configured() {
                match state.run_poll() {
                    Ok(n) if n > 0 => log::info!("poll added {n} statement(s)"),
                    Ok(_) => {}
                    Err(e) => log::warn!("poll failed: {e}"),
                }
            }
            run_reminders(&app, &state, &cfg);
            cfg.poll_interval_min.max(1)
        };
        std::thread::sleep(Duration::from_secs(interval_min * 60));
    });
}

/// Send a native notification for every due, unreminded statement, then mark it
/// reminded (send-before-mark, so a failed notification retries next cycle).
/// Suppressed before the configured reminder hour to avoid odd-hour pings.
fn run_reminders(app: &AppHandle, state: &AppState, cfg: &Config) {
    let hour = state.db.local_hour().unwrap_or(0);
    if hour < cfg.reminder_hour {
        return;
    }
    let rows = match state.db.due_unreminded(cfg.reminder_days_before) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("due_unreminded failed: {e}");
            return;
        }
    };
    let today = state.db.today_local().unwrap_or_default();
    for row in rows {
        let left = days_left(&row.due_date, &today).unwrap_or(0);
        let sent = app
            .notification()
            .builder()
            .title(reminder_title_lead(&row, left))
            .body(reminder_body(&row))
            .show();
        match sent {
            Ok(()) => {
                if let Ok(now) = state.db.now_iso() {
                    let _ = state.db.mark_reminded(row.id, &now);
                }
                log::info!("reminded {} / {}", row.bank, row.due_date);
            }
            Err(e) => log::warn!("notification failed: {e}"),
        }
    }
}

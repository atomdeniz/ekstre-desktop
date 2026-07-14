# CLAUDE.md — Ekstre Desktop

## Overview

Ekstre is a desktop app (macOS, Windows, Linux) that tracks Turkish credit-card
statements. It scans the user's email inbox over IMAP **read-only** (never marks
mail seen), parses bank statements (from the email body or the first PDF
attachment via pdfium), stores them in a local SQLite file, shows a menu-bar
dashboard, and fires **native OS notifications** on the due date. Data never
leaves the machine.

It is a desktop rewrite of the older Docker/FastAPI self-hosted project
[`../ekstre`](../ekstre). The parsing/storage/matching core was ported from
Python to Rust; behavior is preserved and pinned by golden tests. Built for
**non-technical users** — no server, no Docker, no config files. Install the app,
run the setup wizard, done.

## Architecture

Three layers:

```
Tauri shell (Rust, src-tauri/) ── tray/menu-bar · native notifications · updater · scheduler
   └─ core (Rust lib, core/)   ── banks · parser · db · imap · pdf · format · matcher
Webview (vanilla HTML/JS, ui/) ── setup wizard · dashboard · settings
```

- The webview talks to Rust only through Tauri IPC: `invoke(command, args)` for
  calls, `emit` for events. `withGlobalTauri` is **on**, so the JS uses
  `window.__TAURI__.core.invoke` directly (no npm bundler/frontend build step).
- **There is NO HTTP server and NO Telegram** (both existed in the old Python
  project). Reminders are native OS notifications, not chat messages.

## Key modules

### core/ (`ekstre-core` crate — pure logic, no GUI)
- `banks.rs` — `Bank` definitions + `load_banks_str`. Loads `banks/banks.yml`
  (embedded via `include_str!`), compiles per-field regexes case-insensitively.
- `parser.rs` — `parse_statement(bank, text)`. Valid **iff both `total_due` and
  `due_date` match**; other fields optional. Turkish amount/date parsing.
- `db.rs` — `Database` (single connection behind a `Mutex`). SQLite schema,
  dedup unique index, `settings` table, and reminder-eligibility rules
  (`due_unreminded`, `latest_per_card`, past-due suppression on insert).
- `imap.rs` — `scan(...)`: read-only rolling-window mailbox scan (`BODY.PEEK[]`,
  `SINCE` lookback), per-bank From/Subject match, body-or-PDF text extraction.
- `pdf.rs` — pdfium text extraction (`bind_pdfium`, `extract_text`). Dynamically
  loaded shared lib; no link-time dependency. pdfium is not `Send` — bound fresh
  per poll thread.
- `format.rs` — Turkish amount formatting, `days_left`, reminder title/body text.
- `matcher.rs` — header substring matching, HTML stripping, body-text selection.

### src-tauri/ (`ekstre-desktop` crate — the shell)
- `lib.rs` — `run()`: Tauri builder, tray/menu-bar setup (Panoyu aç / Şimdi tara /
  Çıkış), close-to-tray, background update check, `invoke_handler` registration.
- `state.rs` — `AppState` (db, banks, pdfium lib dir, data dir) held in Tauri
  managed state, plus `Config` (non-secret settings loaded from the DB) and the
  poll orchestration (`run_poll`, `test_scan`, keychain password get/set).
- `commands.rs` — Tauri IPC commands: `get_statements`, `is_configured`,
  `list_banks`, `poll_now`, `test_imap`, `complete_setup`, `get_settings`,
  `save_settings`. Builds the dashboard view-model (`CardView`).
- `scheduler.rs` — background thread: periodic poll + daily reminder pass.
  Wake/misfire-safe — `due_unreminded` is re-checked every cycle, so a reminder
  missed while asleep fires at the next wake. Reminders suppressed before the
  configured `reminder_hour`.

## Data locations

- **SQLite DB**: `ekstre.db` in the OS app-data dir (`app_data_dir()`,
  e.g. `~/Library/Application Support/com.denizozogul.ekstre/` on macOS).
- **IMAP password**: OS Keychain via `keyring`, service
  `com.denizozogul.ekstre`, keyed by the account user. Dev fallback:
  `EKSTRE_IMAP_PASSWORD` env var.
- **Settings**: DB `settings` table (host, port, user, mailbox, selected_banks,
  lookback_days, poll_interval_min, reminder_hour). Non-secret only.
- **libpdfium**: loaded from `vendor/pdfium/lib` in dev (or `PDFIUM_LIB_DIR`), or
  from bundled app resources (`resources/pdfium/libpdfium.dylib`) when shipped.
  `find_pdfium_dir` tries candidates in order.

## Build / test / run

```bash
cargo test -p ekstre-core        # core golden + unit tests (CI runs exactly this)
cargo tauri dev                  # run the app (menu-bar + window)
cargo build -p ekstre-desktop    # build the shell
```

Dev examples (in `core/examples/`, need pdfium bound):

```bash
# Extract PDF text and optionally parse with a named built-in bank:
cargo run -p ekstre-core --example pdfcheck -- <pdf-file> [bank-name]
# Strip an HTML email body and parse it with a named bank:
cargo run -p ekstre-core --example realcheck -- <bank-name> <html-file>
# Parse every PDF in a dir into a DB and print the dashboard view-model:
cargo run -p ekstre-core --example seed -- <db-path> <samples-dir>
```

Set `PDFIUM_LIB_DIR` if libpdfium isn't at `vendor/pdfium/lib`.

**Gitignored, not in the repo** (see `.gitignore`):
- `vendor/pdfium/lib/libpdfium.dylib` — **required** for PDF-source banks. Download
  once (CI downloads per-platform). Without it, PDF banks silently won't parse.
- `samples/` — real bank PDFs used for manual validation. Never commit these.
- `*.db`, `data/`, `src-tauri/resources/pdfium/`.

## Bank definitions

Banks live in `core/banks/banks.yml`, embedded at compile time. Config-driven and
community-extendable: adding a bank is a YAML entry (a community PR), not code.
Users never edit it — the wizard shows the banks as checkboxes. Field regex
contract: `card` → 2 groups (first4, last4); `total_due`/`min_due` → 1 amount
group; `due_date`/`statement_date` → 3 groups (day, month, year). Amounts are
Turkish-formatted (`1.234,56`, and the `,-` round-amount shorthand meaning
`,00`). The built-in TEB / Enpara / İş Bankası regexes are **verified against
real 2026 PDFs**.

## Conventions

- Rust 2021. Match the existing concise style in each file.
- **Parity with the original Python behavior is intentional and preserved.** Many
  modules carry a `//! Mirrors app/xyz.py` note. Behavior is pinned by golden
  tests in `core/tests/` (`golden.rs`, `format_matcher.rs`). If you change parsing
  or storage, keep these green — they are the port-parity guarantee.
- Test fixtures in `core/tests/fixtures/` are **SYNTHETIC**. Never commit real
  financial data. Real PDFs live in the gitignored `samples/` dir.

## CODE STYLE

Write **NO unnecessary comments**. A comment may only state a constraint the code
cannot show — a non-obvious invariant, a subtle ordering requirement, a format
quirk. Never write a comment that restates what the next line does, says where
code came from, or explains an obvious mapping. Keep comment density low and match
surrounding code.

## Release / signing

Signing, notarization, and auto-update are documented in
[`docs/RELEASING.md`](docs/RELEASING.md). Releases are built by
`.github/workflows/release.yml` on a `v*` tag push (macOS universal, Developer ID
sign + notarize, Tauri updater artifacts). The updater public key and endpoint are
in `src-tauri/tauri.conf.json`.

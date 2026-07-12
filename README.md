# Ekstre

**Never miss a credit-card payment again.** Ekstre is a small desktop app for
macOS and Windows that reads your bank's statement emails, understands them, and
reminds you before each payment is due — with a native notification, right from
your menu bar or system tray.

No server, no Docker, no accounts. You download it, connect your email once, and
it quietly does the rest. **Your data never leaves your computer.**

> Turkish banks send credit-card statements by email. Ekstre reads those emails,
> pulls out the amount due and the due date, and makes sure you see them in time.

## Features

- **Reads your statements automatically** — scans your mailbox over IMAP
  (read-only; it never marks anything as read) and recognizes statement emails
  from the banks you choose.
- **Understands email and PDF statements** — parses the amount due, minimum
  payment, statement date, and due date, whether they're in the email body or a
  PDF attachment.
- **Reminds you on time** — a native macOS/Windows notification on the due day, so
  a payment never slips by. Reminders survive sleep/wake, so a laptop that was
  closed at 9 a.m. still gets reminded when it wakes up.
- **A clean dashboard** — the latest statement per card at a glance: amount, due
  date, and a color-coded "days left" badge. Blur amounts with one click for
  privacy in public.
- **Lives in your menu bar / tray** — starts at login, runs quietly in the
  background, out of your way.
- **Private by design** — see below.

## Privacy

Ekstre stores everything (statements, account settings) **only on your own
device**. It never sends your data to any server and collects no telemetry. Your
mailbox is scanned **read-only**, and your email password is kept in the operating
system's secure store (macOS Keychain / Windows Credential Manager) — never in
plain text.

## Supported banks

Out of the box: **TEB**, **Enpara**, and **İş Bankası**, verified against real
statement PDFs. Bank definitions are simple config entries, so adding a bank is a
small pull request rather than a code change — see [Adding a bank](#adding-a-bank).

## Download & install

Grab the latest build from the
[Releases](https://github.com/atomdeniz/ekstre-desktop/releases) page:

- **macOS** — a `.dmg`, signed with an Apple Developer ID and notarized by Apple,
  so it opens without security warnings.
- **Windows** — an installer (`.exe`). Windows builds are not yet code-signed, so
  on first run SmartScreen may warn you — choose **More info → Run anyway**.
  (Signing is planned once the project qualifies.)

The app updates itself automatically, so you install once and stay current.

### First run

On first launch a short setup wizard walks you through:

1. Choosing your email provider (Gmail, Outlook/Hotmail, Yahoo, iCloud, or a
   custom IMAP server).
2. Connecting your mailbox. For Gmail (and most providers) you'll create an
   **app-specific password** — the wizard links you to the right page. A **Test
   connection** button confirms it works and shows how many statements it found.
3. Selecting which banks to track.

That's it. Ekstre then checks periodically and reminds you when a payment is due.

## How it works

```
Tauri shell (Rust) ── tray · native notifications · autostart · auto-update · scheduler
   └─ core ── bank definitions · statement parser · SQLite store · matching
Webview (HTML/JS) ── setup wizard · dashboard · settings
```

A Rust core does the parsing and storage; a thin Tauri shell provides the window,
tray, notifications, and background scheduling; a small HTML/JS front end is the
UI. There is no HTTP server — the UI talks to the core directly over Tauri's IPC,
and notifications come from the operating system itself.

## Development

Prerequisites: [Rust](https://rustup.rs) and the
[Tauri CLI](https://v2.tauri.app/start/) (`cargo install tauri-cli --version "^2"`).

```bash
cargo test -p ekstre-core      # run the core test suite
cargo tauri dev                # run the app (menu bar + window)
cargo build -p ekstre-desktop  # compile the whole app
```

PDF parsing uses [pdfium](https://github.com/bblanchon/pdfium-binaries). For local
development, place `libpdfium.dylib` in `vendor/pdfium/lib/` (downloaded once); CI
and release builds fetch the right binary per platform automatically.

To try a real scan, enter your email app-password in the setup wizard.

## Adding a bank

Bank definitions live in [`core/banks/banks.yml`](core/banks/banks.yml) and are
compiled into the app; users pick from them with checkboxes and never edit YAML.
Each entry is a display name, a color, From/Subject match rules, a source
(`body` or `pdf`), and a set of field regexes:

```yaml
- name: My Bank
  color: "#123456"
  match: { from: mybank.com, subject: Statement }
  source: pdf            # or "body" for the plain-text email
  fields:
    card: 'Card No:\s*(\d{4})\*+(\d{4})'          # 2 groups: first4, last4
    total_due: 'Total Due:\s*([\d.]+,\d{2})'       # Turkish amount, e.g. 1.234,56
    due_date: 'Due Date:\s*(\d{2})[./](\d{2})[./](\d{4})'  # day, month, year
```

A statement is stored only when both `total_due` and `due_date` match. Add your
bank, verify it against a real statement, and open a pull request.

## Releasing

Tagging a version (`git tag v0.1.0 && git push origin v0.1.0`) builds, signs,
notarizes, and publishes macOS and Windows releases with auto-update artifacts.
See [`docs/RELEASING.md`](docs/RELEASING.md) for the required secrets and setup.

## License

[MIT](LICENSE)

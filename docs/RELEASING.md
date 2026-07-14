# Releasing (macOS signed + notarized + auto-update)

A release is built automatically by `.github/workflows/release.yml` when a `v*`
tag is pushed. It is **signed, notarized**, and uploaded to GitHub Releases as a
**draft** along with the auto-update artifacts.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Required GitHub Secrets

Repo → Settings → Secrets and variables → Actions:

### Apple signing + notarization
| Secret | Where from |
|---|---|
| `APPLE_CERTIFICATE` | Export your "Developer ID Application" certificate as `.p12`, then `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | The password you set when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: First Last (TEAMID)` |
| `APPLE_ID` | The email of your Apple Developer account |
| `APPLE_PASSWORD` | An **app-specific password** from appleid.apple.com (for notarization) |
| `APPLE_TEAM_ID` | Your 10-character Team ID |

### Auto-update signing (Tauri)
| Secret | Where from |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The **contents** of the updater private key (see below) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The key password (empty in our case) |

## Updater key pair

The **public key** embedded in the `plugins.updater.pubkey` field of
`tauri.conf.json` is already in the repo. You must generate the matching
**private key** yourself (if you want to keep the existing public key, use the
private key generated during initial setup; if you lost it, generate a new one
and update `pubkey`):

```bash
cargo tauri signer generate -w ~/.tauri/ekstre-updater.key
# Put the resulting public key into tauri.conf.json > plugins.updater.pubkey
# Put the contents of the private key into the TAURI_SIGNING_PRIVATE_KEY secret:
cat ~/.tauri/ekstre-updater.key | pbcopy
```

> ⚠️ If you lose the private key you cannot ship updates to existing users.
> Keep it somewhere safe.

## Updater endpoint

`tauri.conf.json > plugins.updater.endpoints` currently points at the
`github.com/atomdeniz/ekstre-desktop` repo. If your repo name differs, update
this and the pdfium version in `release.yml`.

## Windows

The release workflow now builds **Windows too** (matrix): on `windows-latest`
`pdfium.dll` is downloaded, and the NSIS `.exe` installer + updater artifacts are
produced and uploaded to the same release. No extra secrets needed — the updater
signature is already shared.

**Signing (not yet in place):** the Windows build is currently **unsigned**, so
on first launch SmartScreen shows an "unknown publisher" warning (dismissable:
"More info → Run anyway"). If you want to remove this for non-technical users,
add signing:

- **[SignPath](https://signpath.io)** — offers **free** code signing for
  open-source projects (application/approval required). The recommended path.
- **Azure Trusted Signing** — ~$10/month, quick setup.

Once signing is added, wire the Windows signing env vars into `tauri-action`
(e.g. the SignPath action or `certificateThumbprint`); the rest of the pipeline
stays the same.

## Linux

The matrix also builds on `ubuntu-22.04` (kept at the oldest supported LTS so
the AppImage runs on older glibc): `libpdfium.so` is downloaded, the Tauri
system packages are installed, and an `.AppImage`, `.deb`, and `.rpm` are
produced and uploaded to the same release. No extra secrets needed.

Auto-update on Linux only applies to the **AppImage** (the updater artifact +
signature are generated for it and included in `latest.json`); `.deb`/`.rpm`
installs are updated by the user's package manager. Linux packages are not
signed — this is normal for direct downloads.

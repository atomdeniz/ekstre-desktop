# Sürüm çıkarma (macOS imzalı + notarized + auto-update)

Bir sürüm, `v*` etiketi push'lanınca `.github/workflows/release.yml` tarafından
otomatik derlenir, **imzalanır, notarize edilir** ve auto-update artifact'leriyle
birlikte GitHub Releases'a **taslak** olarak yüklenir.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Gerekli GitHub Secrets

Repo → Settings → Secrets and variables → Actions:

### Apple imzalama + notarization
| Secret | Nereden |
|---|---|
| `APPLE_CERTIFICATE` | "Developer ID Application" sertifikanı `.p12` olarak dışa aktar, `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` dışa aktarırken belirlediğin şifre |
| `APPLE_SIGNING_IDENTITY` | ör. `Developer ID Application: Ad Soyad (TEAMID)` |
| `APPLE_ID` | Apple Developer hesabının e-postası |
| `APPLE_PASSWORD` | appleid.apple.com'dan **uygulamaya özel şifre** (notarization için) |
| `APPLE_TEAM_ID` | 10 haneli Team ID |

### Auto-update imzalama (Tauri)
| Secret | Nereden |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Updater özel anahtarının **içeriği** (aşağıya bak) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Anahtar şifresi (bizde boş) |

## Updater anahtar çifti

`tauri.conf.json` içindeki `plugins.updater.pubkey` alanına gömülü **açık anahtar**
zaten repoda. Eşleşen **özel anahtarı** kendin üretmelisin (mevcut açık anahtarı
kullanmak istiyorsan, ilk kurulumda üretilen özel anahtarı kullan; kaybettiysen
yenisini üret ve `pubkey`'i güncelle):

```bash
cargo tauri signer generate -w ~/.tauri/ekstre-updater.key
# Çıkan açık anahtarı tauri.conf.json > plugins.updater.pubkey içine koy
# Özel anahtarın içeriğini TAURI_SIGNING_PRIVATE_KEY secret'ına koy:
cat ~/.tauri/ekstre-updater.key | pbcopy
```

> ⚠️ Özel anahtarı kaybedersen mevcut kullanıcılara güncelleme gönderemezsin.
> Güvenli bir yerde sakla.

## Updater endpoint

`tauri.conf.json > plugins.updater.endpoints` şu an
`github.com/atomdeniz/ekstre-desktop` reposuna işaret ediyor. Repo adın farklıysa
burayı ve `release.yml`'deki pdfium sürümünü güncelle.

## Windows (sonraki aşama)

Yapı hazır; `release.yml`'e `windows-latest` job'ı eklenip imza için
[SignPath](https://signpath.io) (açık kaynak ücretsiz) veya Azure Trusted Signing
bağlanır. pdfium için `pdfium-win-x64.tgz` indirilir.

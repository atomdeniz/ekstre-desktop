# Ekstre Desktop

Türk kredi kartı ekstrelerini takip eden bir **masaüstü uygulaması** (macOS, sonra Windows).
Sunucu ya da Docker istemez: uygulamayı indir, kur, menü çubuğunda çalışsın. E-posta kutunu
(salt-okunur) tarar, banka ekstrelerini ayrıştırır, yerel bir SQLite'a yazar ve son ödeme günü
**işletim sisteminin native bildirimini** gönderir. Verin bu bilgisayardan çıkmaz.

> Bu, Docker'da çalışan self-hosted [`ekstre`](../ekstre) projesinin masaüstü yeniden yazımıdır.
> Çekirdek (ayrıştırma/depolama/eşleştirme) Python'dan Rust'a portlandı; davranış, taşınan altın
> testlerle birebir korunuyor.

## Durum

Yapım aşamasında. Milestone'lar:

- [x] **M1 — Çekirdek port + altın testler.** `core/` crate: banka tanımları, statement
      ayrıştırma, SQLite depolama/dedup, hatırlatma-uygunluğu, Türkçe tutar biçimi, From/Subject
      eşleştirme, IMAP taraması, pdfium metin çıkarma. `cargo test` yeşil. Regex'ler **gerçek 2026
      TEB/Enpara/İş Bankası PDF ekstrelerine** karşı doğrulandı.
- [x] **M2 — Tauri kabuk + tray/menübar + dashboard.** `src-tauri/` + vanilla `ui/`. Menü çubuğu
      ikonu (Panoyu aç / Şimdi tara / Çıkış), pencere kapatınca tray'de kalma, `invoke` komutları
      (`get_statements`, `poll_now`, `list_banks`, ayarlar). Uygulama temiz açılıyor; veri akışı
      gerçek PDF'lerle uçtan uca doğrulandı.
- [x] **M3 — Scheduler + native bildirim + uyanma/misfire.** Arka plan thread'i: periyodik poll +
      günlük hatırlatma taraması. Uyku/uyanma sonrası kaçan hatırlatmalar `due_unreminded` her döngüde
      yeniden kontrol edilerek yakalanır. Native bildirim `tauri-plugin-notification` ile.
- [x] **M4 — Setup wizard + Keychain + IMAP/PDF entegrasyonu.** İlk açılışta çok adımlı sihirbaz
      (hoş geldin/gizlilik → e-posta + "bağlantıyı test et" → banka seçimi). IMAP parolası macOS
      Keychain'de saklanır; `test_imap` gerçek bağlantı kurup bulunan ekstre sayısını döndürür.
- [x] **M5 — İmzalama + notarization + auto-update.** `tauri-plugin-updater` + imzalı `latest.json`;
      pdfium bundle kaynağı olarak gömülü; GitHub Actions CI (testler) + release workflow (macOS
      universal build, Developer ID imza + notarization, updater artifact'leri). Bkz.
      [`docs/RELEASING.md`](docs/RELEASING.md) — gereken secret'lar ve updater anahtarı.

## Mimari

```
Tauri kabuk (Rust) ── tray · native bildirim · autostart · updater · scheduler
   └─ core (bu crate) ── banks · parser · db · format · matcher   ← Python app/ portu
Webview (vanilla HTML/JS) ── setup wizard · dashboard · ayarlar
```

HTTP sunucusu ve Telegram yok; masaüstünde Rust ↔ webview `invoke`/`emit` ile konuşur ve
bildirimler işletim sisteminin kendisinden gider.

## Geliştirme

```bash
cargo test -p ekstre-core        # çekirdek altın testleri
cargo tauri dev                  # uygulamayı çalıştır (menü çubuğu + pencere)
```

pdfium için `vendor/pdfium/lib/libpdfium.dylib` gerekir (bir kez indirilir; CI
platforma göre otomatik indirir). Gerçek bir taramayı test etmek için Gmail
uygulama şifreni sihirbaza gir.

Sürüm çıkarma ve imzalama için [`docs/RELEASING.md`](docs/RELEASING.md).

Banka tanımları [`core/banks/banks.yml`](core/banks/banks.yml) içinde derlemeye gömülü;
kullanıcı düzenlemez, wizard'da checkbox ile seçer. Yeni banka eklemek bir YAML girdisidir
(topluluk PR'ı) — kod değişikliği değil.

## Gizlilik

Ekstre tüm verini (ekstreler, hesap bilgileri) yalnızca **kendi cihazında** saklar;
hiçbir sunucuya veri göndermez, telemetri toplamaz. E-posta kutun **salt-okunur**
taranır ve hiçbir e-posta okundu olarak işaretlenmez. IMAP parolan işletim sisteminin
güvenli deposunda (macOS Keychain / Windows Credential Manager) tutulur.

## İndirme ve kod imzalama

Kurulum dosyaları [Releases](https://github.com/atomdeniz/ekstre-desktop/releases)
sayfasındadır. Windows derlemeleri [SignPath Foundation](https://signpath.org)'ın açık
kaynak programı aracılığıyla kod-imzalanır; macOS derlemeleri Apple Developer ID ile
imzalanıp notarize edilir.

## Lisans

MIT.

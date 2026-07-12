// Shared email-provider presets, used by the setup wizard and settings screen.
export const PROVIDERS = [
  {
    id: "gmail",
    label: "Gmail",
    host: "imap.gmail.com",
    port: 993,
    hint: 'Gmail için normal şifreni değil bir <b>uygulama şifresi</b> kullan: <code>myaccount.google.com/apppasswords</code> adresinden oluştur.',
  },
  {
    id: "outlook",
    label: "Outlook / Hotmail / Live",
    host: "outlook.office365.com",
    port: 993,
    hint: 'Outlook için hesap güvenlik ayarlarından bir <b>uygulama şifresi</b> oluşturman gerekir.',
  },
  {
    id: "yahoo",
    label: "Yahoo",
    host: "imap.mail.yahoo.com",
    port: 993,
    hint: 'Yahoo için hesap güvenlik ayarlarından bir <b>uygulama şifresi</b> oluştur.',
  },
  {
    id: "icloud",
    label: "iCloud",
    host: "imap.mail.me.com",
    port: 993,
    hint: 'iCloud için <code>appleid.apple.com</code> üzerinden bir <b>uygulamaya özel şifre</b> oluştur.',
  },
  {
    id: "custom",
    label: "Diğer",
    host: "",
    port: 993,
    hint: "Sunucu ve port bilgilerini e-posta sağlayıcından öğrenip aşağıya gir.",
  },
];

export function providerById(id) {
  return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[PROVIDERS.length - 1];
}

export function providerForHost(host) {
  const p = PROVIDERS.find((p) => p.host && p.host === host);
  return p ? p.id : "custom";
}

export function fillProviderSelect(selectEl) {
  selectEl.innerHTML = PROVIDERS.map(
    (p) => `<option value="${p.id}">${p.label}</option>`
  ).join("");
}

export function applyProvider(id, { hostEl, portEl, serverFieldsEl, hintEl }) {
  const p = providerById(id);
  if (hintEl) hintEl.innerHTML = p.hint;
  if (serverFieldsEl) serverFieldsEl.hidden = id !== "custom";
  if (id !== "custom") {
    if (hostEl) hostEl.value = p.host;
    if (portEl) portEl.value = p.port;
  }
}

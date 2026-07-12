import {
  fillProviderSelect,
  applyProvider,
  providerForHost,
} from "./providers.js";

const { invoke } = window.__TAURI__.core;

const providerEl = document.getElementById("provider");
const hostEl = document.getElementById("host");
const portEl = document.getElementById("port");
const userEl = document.getElementById("user");
const passwordEl = document.getElementById("password");
const mailboxEl = document.getElementById("mailbox");
const reminderDaysEl = document.getElementById("reminder-days");
const serverFieldsEl = document.getElementById("server-fields");
const hintEl = document.getElementById("pw-hint");
const testResult = document.getElementById("test-result");

fillProviderSelect(providerEl);
providerEl.addEventListener("change", () =>
  applyProvider(providerEl.value, { hostEl, portEl, serverFieldsEl, hintEl })
);

function form() {
  return {
    host: hostEl.value.trim() || "imap.gmail.com",
    port: Number(portEl.value) || 993,
    user: userEl.value.trim(),
    password: passwordEl.value,
    mailbox: mailboxEl.value.trim() || "INBOX",
  };
}

async function init() {
  const [settings, banks] = await Promise.all([
    invoke("get_settings"),
    invoke("list_banks"),
  ]);

  userEl.value = settings.imap_user || "";
  mailboxEl.value = settings.imap_mailbox || "INBOX";
  reminderDaysEl.value =
    settings.reminder_days_before != null ? settings.reminder_days_before : 3;
  const host = settings.imap_host || "imap.gmail.com";
  const port = Number(settings.imap_port) || 993;
  const providerId = providerForHost(host);
  providerEl.value = providerId;
  applyProvider(providerId, { hostEl, portEl, serverFieldsEl, hintEl });
  if (providerId === "custom") {
    hostEl.value = host;
    portEl.value = port;
  }

  const selected = (settings.selected_banks || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const all = selected.length === 0;
  document.getElementById("bank-list").innerHTML = banks
    .map(
      (b) => `
      <label class="bank-row" style="--accent:${b.color}">
        <input type="checkbox" value="${b.name}" ${
        all || selected.includes(b.name) ? "checked" : ""
      } />
        <span style="color:${b.color}">${b.name}</span>
      </label>`
    )
    .join("");
}

document.getElementById("test-btn").addEventListener("click", async () => {
  const f = form();
  if (!f.user || !f.password) {
    testResult.className = "result err";
    testResult.textContent = "Test için e-posta ve şifre gir.";
    return;
  }
  testResult.className = "result";
  testResult.textContent = "Bağlanılıyor…";
  try {
    const n = await invoke("test_imap", { form: f });
    testResult.className = "result ok";
    testResult.textContent = `✓ Bağlandı — son 90 günde ${n} ekstre bulundu`;
  } catch (e) {
    testResult.className = "result err";
    testResult.textContent = `✗ ${e}`;
  }
});

document.getElementById("cancel").addEventListener("click", () => {
  window.location.href = "index.html";
});

document.getElementById("save").addEventListener("click", async () => {
  const f = form();
  if (!f.user) {
    testResult.className = "result err";
    testResult.textContent = "E-posta adresi gerekli.";
    return;
  }
  const selected = [...document.querySelectorAll("#bank-list input:checked")].map(
    (i) => i.value
  );
  const reminderDaysBefore = Math.max(0, Number(reminderDaysEl.value) || 0);
  try {
    await invoke("update_settings", {
      form: f,
      selectedBanks: selected,
      reminderDaysBefore,
    });
    window.location.href = "index.html";
  } catch (e) {
    alert(`Kaydedilemedi: ${e}`);
  }
});

init();

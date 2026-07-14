import {
  fillProviderSelect,
  applyProvider,
} from "./providers.js";

const { invoke } = window.__TAURI__.core;

const providerEl = document.getElementById("provider");
const hostEl = document.getElementById("host");
const portEl = document.getElementById("port");
const userEl = document.getElementById("user");
const passwordEl = document.getElementById("password");
const mailboxEl = document.getElementById("mailbox");
const reminderDaysEl = document.getElementById("reminder-days");
const launchAtLoginEl = document.getElementById("launch-at-login");
const serverFieldsEl = document.getElementById("server-fields");
const hintEl = document.getElementById("pw-hint");
const testResult = document.getElementById("test-result");
const accountListEl = document.getElementById("account-list");
const addBoxEl = document.getElementById("add-account");

fillProviderSelect(providerEl);
function syncProvider() {
  applyProvider(providerEl.value, { hostEl, portEl, serverFieldsEl, hintEl });
}
providerEl.addEventListener("change", syncProvider);
syncProvider();

function form() {
  return {
    host: hostEl.value.trim() || "imap.gmail.com",
    port: Number(portEl.value) || 993,
    user: userEl.value.trim(),
    password: passwordEl.value,
    mailbox: mailboxEl.value.trim() || "INBOX",
  };
}

// No native confirm() in the webview (wry doesn't implement the JS panels on
// macOS), so the row itself switches into an explicit Vazgeç/Kaldır state.
let accounts = [];
let confirmingUser = null;
let removeError = null;

function accountRow(a) {
  if (a.user === confirmingUser) {
    return `
      <div class="account-row confirming">
        <div class="account-info">
          <b>${a.user}</b>
          <span class="hint">Hesap kaldırılsın mı? Kayıtlı ekstreler silinmez.</span>
        </div>
        <div class="confirm-actions">
          <button class="btn" data-cancel>Vazgeç</button>
          <button class="btn danger-solid" data-confirm="${a.user}">Kaldır</button>
        </div>
      </div>`;
  }
  return `
    <div class="account-row">
      <div class="account-info">
        <b>${a.user}</b>
        <span class="hint">${a.host}</span>
      </div>
      <button class="btn danger" data-remove="${a.user}">Kaldır</button>
    </div>`;
}

function renderAccounts() {
  const err = removeError ? `<p class="result err">${removeError}</p>` : "";
  accountListEl.innerHTML = accounts.length
    ? accounts.map(accountRow).join("") + err
    : `<p class="hint">Henüz hesap yok — aşağıdan ekle.</p>`;
}

async function loadAccounts() {
  accounts = await invoke("list_accounts");
  renderAccounts();
}

accountListEl.addEventListener("click", async (e) => {
  const remove = e.target.closest("[data-remove]");
  if (remove) {
    confirmingUser = remove.dataset.remove;
    removeError = null;
    renderAccounts();
    return;
  }
  if (e.target.closest("[data-cancel]")) {
    confirmingUser = null;
    renderAccounts();
    return;
  }
  const confirmBtn = e.target.closest("[data-confirm]");
  if (!confirmBtn) return;
  confirmBtn.disabled = true;
  try {
    await invoke("remove_account", { user: confirmBtn.dataset.confirm });
    confirmingUser = null;
    removeError = null;
    await loadAccounts();
  } catch (err) {
    confirmingUser = null;
    removeError = `Kaldırılamadı: ${err}`;
    renderAccounts();
  }
});

document.getElementById("add-btn").addEventListener("click", async () => {
  const f = form();
  if (!f.user || !f.password) {
    testResult.className = "result err";
    testResult.textContent = "E-posta ve şifre gerekli.";
    return;
  }
  try {
    await invoke("add_account", { form: f });
    testResult.className = "result ok";
    testResult.textContent = "✓ Hesap eklendi";
    userEl.value = "";
    passwordEl.value = "";
    addBoxEl.open = false;
    await loadAccounts();
  } catch (err) {
    testResult.className = "result err";
    testResult.textContent = `✗ ${err}`;
  }
});

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

function agoLabel(d) {
  if (d === null || d === undefined) return "";
  if (d <= 0) return "bugün";
  if (d < 31) return `${d} gün önce`;
  if (d < 365) return `${Math.round(d / 30)} ay önce`;
  return `${Math.round(d / 365)} yıl önce`;
}

function cardSub(c) {
  const label = c.card_masked ?? (c.card_last4 ? `**** ${c.card_last4}` : "Kart");
  const ago = agoLabel(c.last_seen_days);
  return `
    <label class="card-sub">
      <input type="checkbox" data-card="${c.key}" ${c.enabled ? "checked" : ""} />
      <span class="masked">${label}</span>
      ${ago ? `<span class="card-ago">son ekstre ${ago}</span>` : ""}
    </label>`;
}

async function init() {
  const [settings, banks, cards] = await Promise.all([
    invoke("get_settings"),
    invoke("list_banks"),
    invoke("list_cards"),
  ]);
  await loadAccounts();

  reminderDaysEl.value =
    settings.reminder_days_before != null ? settings.reminder_days_before : 3;
  launchAtLoginEl.checked = settings.launch_at_login !== "false";

  const cardsByBank = new Map();
  for (const c of cards) {
    const arr = cardsByBank.get(c.bank) || [];
    arr.push(c);
    cardsByBank.set(c.bank, arr);
  }

  const selected = (settings.selected_banks || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const all = selected.length === 0;
  document.getElementById("bank-list").innerHTML = banks
    .map((b) => {
      const subs = (cardsByBank.get(b.name) || []).map(cardSub).join("");
      return `
      <div class="bank-group" style="--accent:${b.color}">
        <label class="bank-row" style="--accent:${b.color}">
          <input type="checkbox" value="${b.name}" ${
        all || selected.includes(b.name) ? "checked" : ""
      } />
          <span style="color:${b.color}">${b.name}</span>
        </label>
        ${subs ? `<div class="card-subs">${subs}</div>` : ""}
      </div>`;
    })
    .join("");
}

document.getElementById("cancel").addEventListener("click", () => {
  window.location.href = "index.html";
});
document.getElementById("back").addEventListener("click", () => {
  window.location.href = "index.html";
});

const deepBtn = document.getElementById("deep-scan");
const deepDaysEl = document.getElementById("deep-days");
const deepResult = document.getElementById("deep-result");
deepBtn.addEventListener("click", async () => {
  const days = Math.min(3650, Math.max(0, Number(deepDaysEl.value) || 0));
  deepDaysEl.value = days;
  deepBtn.disabled = true;
  deepResult.className = "result";
  deepResult.textContent =
    days === 0
      ? "Tüm geçmiş taranıyor… birkaç dakika sürebilir."
      : `Son ${days} gün taranıyor… birkaç dakika sürebilir.`;
  try {
    const n = await invoke("deep_scan", { days });
    deepResult.className = "result ok";
    deepResult.textContent =
      n > 0 ? `✓ ${n} eski ekstre eklendi` : "✓ Tarama bitti — yeni ekstre yok";
  } catch (e) {
    deepResult.className = "result err";
    deepResult.textContent = `✗ ${e}`;
  } finally {
    deepBtn.disabled = false;
  }
});

document.getElementById("save").addEventListener("click", async () => {
  const selected = [
    ...document.querySelectorAll("#bank-list .bank-row input:checked"),
  ].map((i) => i.value);
  const disabledCards = [
    ...document.querySelectorAll("#bank-list input[data-card]:not(:checked)"),
  ].map((i) => i.dataset.card);
  const reminderDaysBefore = Math.max(0, Number(reminderDaysEl.value) || 0);
  const saveResult = document.getElementById("save-result");
  try {
    await invoke("update_settings", {
      selectedBanks: selected,
      disabledCards,
      reminderDaysBefore,
      launchAtLogin: launchAtLoginEl.checked,
    });
    window.location.href = "index.html";
  } catch (e) {
    saveResult.className = "result err";
    saveResult.textContent = `Kaydedilemedi: ${e}`;
  }
});

init();

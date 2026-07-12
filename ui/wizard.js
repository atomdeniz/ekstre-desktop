import { fillProviderSelect, applyProvider } from "./providers.js";

const { invoke } = window.__TAURI__.core;

const providerEl = document.getElementById("provider");
const serverFieldsEl = document.getElementById("server-fields");
const hintEl = document.getElementById("pw-hint");
fillProviderSelect(providerEl);
function syncProvider() {
  applyProvider(providerEl.value, {
    hostEl: document.getElementById("host"),
    portEl: document.getElementById("port"),
    serverFieldsEl,
    hintEl,
  });
}
providerEl.addEventListener("change", syncProvider);
syncProvider();

let step = 1;
function show(n) {
  step = n;
  document.querySelectorAll(".step").forEach((s) => {
    s.hidden = Number(s.dataset.step) !== n;
  });
  if (n === 3) loadBanks();
}

document.querySelectorAll("[data-next]").forEach((b) =>
  b.addEventListener("click", () => show(step + 1))
);
document.querySelectorAll("[data-prev]").forEach((b) =>
  b.addEventListener("click", () => show(step - 1))
);

function form() {
  return {
    host: document.getElementById("host").value.trim() || "imap.gmail.com",
    port: Number(document.getElementById("port").value) || 993,
    user: document.getElementById("user").value.trim(),
    password: document.getElementById("password").value,
    mailbox: document.getElementById("mailbox").value.trim() || "INBOX",
  };
}

const testResult = document.getElementById("test-result");
document.getElementById("test-btn").addEventListener("click", async () => {
  const f = form();
  if (!f.user || !f.password) {
    testResult.className = "result err";
    testResult.textContent = "E-posta ve şifre gerekli.";
    return;
  }
  testResult.className = "result";
  testResult.textContent = "Bağlanılıyor…";
  try {
    const n = await invoke("test_imap", { form: f });
    testResult.className = "result ok";
    testResult.textContent = `✓ Bağlandı — son 90 günde ${n} ekstre bulundu`;
    document.getElementById("email-next").disabled = false;
  } catch (e) {
    testResult.className = "result err";
    testResult.textContent = `✗ ${e}`;
  }
});

let banks = [];
async function loadBanks() {
  if (banks.length) return;
  banks = await invoke("list_banks");
  const list = document.getElementById("bank-list");
  list.innerHTML = banks
    .map(
      (b) => `
      <label class="bank-row" style="--accent:${b.color}">
        <input type="checkbox" value="${b.name}" checked />
        <span style="color:${b.color}">${b.name}</span>
      </label>`
    )
    .join("");
}

document.getElementById("finish").addEventListener("click", async () => {
  const selected = [...document.querySelectorAll("#bank-list input:checked")].map(
    (i) => i.value
  );
  try {
    await invoke("complete_setup", { form: form(), selectedBanks: selected });
    await invoke("poll_now").catch(() => {});
    window.location.href = "index.html";
  } catch (e) {
    alert(`Kaydedilemedi: ${e}`);
  }
});

show(1);

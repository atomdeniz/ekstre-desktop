const { invoke } = window.__TAURI__.core;

invoke("is_configured").then((ok) => {
  if (!ok) window.location.href = "wizard.html";
});

const cardsEl = document.getElementById("cards");
const emptyEl = document.getElementById("empty");
const statusEl = document.getElementById("status");

if (localStorage.getItem("blur") === "off") document.body.classList.remove("blur");

document.getElementById("toggle").addEventListener("click", () => {
  const blurred = document.body.classList.toggle("blur");
  localStorage.setItem("blur", blurred ? "on" : "off");
});

document.getElementById("settings").addEventListener("click", () => {
  window.location.href = "settings.html";
});

document.getElementById("poll").addEventListener("click", async () => {
  statusEl.textContent = "Taranıyor…";
  try {
    const added = await invoke("poll_now");
    statusEl.textContent = added > 0 ? `${added} yeni ekstre bulundu ✓` : "Yeni ekstre yok.";
    await load();
  } catch (e) {
    statusEl.textContent = `Hata: ${e}`;
  }
});

function badge(daysLeft) {
  if (daysLeft === null || daysLeft === undefined) return "";
  if (daysLeft < 0) return `<span class="badge b-gray">Geçti</span>`;
  if (daysLeft === 0) return `<span class="badge b-red">Bugün</span>`;
  const cls = daysLeft <= 3 ? "b-orange" : "b-green";
  return `<span class="badge ${cls}">${daysLeft} gün</span>`;
}

function cardHtml(c) {
  const min = c.min_due_fmt
    ? `<div><div class="amt-label">Asgari</div><div class="amt-value">${c.min_due_fmt} TL</div></div>`
    : "";
  const stmt = c.statement_date
    ? `<div class="meta"><span>Hesap kesim: ${c.statement_date}</span></div>`
    : "";
  return `
    <div class="card" style="--accent:${c.color}">
      <div class="top"><span class="bank" style="color:${c.color}">${c.bank}</span></div>
      <div class="masked">${c.card_masked ?? "-"}</div>
      <div class="amounts">
        <div><div class="amt-label">Dönem borcu</div><div class="amt-value">${c.total_due_fmt} TL</div></div>
        ${min}
      </div>
      ${stmt}
      <div class="meta"><span>Son ödeme: ${c.due_date}</span>${badge(c.days_left)}</div>
    </div>`;
}

async function load() {
  try {
    const cards = await invoke("get_statements");
    cardsEl.innerHTML = cards.map(cardHtml).join("");
    emptyEl.hidden = cards.length > 0;
  } catch (e) {
    statusEl.textContent = `Yüklenemedi: ${e}`;
  }
}

load();

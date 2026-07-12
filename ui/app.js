const { invoke } = window.__TAURI__.core;

invoke("is_configured").then((ok) => {
  if (!ok) window.location.href = "wizard.html";
});

const cardsEl = document.getElementById("cards");
const calEl = document.getElementById("calendar");
const emptyEl = document.getElementById("empty");
const statusEl = document.getElementById("status");
const tabCardsEl = document.getElementById("tab-cards");
const tabCalEl = document.getElementById("tab-calendar");

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
    await (currentView === "calendar" ? loadCalendar() : load());
  } catch (e) {
    statusEl.textContent = `Hata: ${e}`;
  }
});

let currentView = localStorage.getItem("view") === "calendar" ? "calendar" : "cards";

function setView(view) {
  currentView = view;
  localStorage.setItem("view", view);
  const cal = view === "calendar";
  tabCalEl.classList.toggle("active", cal);
  tabCardsEl.classList.toggle("active", !cal);
  calEl.hidden = !cal;
  cardsEl.hidden = cal;
  emptyEl.hidden = true;
  if (cal) loadCalendar();
  else load();
}

tabCardsEl.addEventListener("click", () => setView("cards"));
tabCalEl.addEventListener("click", () => setView("calendar"));

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
      <div class="top">
        <span class="bank" style="color:${c.color}">${c.bank}</span>
        <button class="dl" data-id="${c.id}" title="Ekstre PDF'ini indir">⭳ PDF</button>
      </div>
      <div class="masked">${c.card_masked ?? "-"}</div>
      <div class="amounts">
        <div><div class="amt-label">Dönem borcu</div><div class="amt-value">${c.total_due_fmt} TL</div></div>
        ${min}
      </div>
      ${stmt}
      <div class="meta"><span>Son ödeme: ${c.due_date}</span>${badge(c.days_left)}</div>
    </div>`;
}

cardsEl.addEventListener("click", async (e) => {
  const btn = e.target.closest(".dl");
  if (!btn) return;
  btn.disabled = true;
  statusEl.textContent = "PDF indiriliyor…";
  try {
    const path = await invoke("download_statement", { id: Number(btn.dataset.id) });
    statusEl.textContent = `İndirilenler'e kaydedildi: ${path}`;
  } catch (err) {
    statusEl.textContent = `Hata: ${err}`;
  } finally {
    btn.disabled = false;
  }
});

async function load() {
  try {
    const cards = await invoke("get_statements");
    cardsEl.innerHTML = cards.map(cardHtml).join("");
    emptyEl.hidden = currentView === "calendar" || cards.length > 0;
  } catch (e) {
    statusEl.textContent = `Yüklenemedi: ${e}`;
  }
}

const TR_MONTHS = ["Ocak","Şubat","Mart","Nisan","Mayıs","Haziran","Temmuz","Ağustos","Eylül","Ekim","Kasım","Aralık"];
const TR_DOW = ["Pzt","Sal","Çar","Per","Cum","Cmt","Paz"];
const pad2 = (n) => String(n).padStart(2, "0");

let calByDay = new Map();
let viewYear, viewMonth;
let selectedIso = null;

function isoToday() {
  const d = new Date();
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

function dayDetail(iso) {
  const items = calByDay.get(iso) || [];
  if (!items.length) return "";
  const rows = items
    .map((c) => `
      <div class="cal-item" style="--accent:${c.color}">
        <div><span class="bank" style="color:${c.color}">${c.bank}</span>
          <span class="masked">${c.card_masked ?? ""}</span></div>
        <div class="amt-value">${c.total_due_fmt} TL</div>
      </div>`)
    .join("");
  return `<div class="cal-day-title">${iso} — son ödeme</div>${rows}`;
}

function cellTags(items) {
  const shown = items
    .slice(0, 2)
    .map((c) => `<span class="cal-tag" style="background:${c.color}">${c.bank}</span>`)
    .join("");
  const more = items.length > 2 ? `<span class="cal-tag more">+${items.length - 2}</span>` : "";
  return shown + more;
}

function renderCalendar() {
  const startDow = (new Date(viewYear, viewMonth, 1).getDay() + 6) % 7;
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const today = isoToday();

  let cells = "";
  for (let i = 0; i < startDow; i++) cells += `<div class="cal-cell empty"></div>`;
  for (let d = 1; d <= daysInMonth; d++) {
    const iso = `${viewYear}-${pad2(viewMonth + 1)}-${pad2(d)}`;
    const items = calByDay.get(iso) || [];
    const cls = ["cal-cell"];
    if (items.length) cls.push("has");
    if (iso === today) cls.push("today");
    if (iso === selectedIso) cls.push("sel");
    cells += `<div class="${cls.join(" ")}" data-iso="${iso}">
      <span class="cal-num">${d}</span>
      <div class="cal-tags">${items.length ? cellTags(items) : ""}</div>
    </div>`;
  }

  calEl.innerHTML = `
    <div class="cal-head">
      <button class="cal-nav" data-nav="-1" title="Önceki ay">‹</button>
      <span class="cal-title">${TR_MONTHS[viewMonth]} ${viewYear}</span>
      <button class="cal-nav" data-nav="1" title="Sonraki ay">›</button>
    </div>
    <div class="cal-detail">${selectedIso ? dayDetail(selectedIso) : ""}</div>
    <div class="cal-grid">
      ${TR_DOW.map((w) => `<div class="cal-dow">${w}</div>`).join("")}
      ${cells}
    </div>`;
}

calEl.addEventListener("click", (e) => {
  const nav = e.target.closest(".cal-nav");
  if (nav) {
    viewMonth += Number(nav.dataset.nav);
    if (viewMonth < 0) { viewMonth = 11; viewYear--; }
    if (viewMonth > 11) { viewMonth = 0; viewYear++; }
    renderCalendar();
    return;
  }
  const cell = e.target.closest(".cal-cell.has");
  if (!cell) return;
  selectedIso = cell.dataset.iso === selectedIso ? null : cell.dataset.iso;
  renderCalendar();
});

async function loadCalendar() {
  try {
    const rows = await invoke("get_calendar");
    calByDay = new Map();
    for (const c of rows) {
      const arr = calByDay.get(c.due_date) || [];
      arr.push(c);
      calByDay.set(c.due_date, arr);
    }
    if (viewYear === undefined) {
      const t = new Date();
      viewYear = t.getFullYear();
      viewMonth = t.getMonth();
    }
    renderCalendar();
  } catch (e) {
    statusEl.textContent = `Yüklenemedi: ${e}`;
  }
}

setView(currentView);

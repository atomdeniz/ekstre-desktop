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

const sunSvg = `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="4.2"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6l1.4 1.4M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/></svg>`;
const moonSvg = `<svg viewBox="0 0 24 24"><path d="M20.5 14.5A8.5 8.5 0 0 1 9.5 3.5a7 7 0 1 0 11 11Z"/></svg>`;
const themeEl = document.getElementById("theme");

function renderThemeIcon() {
  themeEl.innerHTML =
    document.documentElement.getAttribute("data-theme") === "dark" ? sunSvg : moonSvg;
}
renderThemeIcon();

themeEl.addEventListener("click", () => {
  const dark = document.documentElement.getAttribute("data-theme") === "dark";
  const next = dark ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", next);
  localStorage.setItem("theme", next);
  renderThemeIcon();
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

const MONTHS_SHORT = ["Oca","Şub","Mar","Nis","May","Haz","Tem","Ağu","Eyl","Eki","Kas","Ara"];
const HEAT_HORIZON = 30;

function fmtDate(iso) {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso || "");
  return m ? `${Number(m[3])} ${MONTHS_SHORT[Number(m[2]) - 1]}` : (iso || "");
}

function urgency(d) {
  if (d === null || d === undefined) return "";
  if (d < 0) return "past";
  if (d === 0) return "now";
  if (d <= 3) return "warn";
  return "ok";
}

function heatPct(d) {
  if (d === null || d === undefined) return 0;
  if (d < 0) return 100;
  return Math.round(Math.min(1, (HEAT_HORIZON - d) / HEAT_HORIZON) * 100);
}

function daysLabel(d) {
  if (d === null || d === undefined) return "";
  if (d < 0) return "Geçti";
  if (d === 0) return "Bugün";
  return `${d} gün`;
}

const dlSvg = `<svg viewBox="0 0 24 24"><path d="M12 3v12"/><path d="m7 11 5 5 5-5"/><path d="M5 20h14"/></svg>`;

function cardHtml(c, i) {
  const min = c.min_due_fmt ? `<span class="min">Asgari <b>${c.min_due_fmt}</b> ₺</span>` : "";
  const kesim = c.statement_date ? `Kesim ${fmtDate(c.statement_date)} · ` : "";
  return `
    <article class="card ${urgency(c.days_left)}" style="--accent:${c.color};--i:${i}">
      <div class="card-head">
        <span class="bank" style="color:${c.color}">${c.bank}</span>
        <span class="masked">${c.card_masked ?? "-"}</span>
        <button class="dl" data-id="${c.id}" title="Ekstre PDF'ini indir" aria-label="Ekstre PDF'ini indir">${dlSvg}</button>
      </div>
      <div class="hero">
        <span class="amt-value">${c.total_due_fmt}<span class="cur"> ₺</span></span>
        ${min}
      </div>
      <div class="track" aria-hidden="true"><span class="track-fill" style="width:${heatPct(c.days_left)}%"></span></div>
      <div class="card-foot">
        <span class="due">${kesim}Son ödeme ${fmtDate(c.due_date)}</span>
        <span class="days">${daysLabel(c.days_left)}</span>
      </div>
    </article>`;
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
        <div class="amt-value">${c.total_due_fmt}<span class="cur"> ₺</span></div>
      </div>`)
    .join("");
  return `<div class="cal-day-title">${fmtDate(iso)} — son ödeme</div>${rows}`;
}

function tag(c) {
  return `<span class="cal-tag" style="background:${c.color}">${c.bank}</span>`;
}

function cellTags(items) {
  if (items.length <= 2) return items.map(tag).join("");
  return tag(items[0]) + `<span class="cal-tag more">+${items.length - 1}</span>`;
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
    <div class="cal-dow-row">${TR_DOW.map((w) => `<div class="cal-dow">${w}</div>`).join("")}</div>
    <div class="cal-grid">${cells}</div>`;
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

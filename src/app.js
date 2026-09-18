// Fay — config-driven hub logic. Rendering of the Heart lives in heart.js.
// The UI is rendered entirely from the config. Never hardcode tiles here.

const els = {
  scenes: document.getElementById("scenes"),
  apps: document.getElementById("apps"),
  system: document.getElementById("system"),
  groups: document.getElementById("groups"),
  folder: document.getElementById("folder"),
  folderLabel: document.getElementById("folder-label"),
  brand: document.getElementById("brand"),
  status: document.getElementById("status"),
  monitors: document.getElementById("monitors"),
  clock: document.getElementById("clock"),
  core: document.getElementById("core"),
  canvas: document.getElementById("heart"),
  filter: document.getElementById("filter"),
};

const tauri = window.__TAURI__ || null;
const invoke =
  tauri && tauri.core && tauri.core.invoke ? tauri.core.invoke.bind(tauri.core) : null;

// ---- tile kinds -----------------------------------------------------------
// launch (default): target = exe / command / .lnk / URL (may contain {query})
// system: action = lock sleep hibernate restart shutdown logoff recycle darkmode
// media:  action = playpause next prev stop mute volup voldown
// snippet: text = pasted into the foreground app (paste:false = copy only)
// multi:   actions = [ step, {"wait": ms}, … ] run in order (any kind per step)
// close:   closes = ["Discord", "zen!"] processes to close ("!" = force)
// folder:  children = [ tiles… ] opens a sub-deck
const KINDS = new Set(["launch", "system", "media", "snippet", "multi", "close", "folder"]);
const SYSTEM_ACTIONS = new Set(["lock", "sleep", "hibernate", "restart", "shutdown", "logoff", "recycle", "darkmode"]);
const MEDIA_ACTIONS = new Set(["playpause", "play", "pause", "next", "prev", "previous", "stop", "mute", "volup", "voldown"]);
const CONFIRM_ACTIONS = new Set(["restart", "shutdown", "logoff", "hibernate"]);

function kindOf(item) {
  if (Array.isArray(item.children)) return "folder";
  if (Array.isArray(item.actions)) return "multi";
  return item.kind && KINDS.has(item.kind) ? item.kind : "launch";
}
const isQuicklink = (item) => kindOf(item) === "launch" && /\{query\}/.test(item.target || "");
const keywordOf = (item) => String(item.keyword || item.id || "").toLowerCase();
const needsConfirm = (item) =>
  typeof item.confirm === "boolean" ? item.confirm : kindOf(item) === "system" && CONFIRM_ACTIONS.has(item.action);
const canClose = (item) => Array.isArray(item.closes) && item.closes.length > 0;

// The payload the backend's `fire` command / hotkey bindings understand.
function actionOf(item, query) {
  let target = item.target || null;
  if (target && query != null) target = target.replace(/\{query\}/g, encodeURIComponent(query.trim()));
  const kind = kindOf(item);
  return {
    kind,
    target,
    elevated: !!item.elevated,
    audioOut: item.audioOut || null,
    action: item.action || null,
    text: typeof item.text === "string" ? item.text : null,
    paste: item.paste !== false,
    steps: kind === "multi" ? item.actions.map(stepOf) : [],
    wait: item.wait != null ? Number(item.wait) : null,
    closes: kind === "close" && canClose(item) ? item.closes.map(String) : [],
  };
}
const stepOf = (s) => (s && s.wait != null && !s.kind && !s.target ? { kind: "wait", wait: Number(s.wait) } : actionOf(s || {}));
// The teardown counterpart of a scene: close what it opened.
const closeActionOf = (item) => ({ kind: "close", closes: item.closes.map(String) });

// Walk every tile in the config, including folder children (depth-first).
function walkTiles(cfg, fn) {
  const visit = (list, depth) => {
    for (const t of list || []) {
      if (!t || typeof t !== "object") continue;
      fn(t, depth);
      if (Array.isArray(t.children)) visit(t.children, depth + 1);
    }
  };
  for (const g of ["scenes", "apps", "system"]) visit(cfg[g], 0);
}
function allTiles(cfg) { const out = []; walkTiles(cfg, (t) => out.push(t)); return out; }

// ---- open / rest state ----------------------------------------------------
function openDeck() { document.body.classList.add("open"); refreshRunning(); }
function closeDeck() { document.body.classList.remove("open"); leaveFolders(); setFilter(""); }
function isOpen() { return document.body.classList.contains("open"); }

// ---- folders (sub-decks) --------------------------------------------------
const folderStack = [];
function openFolder(item) {
  folderStack.push(item);
  renderFolder();
  setFilter("");
  refreshRunning();
}
function closeFolder() {
  folderStack.pop();
  renderFolder();
  setFilter("");
}
function leaveFolders() { folderStack.length = 0; renderFolder(); }
function renderFolder() {
  const top = folderStack[folderStack.length - 1];
  document.body.classList.toggle("in-folder", !!top);
  els.folder.innerHTML = "";
  if (!top) return;
  els.folderLabel.textContent = folderStack.map((f) => f.name).join(" › ");
  (top.children || []).forEach((c) => els.folder.appendChild(tile(c, "app")));
}
const inFolder = () => folderStack.length > 0;

// ---- filter bar: type-to-filter, quicklinks, calculator, conversions --------
let filterText = "";
let currentQuicklink = null; // { item, query }
let currentAnswer = null;    // string result of a calculation / conversion
let quicklinks = [];         // tiles whose target contains {query} (anywhere in the config)
// Only tiles in the visible container count (the folder when one is open).
const visibleTiles = () =>
  [...(inFolder() ? els.folder : els.groups).querySelectorAll(".tile:not(.is-hidden)")];

function matchQuicklink(q) {
  const m = q.match(/^(\S+)\s(.*)$/);
  if (!m) return null;
  const kw = m[1].toLowerCase();
  const item = quicklinks.find((i) => keywordOf(i) === kw);
  return item ? { item, query: m[2] } : null;
}

function setFilter(q) {
  filterText = q.replace(/^\s+/, "");
  const raw = filterText;
  currentQuicklink = matchQuicklink(raw);
  currentAnswer = currentQuicklink ? null : answerFor(raw);

  if (!raw) {
    els.filter.textContent = "";
  } else if (currentQuicklink) {
    els.filter.innerHTML =
      `› <span class="filter__kw">${escapeHtml(currentQuicklink.item.name)}</span> ▸ ${escapeHtml(currentQuicklink.query)}`;
  } else if (currentAnswer != null) {
    els.filter.innerHTML =
      `› ${escapeHtml(raw)} <span class="filter__ans">= ${escapeHtml(currentAnswer)}</span>` +
      `<span class="filter__hint">Enter copies</span>`;
  } else {
    els.filter.textContent = `› ${raw}`;
  }
  els.filter.classList.toggle("is-active", !!raw);

  const needle = raw.toLowerCase();
  for (const t of document.querySelectorAll(".tile")) {
    const name = (t.dataset.name || "").toLowerCase();
    const hide = currentQuicklink
      ? t !== currentQuicklink.item._el
      : !!needle && !name.includes(needle);
    t.classList.toggle("is-hidden", hide);
  }
  renumber();
}

// Fire the tile behind a DOM element, honoring Shift = teardown for scenes.
function fireEl(el, shift) {
  const item = el && el._item;
  if (!item) return;
  if (shift && canClose(item)) { fireClose(item); return; }
  el.click();
}

// Label the first nine visible tiles 1–9 so a digit fires them.
function renumber() {
  document.querySelectorAll(".tile__idx").forEach((s) => (s.textContent = ""));
  visibleTiles().slice(0, 9).forEach((t, i) => {
    const s = t.querySelector(".tile__idx");
    if (s) s.textContent = String(i + 1);
  });
}

// A calculation ("= 1440*0.62", "sqrt(2)*3") or a conversion ("5 mi to km").
// Anything that isn't one returns null and is treated as a plain name filter.
function answerFor(text) {
  const expr = text.replace(/^=\s*/, "").trim();
  if (!expr) return null;
  const conv = convertUnits(expr);
  if (conv != null) return conv;
  const forced = text.trimStart().startsWith("=");
  const looksMath = /^[\d\s.+\-*/^%(),a-z]+$/i.test(expr) && /\d/.test(expr) && (forced || /[+\-*/^%(]/.test(expr));
  if (!looksMath) return null;
  try {
    const v = evalMath(expr);
    return v == null ? null : fmtNum(v);
  } catch (e) {
    return null;
  }
}

function fmtNum(n) {
  if (!Number.isFinite(n)) return null;
  const s = String(Number(n.toPrecision(12)));
  return s;
}

// Tiny safe expression evaluator (no eval): + - * / ^ %, parentheses, unary
// minus, constants pi/e, functions sqrt abs sin cos tan log ln round floor
// ceil min max.
function evalMath(src) {
  const toks = [];
  const re = /\s*(?:(\d+\.?\d*(?:e[+-]?\d+)?|\.\d+)|([a-z]+)|([+\-*/^%(),]))/giy;
  let m;
  re.lastIndex = 0;
  while (re.lastIndex < src.length && (m = re.exec(src))) {
    if (m[1] != null) toks.push({ t: "n", v: Number(m[1]) });
    else if (m[2] != null) toks.push({ t: "id", v: m[2].toLowerCase() });
    else toks.push({ t: m[3] });
  }
  if (re.lastIndex < src.length) return null; // junk in the input
  let i = 0;
  const peek = () => toks[i];
  const take = (t) => { const k = toks[i]; if (!k || (t && k.t !== t)) throw new Error("syntax"); i++; return k; };
  const FN = {
    sqrt: Math.sqrt, abs: Math.abs, sin: Math.sin, cos: Math.cos, tan: Math.tan,
    log: Math.log10, ln: Math.log, round: Math.round, floor: Math.floor, ceil: Math.ceil,
    min: Math.min, max: Math.max,
  };
  const CONST = { pi: Math.PI, e: Math.E };
  function expr() {
    let v = term();
    while (peek() && (peek().t === "+" || peek().t === "-")) v = take().t === "+" ? v + term() : v - term();
    return v;
  }
  function term() {
    let v = unary();
    while (peek() && (peek().t === "*" || peek().t === "/" || peek().t === "%")) {
      const op = take().t, r = unary();
      v = op === "*" ? v * r : op === "/" ? v / r : v % r;
    }
    return v;
  }
  function unary() { return peek() && peek().t === "-" ? (take(), -unary()) : power(); }
  function power() { const b = primary(); if (peek() && peek().t === "^") { take(); return Math.pow(b, unary()); } return b; }
  function primary() {
    const k = take();
    if (k.t === "n") return k.v;
    if (k.t === "(") { const v = expr(); take(")"); return v; }
    if (k.t === "id") {
      if (peek() && peek().t === "(") {
        take("(");
        const args = [expr()];
        while (peek() && peek().t === ",") { take(); args.push(expr()); }
        take(")");
        if (!FN[k.v]) throw new Error("fn");
        return FN[k.v](...args);
      }
      if (k.v in CONST) return CONST[k.v];
      throw new Error("ident");
    }
    throw new Error("syntax");
  }
  const v = expr();
  if (i !== toks.length) throw new Error("trailing");
  return v;
}

// "5 mi to km", "72 f in c", "3.5 kg to lb", "1 gb to mb"
const UNITS = (() => {
  const u = {};
  const add = (dim, factor, ...names) => names.forEach((n) => (u[n] = { dim, factor }));
  add("len", 0.001, "mm"); add("len", 0.01, "cm"); add("len", 1, "m", "meter", "meters", "metre", "metres");
  add("len", 1000, "km", "kilometer", "kilometers"); add("len", 0.0254, "in", "inch", "inches");
  add("len", 0.3048, "ft", "foot", "feet"); add("len", 0.9144, "yd", "yard", "yards"); add("len", 1609.344, "mi", "mile", "miles");
  add("mass", 0.001, "g", "gram", "grams"); add("mass", 1, "kg", "kgs", "kilogram", "kilograms");
  add("mass", 0.45359237, "lb", "lbs", "pound", "pounds"); add("mass", 0.028349523125, "oz", "ounce", "ounces"); add("mass", 6.35029318, "st", "stone");
  add("vol", 0.001, "ml"); add("vol", 1, "l", "liter", "liters", "litre", "litres"); add("vol", 3.785411784, "gal", "gallon", "gallons");
  add("vol", 0.946352946, "qt", "quart"); add("vol", 0.473176473, "pt", "pint"); add("vol", 0.2365882365, "cup", "cups"); add("vol", 0.0295735296, "floz");
  add("time", 0.001, "ms"); add("time", 1, "s", "sec", "secs", "second", "seconds"); add("time", 60, "min", "mins", "minute", "minutes");
  add("time", 3600, "h", "hr", "hrs", "hour", "hours"); add("time", 86400, "d", "day", "days"); add("time", 604800, "wk", "week", "weeks");
  add("data", 1, "b", "byte", "bytes"); add("data", 1e3, "kb"); add("data", 1e6, "mb"); add("data", 1e9, "gb"); add("data", 1e12, "tb");
  add("data", 1024, "kib"); add("data", 1024 ** 2, "mib"); add("data", 1024 ** 3, "gib"); add("data", 1024 ** 4, "tib");
  add("speed", 1, "mps", "m/s"); add("speed", 1 / 3.6, "kmh", "km/h", "kph"); add("speed", 0.44704, "mph"); add("speed", 0.514444, "kn", "knot", "knots");
  add("area", 1, "m2", "sqm"); add("area", 1e6, "km2"); add("area", 0.09290304, "ft2", "sqft"); add("area", 4046.8564224, "acre", "acres"); add("area", 10000, "ha", "hectare");
  add("temp", 1, "c", "°c", "celsius"); add("temp", 1, "f", "°f", "fahrenheit"); add("temp", 1, "k", "kelvin");
  return u;
})();

function convertUnits(text) {
  const m = text.toLowerCase().match(/^(-?\d+(?:\.\d+)?)\s*([a-z°\/0-9]+)\s+(?:to|in|as|->)\s+([a-z°\/0-9]+)$/);
  if (!m) return null;
  const n = Number(m[1]), a = UNITS[m[2]], b = UNITS[m[3]];
  if (!a || !b || a.dim !== b.dim) return null;
  let out;
  if (a.dim === "temp") {
    const toC = { c: (v) => v, f: (v) => (v - 32) * 5 / 9, k: (v) => v - 273.15 };
    const fromC = { c: (v) => v, f: (v) => v * 9 / 5 + 32, k: (v) => v + 273.15 };
    const key = (s) => s.replace("°", "")[0];
    out = fromC[key(m[3])](toC[key(m[2])](n));
  } else {
    out = (n * a.factor) / b.factor;
  }
  const s = fmtNum(Number(out.toPrecision(6)));
  return s == null ? null : `${s} ${m[3]}`;
}

async function copyAnswer() {
  const text = currentAnswer;
  if (text == null) return;
  try {
    if (invoke) await invoke("set_clipboard", { text });
    else await navigator.clipboard.writeText(text);
    flash(`copied ${text}`);
  } catch (e) { warn(`copy: ${e}`); }
}

// ---- status line (the footer) ---------------------------------------------
// flash = transient info; warn = a problem the user should actually see.
function setStatus(msg, cls, ms) {
  els.status.textContent = msg;
  els.status.className = cls || "";
  clearTimeout(setStatus._t);
  setStatus._t = setTimeout(() => {
    els.status.textContent = "ready";
    els.status.className = "";
  }, ms);
}
const flash = (m) => setStatus(m, "is-flash", 1600);
const warn = (m) => { console.warn(m); setStatus(m, "is-warn", 9000); };

// ---- firing tiles ---------------------------------------------------------
async function fire(item, query) {
  const kind = kindOf(item);
  if (kind === "folder") { openFolder(item); return; }
  if (kind === "launch" && !item.target) return;
  const payload = actionOf(item, query);
  await send(payload, `→ ${item.name}${query ? ` ▸ ${query.trim()}` : ""}${item.elevated ? " (admin)" : ""}`, item.name);
}

// Scene teardown: close the processes listed in `closes` (Shift+click / Shift+digit / ×).
async function fireClose(item) {
  if (!canClose(item)) return;
  await send(closeActionOf(item), `× ${item.name}`, item.name);
}

const pastes = (a) => (a.kind === "snippet" && a.paste) || (a.steps || []).some(pastes);
async function send(payload, label, name) {
  flash(label);
  if (!invoke) {
    console.log("[preview] would fire:", payload);
    flash(`(preview) ${name}`);
    return;
  }
  // A snippet pastes into whatever is in front: get out of the way first.
  if (pastes(payload)) await invoke("hide_window").catch(() => {});
  try {
    const w = await invoke("fire", { action: payload });
    if (w) warn(w);
  } catch (e) {
    warn(`${name}: ${e}`);
  }
}

// Destructive tiles (shutdown, restart…) need a second press within 3 s.
function arm(el) {
  el.classList.add("is-confirm");
  el.dataset.armed = "1";
  clearTimeout(el._armT);
  el._armT = setTimeout(() => disarm(el), 3000);
  flash("press again to confirm");
}
function disarm(el) {
  el.classList.remove("is-confirm");
  delete el.dataset.armed;
  clearTimeout(el._armT);
}

function tile(item, kind) {
  const el = document.createElement("button");
  const k = kindOf(item);
  el.className = `tile tile--${kind} tile--k-${k}`;
  el.dataset.name = item.name || "";
  el.dataset.proc = processNameFor(item);
  item._el = el;
  el._item = item;
  const badge = item.elevated ? `<span class="tile__badge">ADMIN</span>` : "";
  const key = item.hotkey ? `<span class="tile__key">${escapeHtml(item.hotkey)}</span>` : "";
  const ql = isQuicklink(item) ? `<span class="tile__key tile__kw">${escapeHtml(keywordOf(item))} …</span>` : "";
  const close = canClose(item) ? `<span class="tile__close" title="Close ${escapeHtml(item.closes.join(", "))} (Shift+click)">×</span>` : "";
  const hint = item.hint || (k === "folder" ? `${item.children.length} item${item.children.length === 1 ? "" : "s"}` : "");
  el.innerHTML = `
    ${badge}${key || ql}${close}
    <span class="tile__glyph">${escapeHtml(item.glyph || (k === "folder" ? "▤" : "○"))}</span>
    <div>
      <div class="tile__name"><span class="tile__idx"></span>${escapeHtml(item.name)}</div>
      ${hint ? `<div class="tile__hint">${escapeHtml(hint)}</div>` : ""}
    </div>`;
  el.addEventListener("click", (e) => {
    if (e.shiftKey && canClose(item)) { fireClose(item); return; }
    if (isQuicklink(item)) {
      const q = currentQuicklink && currentQuicklink.item === item ? currentQuicklink.query : null;
      if (q == null) { setFilter(`${keywordOf(item)} `); return; } // prompt for the query
      fire(item, q);
      return;
    }
    if (needsConfirm(item) && el.dataset.armed !== "1") { arm(el); return; }
    disarm(el);
    fire(item);
  });
  const x = el.querySelector(".tile__close");
  if (x) x.addEventListener("click", (e) => { e.stopPropagation(); fireClose(item); });
  loadIcon(el, item);
  return el;
}

// Swap the glyph for the app's real icon: an explicit `icon` (URL/data URL)
// wins; otherwise ask the backend to extract it from the target. Failures keep
// the glyph (protocol targets like steam:// have no icon).
function loadIcon(el, item) {
  const glyph = el.querySelector(".tile__glyph");
  const show = (src) => { glyph.innerHTML = `<img class="tile__icon" src="${escapeHtml(src)}" alt="">`; };
  if (item.icon) { show(item.icon); return; }
  if (!invoke || kindOf(item) !== "launch" || !item.target || isQuicklink(item)) return;
  invoke("get_app_icon", { target: item.target }).then(show).catch(() => {});
}

// ---- running-app indicator -------------------------------------------------
// Guess the process name from the target (explicit `process` wins):
//   ...\zen.exe -> zen, Discord.lnk -> discord, taskmgr -> taskmgr, steam:// -> steam
function processNameFor(item) {
  if (item.process) return String(item.process).toLowerCase();
  if (kindOf(item) !== "launch") return "";
  const t = String(item.target || "");
  const proto = t.match(/^([a-z][a-z0-9+.-]*):/i);
  if (proto && !/^[a-z]:\\/i.test(t)) return proto[1].toLowerCase();
  const base = t.split(/[\\/]/).pop() || "";
  return base.replace(/\.(exe|lnk|bat|cmd)$/i, "").toLowerCase();
}

async function refreshRunning() {
  if (!invoke) return;
  try {
    const running = new Set(await invoke("list_running"));
    for (const t of document.querySelectorAll(".tile")) {
      t.classList.toggle("is-running", !!t.dataset.proc && running.has(t.dataset.proc));
    }
  } catch (e) { /* non-fatal */ }
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

function startClock() {
  const tick = () => {
    els.clock.textContent = new Date()
      .toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      .toUpperCase();
  };
  tick();
  setInterval(tick, 15000);
}

async function refreshMonitors() {
  if (!invoke) { els.monitors.textContent = "⧉ preview"; return; }
  try {
    const mons = await invoke("list_monitors");
    const w = mons.reduce((a, m) => a + m.width, 0);
    const h = mons.reduce((a, m) => Math.max(a, m.height), 0);
    els.monitors.textContent =
      `⧉ ${mons.length} display${mons.length === 1 ? "" : "s"} · ${w}×${h}` +
      (window.__fayHost ? ` · ${window.__fayHost}` : "");
  } catch (e) {
    els.monitors.textContent = "⧉ —";
  }
}

async function applyAccent(value) {
  let accent = value || "#34e6c6";
  if (accent === "auto" && invoke) {
    try { accent = await invoke("get_accent_color"); } catch (e) { accent = "#34e6c6"; }
  }
  document.documentElement.style.setProperty("--accent", accent);
  if (window.Heart) window.Heart.setAccent(accent);
}

function wireInput() {
  els.core.addEventListener("click", openDeck);
  // clicking the empty canvas (backdrop) while open returns to rest
  els.canvas.addEventListener("mousedown", () => { if (isOpen()) closeDeck(); });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (isOpen()) { if (filterText) setFilter(""); else if (inFolder()) closeFolder(); else closeDeck(); }
      else if (invoke) invoke("hide_window");
      return;
    }
    if (!isOpen()) {
      if (e.key === "Enter") openDeck();
      return;
    }
    // ---- deck is open ----
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey;
    // Digits fire tiles only while nothing is typed; once the filter has text
    // (or starts with "=") they are part of it, so "= 1440*0.62" works.
    // Shift+digit on a scene with `closes` tears it down instead.
    if (/^[1-9!@#$%^&*(]$/.test(e.key) && plain && !filterText) {
      const n = /^[1-9]$/.test(e.key) ? Number(e.key) : ")!@#$%^&*(".indexOf(e.key);
      const t = visibleTiles()[n - 1];
      if (t) fireEl(t, e.shiftKey);
      e.preventDefault();
      return;
    }
    if (e.key === "Backspace") {
      if (!filterText && inFolder()) closeFolder(); else setFilter(filterText.slice(0, -1));
      e.preventDefault();
      return;
    }
    if (e.key === "Enter") {
      if (currentAnswer != null) { copyAnswer(); e.preventDefault(); return; }
      if (currentQuicklink) { fire(currentQuicklink.item, currentQuicklink.query); e.preventDefault(); return; }
      if (filterText) { const t = visibleTiles()[0]; if (t) fireEl(t, e.shiftKey); e.preventDefault(); return; }
      if (e.shiftKey && document.activeElement && document.activeElement._item) { fireEl(document.activeElement, true); e.preventDefault(); }
      return; // no filter: native Enter on a focused tile
    }
    if (e.key.length === 1 && plain) {
      setFilter(filterText + e.key);
      e.preventDefault();
      return;
    }
    if (e.key.startsWith("Arrow")) {
      const tiles = visibleTiles();
      if (!tiles.length) return;
      const cur = tiles.indexOf(document.activeElement);
      let next = cur < 0 ? 0 : cur;
      if (e.key === "ArrowRight" || e.key === "ArrowDown") next = Math.min(cur + 1, tiles.length - 1);
      if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = Math.max(cur - 1, 0);
      tiles[next].focus();
      e.preventDefault();
    }
  });

  // re-read the Windows accent when Fay regains focus (it may have changed)
  window.addEventListener("focus", () => {
    if (window.__fayAccent === "auto") applyAccent("auto");
  });
  // losing focus hides the window (backend); return the deck to rest so the
  // next summon starts clean and the running-poll stops.
  window.addEventListener("blur", () => { if (isOpen()) closeDeck(); });
}

// ---- config ---------------------------------------------------------------
// Installed builds load the user's editable copy (app config dir), seeded from
// the bundled default on first run. Browser preview just uses the bundled file.
async function loadConfig() {
  const bundled = async () => {
    const r = await fetch("apps.config.json", { cache: "no-store" });
    if (!r.ok) throw new Error(`bundled config ${r.status}`);
    return r.text();
  };
  if (invoke) {
    let text = null;
    try { text = await invoke("load_config"); } catch (e) { warn(`config read failed: ${e}`); }
    if (text) {
      try {
        return { cfg: JSON.parse(text), seeded: false };
      } catch (e) {
        // Never touch the user's file; run on the bundled default so Fay stays usable.
        const where = jsonErrorWhere(text, e);
        warn(`config syntax error${where ? ` at ${where}` : ""}: ${shortJsonError(e)} — using defaults until it's fixed`);
        return { cfg: JSON.parse(await bundled()), seeded: false, broken: true };
      }
    }
    const txt = await bundled();
    try {
      const path = await invoke("save_config", { text: txt });
      return { cfg: JSON.parse(txt), seeded: true, path };
    } catch (e) {
      warn(`could not create user config: ${e}`);
      return { cfg: JSON.parse(txt), seeded: false };
    }
  }
  return { cfg: JSON.parse(await bundled()), seeded: false };
}

// "line 23, col 5" from a JSON.parse error (V8 reports either line/column or
// a character position; the latter is mapped onto the text).
function jsonErrorWhere(text, err) {
  const msg = String(err && err.message || err);
  let m = msg.match(/line (\d+) column (\d+)/i);
  if (m) return `line ${m[1]}, col ${m[2]}`;
  m = msg.match(/position (\d+)/i);
  if (m) {
    const pos = Number(m[1]);
    const before = text.slice(0, pos);
    return `line ${before.split("\n").length}, col ${pos - before.lastIndexOf("\n")}`;
  }
  return null;
}
function shortJsonError(err) {
  const msg = String(err && err.message || err);
  return msg.replace(/ in JSON at position.*$/i, "").replace(/ \(line \d+ column \d+\)$/i, "").slice(0, 80);
}

// Preset tile packs (src/packs.json): `packs: ["power", "media"]` appends each
// pack's tiles to its group. A pack tile is skipped if the config already has
// a tile with the same id (so any preset can be overridden).
async function loadPacks() {
  try {
    const r = await fetch("packs.json", { cache: "no-store" });
    return r.ok ? await r.json() : {};
  } catch (e) { return {}; }
}
function applyPacks(cfg, packs) {
  if (!Array.isArray(cfg.packs)) return;
  const have = new Set([...(cfg.scenes || []), ...(cfg.apps || []), ...(cfg.system || [])].map((t) => t && t.id));
  for (const name of cfg.packs) {
    const pk = packs[name];
    if (!pk || !Array.isArray(pk.tiles)) continue;
    const group = ["scenes", "apps", "system"].includes(pk.group) ? pk.group : "system";
    cfg[group] = cfg[group] || [];
    for (const t of pk.tiles) if (!have.has(t.id)) { cfg[group].push({ ...t }); have.add(t.id); }
  }
}

// Structural checks with useful messages (a syntax error is caught earlier).
function validateConfig(cfg, packs) {
  const p = [];
  const isObj = (v) => v && typeof v === "object" && !Array.isArray(v);
  if (!isObj(cfg)) return ["config root must be an object { … }"];
  if ("app" in cfg && !isObj(cfg.app)) p.push(`"app" must be an object`);
  const ids = new Set();
  // One tile action (a tile itself, or a step of a multi-action tile).
  const checkAction = (t, who, isStep) => {
    if (t.kind && !KINDS.has(t.kind)) p.push(`${who}: unknown kind "${t.kind}" (launch, system, media, snippet, multi, close)`);
    const k = kindOf(t);
    if (k === "launch" && !t.target) p.push(`${who} needs a "target"`);
    if (k === "system" && !SYSTEM_ACTIONS.has(t.action)) p.push(`${who}: system action must be one of ${[...SYSTEM_ACTIONS].join("/")}`);
    if (k === "media" && !MEDIA_ACTIONS.has(t.action)) p.push(`${who}: media action must be one of playpause/next/prev/stop/mute/volup/voldown`);
    if (k === "snippet" && typeof t.text !== "string") p.push(`${who} needs "text"`);
    if (k === "close" && !canClose(t)) p.push(`${who} needs "closes": ["process", …]`);
    if (k === "multi") {
      if (!t.actions.length) p.push(`${who}: "actions" is empty`);
      t.actions.forEach((s, i) => {
        const sw = `${who} step ${i + 1}`;
        if (!isObj(s)) { p.push(`${sw} must be an object`); return; }
        if (s.wait != null && (typeof s.wait !== "number" || s.wait < 0)) p.push(`${sw}: "wait" must be milliseconds`);
        else if (s.wait == null || s.kind || s.target) checkAction(s, sw, true);
      });
    }
    if (k === "folder" && isStep) p.push(`${who}: a step can't be a folder`);
    if (t.closes != null && (!Array.isArray(t.closes) || !t.closes.every((c) => typeof c === "string"))) p.push(`${who}: "closes" must be an array of process names`);
  };
  const checkTile = (t, at) => {
    if (!isObj(t)) { p.push(`${at} must be an object`); return; }
    const who = t.id ? `"${t.id}"` : at;
    if (!t.id) p.push(`${at} needs an "id"`);
    else if (ids.has(t.id)) p.push(`duplicate id ${who}`);
    else ids.add(t.id);
    if (!t.name) p.push(`${who} needs a "name"`);
    if (t.hotkey != null && typeof t.hotkey !== "string") p.push(`${who}: "hotkey" must be a string`);
    if (kindOf(t) === "folder") {
      if (t.hotkey) p.push(`${who}: a folder can't have a hotkey`);
      t.children.forEach((c, i) => checkTile(c, `${who} › [${i}]`));
    } else {
      checkAction(t, who, false);
    }
  };
  for (const g of ["scenes", "apps", "system"]) {
    if (!(g in cfg)) continue;
    if (!Array.isArray(cfg[g])) { p.push(`"${g}" must be an array [ … ]`); continue; }
    cfg[g].forEach((t, i) => checkTile(t, `${g}[${i}]`));
  }
  if ("commands" in cfg && typeof cfg.commands !== "boolean" && typeof cfg.commands !== "string") p.push(`"commands" must be true, false or a folder path`);
  if ("packs" in cfg) {
    if (!Array.isArray(cfg.packs)) p.push(`"packs" must be an array`);
    else for (const n of cfg.packs) if (!packs[n]) p.push(`unknown pack "${n}" (have: ${Object.keys(packs).filter((k) => !k.startsWith("_")).join(", ")})`);
  }
  if ("machines" in cfg && !isObj(cfg.machines)) p.push(`"machines" must be an object`);
  return p;
}

// Per-machine overrides: cfg.machines[<HOSTNAME>].tiles[<id>] is merged onto
// the tile with that id, so one config can carry different paths per PC.
async function applyMachineProfile(cfg) {
  if (!invoke) return null;
  let host = null;
  try { host = (await invoke("get_hostname")) || null; } catch (e) { return null; }
  if (!host || !cfg.machines) return host;
  const key = Object.keys(cfg.machines).find((k) => k.toUpperCase() === host);
  const prof = key && cfg.machines[key];
  if (!prof || !prof.tiles) return host;
  for (const item of allTiles(cfg)) {
    const o = prof.tiles[item.id];
    if (o && typeof o === "object") Object.assign(item, o);
  }
  return host;
}

// The commands folder: every .ps1 / .bat / .cmd / .exe / .lnk in it becomes a
// tile in the `system` group. `commands: false` turns it off; a string points
// at a custom folder (default: <config dir>\commands, see tray menu).
async function applyCommandsFolder(cfg) {
  if (!invoke || cfg.commands === false) return;
  const dir = typeof cfg.commands === "string" ? cfg.commands : null;
  let list = [];
  try { list = await invoke("list_commands", { dir }); } catch (e) { warn(`commands folder: ${e}`); return; }
  if (!list.length) return;
  const glyph = { ps1: "›_", bat: "▮", cmd: "▮", exe: "▪", lnk: "↗" };
  cfg.system = cfg.system || [];
  for (const c of list) {
    cfg.system.push({
      id: `cmd-${c.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
      name: c.name, glyph: glyph[c.ext] || "▪", target: c.path, hint: "commands folder",
    });
  }
}

async function main() {
  startClock();
  wireInput();
  let showNow = false;
  try {
    const [{ cfg, seeded, path, broken }, packs] = await Promise.all([loadConfig(), loadPacks()]);
    const problems = validateConfig(cfg, packs);
    applyPacks(cfg, packs);
    await applyCommandsFolder(cfg);
    window.__fayHost = await applyMachineProfile(cfg);
    quicklinks = allTiles(cfg).filter(isQuicklink);
    const app = cfg.app || {};
    if (app.name) els.brand.textContent = app.name.toUpperCase();

    if (typeof app.backdrop === "number") {
      document.documentElement.style.setProperty("--backdrop-alpha", String(app.backdrop));
    }
    window.__fayAccent = app.accent || "#34e6c6";
    applyAccent(window.__fayAccent);

    if (app.hotkey && invoke) {
      invoke("set_summon_hotkey", { accelerator: app.hotkey }).catch((e) => warn(`summon hotkey: ${e}`));
    }
    // Mouse-button summon (e.g. "Ctrl+Mouse5"); an empty/absent value leaves it off.
    if (invoke && "mouseSummon" in app) {
      invoke("set_mouse_summon", { spec: app.mouseSummon || null }).catch((e) => warn(`mouse summon: ${e}`));
    }
    // Which monitor Fay appears on: "cursor" (default) or "current".
    if (invoke && app.summonOn) {
      invoke("set_summon_monitor", { mode: String(app.summonOn) }).catch(() => {});
    }
    if (typeof app.autostart === "boolean" && invoke) {
      invoke("set_autostart", { enabled: app.autostart }).catch((e) => warn(`autostart: ${e}`));
    }

    (cfg.scenes || []).forEach((s) => els.scenes.appendChild(tile(s, "scene")));
    (cfg.apps || []).forEach((a) => els.apps.appendChild(tile(a, "app")));
    (cfg.system || []).forEach((a) => els.system.appendChild(tile(a, "system")));
    renumber();
    refreshRunning();
    // Poll only while the deck is open AND the window is actually in front —
    // otherwise a hidden window would keep spawning the process check forever.
    setInterval(() => { if (isOpen() && document.hasFocus()) refreshRunning(); }, 5000);

    // Direct hotkeys: any tile with a `hotkey` fires without opening Fay
    // (folders excluded); `closeHotkey` tears a scene down.
    const bindings = [];
    for (const i of allTiles(cfg)) {
      const k = kindOf(i);
      if (i.hotkey && k !== "folder" && !isQuicklink(i) && (k !== "launch" || i.target)) {
        bindings.push({ accelerator: i.hotkey, ...actionOf(i) });
      }
      if (i.closeHotkey && canClose(i)) bindings.push({ accelerator: i.closeHotkey, ...closeActionOf(i) });
    }
    if (bindings.length && invoke) {
      invoke("register_item_hotkeys", { bindings })
        .then((bad) => { if (bad && bad.length) warn(`hotkeys not bound: ${bad.join(", ")}`); })
        .catch((e) => warn(`item hotkeys: ${e}`));
    }

    if (problems.length) {
      warn(`config: ${problems.slice(0, 2).join(" · ")}${problems.length > 2 ? ` (+${problems.length - 2} more)` : ""}`);
      console.warn("config problems:", problems);
    } else if (seeded) {
      setStatus(`config created — edit it via tray › Open config file`, "is-flash", 8000);
    }
    // Start hidden by default (tray + hotkey); show on first run, on a broken
    // config (so the warning is seen), or if configured.
    showNow = seeded || !!broken || problems.length > 0 || app.startHidden === false;
  } catch (e) {
    warn(`config error: ${e.message}`);
    showNow = true; // make the problem visible
  }
  if (showNow && invoke) invoke("show_window").catch(() => {});
  refreshMonitors();
}

main();

// Fay — config-driven hub logic. Rendering of the Heart lives in heart.js.
// The UI is rendered entirely from the config. Never hardcode tiles here.

const els = {
  scenes: document.getElementById("scenes"),
  apps: document.getElementById("apps"),
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

// ---- open / rest state ----------------------------------------------------
function openDeck() { document.body.classList.add("open"); }
function closeDeck() { document.body.classList.remove("open"); setFilter(""); }
function isOpen() { return document.body.classList.contains("open"); }

// ---- type-to-filter + number keys -----------------------------------------
let filterText = "";
const visibleTiles = () => [...document.querySelectorAll(".tile:not(.is-hidden)")];

function setFilter(q) {
  filterText = q;
  els.filter.textContent = q ? `› ${q}` : "";
  els.filter.classList.toggle("is-active", !!q);
  for (const t of document.querySelectorAll(".tile")) {
    const name = (t.dataset.name || "").toLowerCase();
    t.classList.toggle("is-hidden", !!q && !name.includes(q));
  }
  renumber();
}

// Label the first nine visible tiles 1–9 so a digit fires them.
function renumber() {
  document.querySelectorAll(".tile__idx").forEach((s) => (s.textContent = ""));
  visibleTiles().slice(0, 9).forEach((t, i) => {
    const s = t.querySelector(".tile__idx");
    if (s) s.textContent = String(i + 1);
  });
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
const warn = (m) => { console.warn(m); setStatus(m, "is-warn", 7000); };

// ---- launching ------------------------------------------------------------
async function launch(item) {
  if (item.audioOut && invoke) {
    invoke("set_audio_output", { device: item.audioOut }).catch((e) => warn(`audio: ${e}`));
  }
  if (!item.target) return;
  flash(`→ ${item.name}${item.elevated ? " (admin)" : ""}`);
  if (invoke) {
    try {
      await invoke("launch", { target: item.target, elevated: !!item.elevated });
    } catch (e) {
      warn(`${item.name}: ${e}`);
    }
  } else {
    console.log("[preview] would launch:", item.target);
    flash(`(preview) ${item.name}`);
  }
}

function tile(item, kind) {
  const el = document.createElement("button");
  el.className = `tile tile--${kind}`;
  el.dataset.name = item.name || "";
  const badge = item.elevated ? `<span class="tile__badge">ADMIN</span>` : "";
  const key = item.hotkey ? `<span class="tile__key">${escapeHtml(item.hotkey)}</span>` : "";
  el.innerHTML = `
    ${badge}${key}
    <span class="tile__glyph">${escapeHtml(item.glyph || "○")}</span>
    <div>
      <div class="tile__name"><span class="tile__idx"></span>${escapeHtml(item.name)}</div>
      ${item.hint ? `<div class="tile__hint">${escapeHtml(item.hint)}</div>` : ""}
    </div>`;
  el.addEventListener("click", () => launch(item));
  return el;
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
    els.monitors.textContent = `⧉ ${mons.length} display${mons.length === 1 ? "" : "s"} · ${w}×${h}`;
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
      if (isOpen()) { if (filterText) setFilter(""); else closeDeck(); }
      else if (invoke) invoke("hide_window");
      return;
    }
    if (!isOpen()) {
      if (e.key === "Enter") openDeck();
      return;
    }
    // ---- deck is open ----
    if (/^[1-9]$/.test(e.key) && !e.ctrlKey && !e.altKey && !e.metaKey) {
      const t = visibleTiles()[Number(e.key) - 1];
      if (t) t.click();
      e.preventDefault();
      return;
    }
    if (e.key === "Backspace") { setFilter(filterText.slice(0, -1)); e.preventDefault(); return; }
    if (e.key === "Enter") {
      if (filterText) { const t = visibleTiles()[0]; if (t) t.click(); e.preventDefault(); }
      return; // no filter: native Enter on a focused tile
    }
    if (e.key.length === 1 && /[a-z0-9 \-_.]/i.test(e.key) && !e.ctrlKey && !e.altKey && !e.metaKey) {
      setFilter(filterText + e.key.toLowerCase());
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
    if (text) return { cfg: JSON.parse(text), seeded: false };
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

async function main() {
  startClock();
  wireInput();
  let showNow = false;
  try {
    const { cfg, seeded, path } = await loadConfig();
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
    if (typeof app.autostart === "boolean" && invoke) {
      invoke("set_autostart", { enabled: app.autostart }).catch((e) => warn(`autostart: ${e}`));
    }

    (cfg.scenes || []).forEach((s) => els.scenes.appendChild(tile(s, "scene")));
    (cfg.apps || []).forEach((a) => els.apps.appendChild(tile(a, "app")));
    renumber();

    // Direct hotkeys: any tile with a `hotkey` fires without opening Fay.
    const bindings = [...(cfg.scenes || []), ...(cfg.apps || [])]
      .filter((i) => i.hotkey && i.target)
      .map((i) => ({
        accelerator: i.hotkey,
        target: i.target,
        elevated: !!i.elevated,
        audioOut: i.audioOut || null,
      }));
    if (bindings.length && invoke) {
      invoke("register_item_hotkeys", { bindings })
        .then((bad) => { if (bad && bad.length) warn(`hotkeys not bound: ${bad.join(", ")}`); })
        .catch((e) => warn(`item hotkeys: ${e}`));
    }

    if (seeded) setStatus(`config created — edit it via tray › Open config file`, "is-flash", 8000);
    // Start hidden by default (tray + hotkey); show on first run or if configured.
    showNow = seeded || app.startHidden === false;
  } catch (e) {
    warn(`config error: ${e.message}`);
    showNow = true; // make the problem visible
  }
  if (showNow && invoke) invoke("show_window").catch(() => {});
  refreshMonitors();
}

main();

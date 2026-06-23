// Fay — config-driven hub logic. Rendering of the Heart lives in heart.js.
// The UI is rendered entirely from apps.config.json. Never hardcode tiles here.

const els = {
  scenes: document.getElementById("scenes"),
  apps: document.getElementById("apps"),
  brand: document.getElementById("brand"),
  status: document.getElementById("status"),
  monitors: document.getElementById("monitors"),
  clock: document.getElementById("clock"),
  core: document.getElementById("core"),
  canvas: document.getElementById("heart"),
};

const tauri = window.__TAURI__ || null;
const invoke =
  tauri && tauri.core && tauri.core.invoke ? tauri.core.invoke.bind(tauri.core) : null;

// ---- open / rest state ----------------------------------------------------
function openDeck() { document.body.classList.add("open"); }
function closeDeck() { document.body.classList.remove("open"); }
function isOpen() { return document.body.classList.contains("open"); }

// ---- launching ------------------------------------------------------------
async function launch(item) {
  if (item.audioOut && invoke) {
    invoke("set_audio_output", { device: item.audioOut }).catch((e) => flash(`✕ audio: ${e}`));
  }
  if (!item.target) return;
  flash(`→ ${item.name}${item.elevated ? " (admin)" : ""}`);
  if (invoke) {
    try {
      await invoke("launch", { target: item.target, elevated: !!item.elevated });
    } catch (e) {
      flash(`✕ ${item.name}: ${e}`);
    }
  } else {
    console.log("[preview] would launch:", item.target);
    flash(`(preview) ${item.name}`);
  }
}

function flash(msg) {
  els.status.textContent = msg;
  els.status.classList.add("is-flash");
  clearTimeout(flash._t);
  flash._t = setTimeout(() => {
    els.status.textContent = "ready";
    els.status.classList.remove("is-flash");
  }, 1600);
}

function tile(item, kind) {
  const el = document.createElement("button");
  el.className = `tile tile--${kind}`;
  const badge = item.elevated ? `<span class="tile__badge">ADMIN</span>` : "";
  el.innerHTML = `
    ${badge}
    <span class="tile__glyph">${escapeHtml(item.glyph || "○")}</span>
    <div>
      <div class="tile__name">${escapeHtml(item.name)}</div>
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
      if (isOpen()) closeDeck();
      else if (invoke) invoke("hide_window");
      return;
    }
    if (e.key === "Enter" && !isOpen()) { openDeck(); return; }
    if (isOpen() && e.key.startsWith("Arrow")) {
      const tiles = [...document.querySelectorAll(".tile")];
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

async function loadConfig() {
  const res = await fetch("apps.config.json", { cache: "no-store" });
  if (!res.ok) throw new Error(`config ${res.status}`);
  return res.json();
}

async function main() {
  startClock();
  wireInput();
  try {
    const cfg = await loadConfig();
    const app = cfg.app || {};
    if (app.name) els.brand.textContent = app.name.toUpperCase();

    if (typeof app.backdrop === "number") {
      document.documentElement.style.setProperty("--backdrop-alpha", String(app.backdrop));
    }
    window.__fayAccent = app.accent || "#34e6c6";
    applyAccent(window.__fayAccent);

    if (app.hotkey && invoke) {
      invoke("set_summon_hotkey", { accelerator: app.hotkey }).catch((e) => console.error("hotkey:", e));
    }
    if (typeof app.autostart === "boolean" && invoke) {
      invoke("set_autostart", { enabled: app.autostart }).catch((e) => console.error("autostart:", e));
    }

    (cfg.scenes || []).forEach((s) => els.scenes.appendChild(tile(s, "scene")));
    (cfg.apps || []).forEach((a) => els.apps.appendChild(tile(a, "app")));
  } catch (e) {
    flash(`config error: ${e.message}`);
    console.error(e);
  }
  refreshMonitors();
}

main();

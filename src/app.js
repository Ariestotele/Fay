// Fay — config-driven hub.
// The UI is rendered entirely from apps.config.json. Never hardcode tiles here.

const els = {
  scenes: document.getElementById("scenes"),
  apps: document.getElementById("apps"),
  brandName: document.getElementById("brandName"),
  brandTag: document.getElementById("brandTag"),
  status: document.getElementById("status"),
  monitors: document.getElementById("monitors"),
  clock: document.getElementById("clock"),
};

// Are we inside Tauri? (withGlobalTauri exposes window.__TAURI__)
const tauri = window.__TAURI__ || null;
const invoke =
  tauri && tauri.core && tauri.core.invoke ? tauri.core.invoke.bind(tauri.core) : null;

async function launch(item) {
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
    <span class="tile__glyph">${escapeHtml(item.glyph || "▢")}</span>
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
    const d = new Date();
    els.clock.textContent = d
      .toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      .toUpperCase();
  };
  tick();
  setInterval(tick, 15000);
}

// Show the live monitor layout so a scene landing off-screen is explainable.
async function refreshMonitors() {
  if (!invoke) {
    els.monitors.textContent = "⧉ preview";
    return;
  }
  try {
    const mons = await invoke("list_monitors");
    const total = mons.reduce(
      (a, m) => ({ w: a.w + m.width, h: Math.max(a.h, m.height) }),
      { w: 0, h: 0 }
    );
    const label = `⧉ ${mons.length} display${mons.length === 1 ? "" : "s"} · ${total.w}×${total.h}`;
    els.monitors.textContent = label;
    els.monitors.dataset.count = String(mons.length);
  } catch (e) {
    els.monitors.textContent = "⧉ —";
    console.error(e);
  }
}

function wireKeys() {
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && invoke) invoke("hide_window");
  });
}

async function loadConfig() {
  const res = await fetch("apps.config.json", { cache: "no-store" });
  if (!res.ok) throw new Error(`config ${res.status}`);
  return res.json();
}

async function main() {
  startClock();
  wireKeys();
  try {
    const cfg = await loadConfig();
    if (cfg.app?.name) els.brandName.textContent = cfg.app.name;
    if (cfg.app?.tagline) els.brandTag.textContent = cfg.app.tagline;
    document.documentElement.style.setProperty("--cols", cfg.app?.columns || 4);

    (cfg.scenes || []).forEach((s) => els.scenes.appendChild(tile(s, "scene")));
    (cfg.apps || []).forEach((a) => els.apps.appendChild(tile(a, "app")));
  } catch (e) {
    flash(`config error: ${e.message}`);
    console.error(e);
  }
  refreshMonitors();
}

main();

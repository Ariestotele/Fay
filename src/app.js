// Fay — config-driven hub.
// The UI is rendered entirely from apps.config.json. Never hardcode tiles here.

const els = {
  scenes: document.getElementById("scenes"),
  apps: document.getElementById("apps"),
  brandName: document.getElementById("brandName"),
  brandTag: document.getElementById("brandTag"),
  status: document.getElementById("status"),
  clock: document.getElementById("clock"),
};

// Are we inside Tauri? (withGlobalTauri exposes window.__TAURI__)
const tauri = window.__TAURI__ || null;

async function launch(target, label) {
  if (!target) return;
  flash(`→ ${label}`);
  if (tauri && tauri.core && tauri.core.invoke) {
    try {
      await tauri.core.invoke("launch", { target });
    } catch (e) {
      flash(`✕ ${label}: ${e}`);
    }
  } else {
    // Browser preview: no OS access. Just log.
    console.log("[preview] would launch:", target);
    flash(`(preview) ${label}`);
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
  el.innerHTML = `
    <span class="tile__glyph">${escapeHtml(item.glyph || "▢")}</span>
    <div>
      <div class="tile__name">${escapeHtml(item.name)}</div>
      ${item.hint ? `<div class="tile__hint">${escapeHtml(item.hint)}</div>` : ""}
    </div>`;
  el.addEventListener("click", () => launch(item.target, item.name));
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

async function loadConfig() {
  const res = await fetch("apps.config.json", { cache: "no-store" });
  if (!res.ok) throw new Error(`config ${res.status}`);
  return res.json();
}

async function main() {
  startClock();
  try {
    const cfg = await loadConfig();
    if (cfg.app?.name) els.brandName.textContent = cfg.app.name;
    if (cfg.app?.tagline) els.brandTag.textContent = cfg.app.tagline;
    const cols = cfg.app?.columns || 4;
    document.documentElement.style.setProperty("--cols", cols);

    (cfg.scenes || []).forEach((s) => els.scenes.appendChild(tile(s, "scene")));
    (cfg.apps || []).forEach((a) => els.apps.appendChild(tile(a, "app")));
  } catch (e) {
    flash(`config error: ${e.message}`);
    console.error(e);
  }
}

main();

// Frontend → backend contract tests. A fake window.__TAURI__ records every
// invoke() so we can assert exactly what the UI sends to Rust for scenes,
// teardown, multi-actions, snippets, hotkey bindings, clipboard, file search
// and AI plans — the seams the preview-mode flow tests can't reach.
// Needs playwright-core + a Chromium binary (CHROME). Run: node docs/test-backend-contract.js
const http = require("http"), fs = require("fs"), path = require("path");
const { chromium } = require("playwright-core");
const SRC = path.join(__dirname, "..", "src");
const MIME = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json" };
const server = http.createServer((req, res) => {
  const p = path.join(SRC, req.url === "/" ? "index.html" : decodeURIComponent(req.url.split("?")[0]));
  fs.readFile(p, (err, d) => { if (err) { res.writeHead(404); res.end(); return; } res.writeHead(200, { "content-type": MIME[path.extname(p)] || "text/plain" }); res.end(d); });
});
let fails = 0;
const ok = (name, cond, extra = "") => { console.log(`${cond ? "PASS" : "FAIL"}  ${name}${extra ? "  — " + extra : ""}`); if (!cond) fails++; };

// The fake bridge: canned answers per command, everything recorded.
const MOCK = `
window.__calls = [];
window.__aiReply = JSON.stringify({ reply: "Game mode, speakers stay.", actions: [{ tile: "game", audioOut: "" }] });
const answers = {
  load_config: () => null,                       // first run → seeds from bundled
  save_config: () => "C:\\\\Users\\\\me\\\\AppData\\\\Roaming\\\\com.fay.hub\\\\apps.config.json",
  get_hostname: () => "TESTPC",
  list_commands: () => [],
  list_running: () => ["discord", "steam"],
  get_accent_color: () => "#3388ff",
  list_monitors: () => [{ name: "a", width: 2560, height: 1440, x: 0, y: 0, scale: 1 }, { name: "b", width: 1920, height: 1080, x: 2560, y: 0, scale: 1 }],
  get_stats: () => ({ cpu: 10, ramUsed: 1, ramTotal: 2, gpu: null, gpuTemp: null, down: 0, up: 0 }),
  register_item_hotkeys: () => [],
  ai_config: () => "anthropic · claude-sonnet-5",
  ai_ask: () => window.__aiReply,
  clipboard_history: () => ["hello world", "second entry"],
  search_files: (a) => a.query ? [{ name: "report.pdf", path: "D:\\\\docs\\\\report.pdf", dir: false }] : [],
  list_bookmarks: () => [{ title: "Tauri docs", url: "https://tauri.app", browser: "Zen" }],
  fire: () => null,
  doctor: (a) => [
    { name: "machine", ok: null, detail: "TESTPC · windows" },
    ...a.targets.map((t) => ({ name: "tile " + t.name, ok: t.name !== "Zen", detail: t.name === "Zen" ? "not found: C:\\\\zen.exe" : "ok" })),
    { name: "tool es.exe", ok: false, detail: "es.exe not on PATH" },
    { name: "AI", ok: true, detail: "anthropic · key set" },
  ],
  set_clipboard: () => null,
};
window.__TAURI__ = { core: { invoke: async (cmd, args) => { window.__calls.push({ cmd, args: args || {} }); const f = answers[cmd]; return f ? f(args || {}) : null; } } };
`;

(async () => {
  await new Promise((r) => server.listen(0, r));
  const browser = await chromium.launch({ executablePath: process.env.CHROME || "/opt/pw-browsers/chromium-1194/chrome-linux/chrome", args: ["--no-sandbox"] });
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript(MOCK);
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await page.waitForTimeout(800);

  const calls = () => page.evaluate(() => window.__calls);
  const last = async (cmd) => { const c = (await calls()).filter((x) => x.cmd === cmd); return c.length ? c[c.length - 1].args : null; };
  const count = async (cmd) => (await calls()).filter((x) => x.cmd === cmd).length;
  const status = () => page.$eval("#status", (e) => e.textContent);
  const reset = () => page.evaluate(() => { window.__calls = []; });
  const inBody = (cls) => page.$eval("body", (b, c) => b.classList.contains(c), cls);
  const shown = (sel) => page.$eval(sel, (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && document.body.getBoundingClientRect().height > 0; });

  // ---- startup contract ----
  ok("no page errors", errors.length === 0, errors.join(" | "));
  ok("first run seeds the user config", (await count("save_config")) === 1);
  ok("summon hotkey sent from config", (await last("set_summon_hotkey"))?.accelerator === "Ctrl+Alt+Space");
  ok("mouse summon sent", (await last("set_mouse_summon"))?.spec === "Ctrl+Mouse5");
  ok("summon monitor mode sent", (await last("set_summon_monitor"))?.mode === "cursor");
  ok("clipboard watcher enabled with max", JSON.stringify(await last("set_clipboard_watch")) === JSON.stringify({ enabled: true, max: 50 }));
  ok("voice configured", (await last("voice_config"))?.enabled === true);
  const grammar = (await last("set_voice_grammar"))?.phrases || [];
  ok("voice grammar has tile names + verbs", grammar.includes("game") && grammar.includes("open discord") && grammar.includes("close game"), `${grammar.length} phrases`);
  ok("voice grammar excludes shutdown/restart", !grammar.some((p) => /shut down|restart/.test(p)));
  ok("ai configured from app.ai", (await last("ai_config"))?.provider === "anthropic");
  ok("accent applied from backend", (await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent").trim())) === "#3388ff");
  ok("hostname shown in footer", (await page.$eval("#monitors", (e) => e.textContent)).includes("TESTPC"));
  ok("2 displays reported", (await page.$eval("#monitors", (e) => e.textContent)).includes("2 displays"));
  ok("running dot on Discord + Steam", (await page.$$eval(".tile.is-running", (els) => els.map((e) => e.dataset.name).sort().join())) === "Discord,Steam");

  const bindings = (await last("register_item_hotkeys"))?.bindings || [];
  const byKey = Object.fromEntries(bindings.map((b) => [b.accelerator, b]));
  ok("hotkey bindings registered", bindings.length >= 6, `${bindings.length}`);
  ok("scene binding carries target + audioOut + say", byKey["Ctrl+Alt+2"]?.kind === "launch" && byKey["Ctrl+Alt+2"].audioOut?.includes("HyperX") && !!byKey["Ctrl+Alt+2"].say);
  ok("focus binding carries minutes", byKey["Ctrl+Alt+F"]?.kind === "focus" && byKey["Ctrl+Alt+F"].minutes === 25);
  ok("clipboard / listen / ask bindings by kind", byKey["Ctrl+Alt+V"]?.kind === "clipboard" && byKey["Ctrl+Alt+L"]?.kind === "listen" && byKey["Ctrl+Alt+A"]?.kind === "ask");

  // ---- firing ----
  await page.keyboard.press("Enter"); await page.waitForTimeout(200);
  await reset();
  await page.keyboard.press("2"); await page.waitForTimeout(150);
  let f = await last("fire");
  ok("digit 2 fires Game as a launch with audioOut", f?.action.kind === "launch" && f.action.target.endsWith("Fay-Game.lnk") && f.action.audioOut.includes("HyperX"));
  ok("payload includes say + no steps", f?.action.say.startsWith("Game mode") && f.action.steps.length === 0);

  await reset();
  await page.keyboard.press("Control+Digit2"); await page.waitForTimeout(150);
  f = await last("fire");
  ok("Ctrl+2 sends close with the scene's process list", f?.action.kind === "close" && f.action.closes.join() === "Discord,zen,Taskmgr");

  await reset();
  await page.click(".tile[data-name='Wind down']"); await page.waitForTimeout(150);
  f = await last("fire");
  ok("multi-action sends ordered steps", f?.action.kind === "multi" && f.action.steps.map((s) => s.kind).join() === "media,wait,system" && f.action.steps[1].wait === 400);

  await reset();
  await page.click(".tile--k-folder"); await page.waitForTimeout(150);
  await page.click(".tile[data-name='Sign-off']"); await page.waitForTimeout(200);
  const seq = (await calls()).map((c) => c.cmd).filter((c) => c === "hide_window" || c === "fire").join(",");
  f = await last("fire");
  ok("snippet hides Fay first, then fires with paste", seq === "hide_window,fire" && f?.action.kind === "snippet" && f.action.paste === true && f.action.text.includes("Best regards"), seq);
  await page.keyboard.press("Backspace");

  // quicklink query is URL-encoded into the target
  await reset();
  await page.keyboard.type("yt lofi girl"); await page.keyboard.press("Enter"); await page.waitForTimeout(150);
  f = await last("fire");
  ok("quicklink target has encoded query", f?.action.target === "https://www.youtube.com/results?search_query=lofi%20girl", f?.action.target);
  await page.keyboard.press("Escape");

  // ---- clipboard history ----
  await reset();
  await page.click(".tile[data-name='Clipboard']"); await page.waitForTimeout(300);
  ok("clipboard rows rendered", (await page.$$eval(".row", (els) => els.length)) === 2);
  await page.keyboard.press("ArrowDown"); await page.keyboard.press("Enter"); await page.waitForTimeout(200);
  const cp = await last("clipboard_pick");
  const order = (await calls()).map((c) => c.cmd).filter((c) => c === "hide_window" || c === "clipboard_pick").join(",");
  ok("Enter on 2nd row hides then pastes that entry", order === "hide_window,clipboard_pick" && cp?.text === "second entry" && cp.paste === true, order);
  await page.keyboard.press("Escape"); // leaves clipboard mode; deck stays open
  ok("deck still open after leaving clipboard mode", await inBody("open"));

  // ---- file search ----
  await reset();
  await page.keyboard.type("> report"); await page.waitForTimeout(500);
  ok("search_files called with the query", (await last("search_files"))?.query === "report");
  ok("file row rendered", (await page.$eval(".row__text", (e) => e.textContent)) === "report.pdf");
  await page.keyboard.press("Shift+Enter"); await page.waitForTimeout(150);
  ok("Shift+Enter reveals in Explorer", (await last("reveal"))?.path === "D:\\docs\\report.pdf");
  await page.keyboard.press("Enter"); await page.waitForTimeout(150);
  ok("Enter opens the file via launch", (await last("fire"))?.action.target === "D:\\docs\\report.pdf");
  await page.keyboard.press("Escape");

  // ---- bookmarks ----
  await page.keyboard.type("@ tauri"); await page.waitForTimeout(300);
  ok("bookmark row rendered", (await page.$eval(".row__text", (e) => e.textContent)) === "Tauri docs");
  await page.keyboard.press("Escape");

  // ---- AI plan → actions ----
  await reset();
  await page.keyboard.type("game mode but keep speakers"); await page.keyboard.press("Enter");
  await page.waitForTimeout(500);
  const ask = await last("ai_ask");
  ok("ai_ask gets the system prompt with the deck + user message", ask?.system.includes('"id":"game"') && ask.messages[0].role === "user" && ask.messages[0].content === "game mode but keep speakers");
  f = await last("fire");
  ok("plan runs as one multi with the Game tile, audio override cleared", f?.action.kind === "multi" && f.action.steps[0].target.endsWith("Fay-Game.lnk") && f.action.steps[0].audioOut === null);
  ok("answer panel is painted on screen", await shown(".answer__text"));
  ok("reply shown in the answer panel", (await page.$eval(".answer__text", (e) => e.textContent)).includes("speakers stay"));
  ok("spoken via say", (await last("say"))?.text.includes("speakers stay"));

  // destructive gate: model asks to shut down, user never said so
  await page.evaluate(() => { window.__aiReply = JSON.stringify({ reply: "Shutting down.", actions: [{ kind: "system", action: "shutdown" }] }); });
  await reset();
  await page.keyboard.type("? do the thing"); await page.keyboard.press("Enter"); await page.waitForTimeout(500);
  ok("unconfirmed shutdown is NOT sent to fire", (await count("fire")) === 0);
  ok("answer notes the skipped action", (await page.$eval(".answer__meta", (e) => e.textContent)).includes("skipped"));
  // confirmed by the user's own words → allowed
  await reset();
  await page.keyboard.type("? yes, shut down now"); await page.keyboard.press("Enter"); await page.waitForTimeout(500);
  f = await last("fire");
  ok("confirmed shutdown is sent", f?.action.steps?.[0]?.kind === "system" && f.action.steps[0].action === "shutdown");
  await page.keyboard.press("Escape"); await page.keyboard.press("Escape");

  // ---- doctor: targets sent, rows rendered, Enter copies the report ----
  await reset();
  if (!(await inBody("open"))) { await page.keyboard.press("Enter"); await page.waitForTimeout(100); }
  await page.click(".tile--k-doctor"); await page.waitForTimeout(300);
  const d = await last("doctor");
  const names = (d?.targets || []).map((t) => t.name);
  ok("doctor sends launch targets only (scenes + apps, no quicklinks / multi)", names.includes("Zen") && names.includes("Game") && !names.includes("YouTube") && !names.includes("Wind down"), names.join());
  ok("doctor passes the Everything path from config", d && "es" in d);
  const report = await page.$eval(".answer__text", (e) => e.textContent);
  ok("report panel is painted on screen", await shown(".answer__text"));
  ok("report lists backend rows with marks", report.includes("✗ tile Zen: not found") && report.includes("· machine: TESTPC") && report.includes("✓ AI:"), report.split("\n").slice(0, 3).join(" / "));
  ok("report counts problems in the meta line", (await page.$eval(".answer__meta", (e) => e.textContent)).includes("2 problems"));
  await page.keyboard.press("Enter"); await page.waitForTimeout(100);
  ok("Enter copies the report via set_clipboard", ((await last("set_clipboard"))?.text || "").includes("✗ tile Zen"));
  await page.keyboard.press("Escape"); await page.waitForTimeout(50);
  ok("Esc closes the report", !(await inBody("in-results")) && (await inBody("open")));

  // ---- focus hotkey entry point used by the backend ----
  await page.evaluate(() => window.__fayFocus(5, "start"));
  ok("__fayFocus starts a 5-minute timer", (await page.$eval("#timer", (e) => e.textContent)).includes("5 5:00") || (await page.$eval("#timer", (e) => e.textContent)).includes("4:59"));
  await page.evaluate(() => window.__fayFocus(0, "stop"));

  ok("no page errors during flows", errors.length === 0, errors.join(" | "));
  await browser.close(); server.close();
  console.log(fails ? `\n${fails} FAILED` : "\nALL PASS");
  process.exit(fails ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(2); });

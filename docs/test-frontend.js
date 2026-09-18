// Drive the real frontend (preview mode, no Tauri) through its keyboard flows
// in headless Chromium. Needs playwright-core and a Chromium/Chrome binary
// (CHROME env var). Run: node docs/test-frontend.js — exits 1 on any FAIL.
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

(async () => {
  await new Promise((r) => server.listen(0, r));
  const browser = await chromium.launch({ executablePath: process.env.CHROME || "/opt/pw-browsers/chromium-1194/chrome-linux/chrome", args: ["--no-sandbox"] });
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error" && !/404/.test(m.text())) errors.push(m.text()); });
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await page.waitForTimeout(600);

  const status = () => page.$eval("#status", (e) => e.textContent);
  const filter = () => page.$eval("#filter", (e) => e.textContent);
  const visible = () => page.evaluate(() => [...(document.body.classList.contains("in-folder") ? document.getElementById("folder") : document.getElementById("groups")).querySelectorAll(".tile:not(.is-hidden)")].map((e) => e.dataset.name));
  const inBody = (cls) => page.$eval("body", (b, c) => b.classList.contains(c), cls);
  const numbered = () => page.$$eval(".tile__idx", (els) => els.map((e) => e.textContent).filter(Boolean));

  ok("no page errors on load", errors.length === 0, errors.join(" | "));
  ok("status is ready (no config warning)", (await status()) === "ready", await status());
  const all = await page.$$eval(".tile", (els) => els.length);
  ok("tiles rendered (scenes+apps+add+system+packs)", all >= 20, `${all} tiles`);

  // open deck via Enter, numbering
  await page.keyboard.press("Enter");
  await page.waitForTimeout(300);
  ok("Enter opens deck", await inBody("open"));
  ok("first nine tiles numbered", (await numbered()).join("") === "123456789", (await numbered()).join(""));

  // type-to-filter
  await page.keyboard.type("zen");
  ok("filter narrows to Zen", (await visible()).join() === "Zen", (await visible()).join());
  ok("filter line shows text", (await filter()).includes("zen"));
  await page.keyboard.press("Escape");
  ok("Esc clears filter, deck stays open", (await visible()).length === all && (await inBody("open")));

  // calculator + conversion
  await page.keyboard.type("= 1440*0.62");
  ok("calculator answer", (await filter()).includes("= 892.8"), await filter());
  await page.keyboard.press("Escape");
  await page.keyboard.type("= 5 mi to km");
  ok("unit conversion", (await filter()).includes("8.04672 km"), await filter());
  await page.keyboard.press("Enter"); await page.waitForTimeout(200);
  ok("Enter copies (preview clipboard)", (await status()).startsWith("copied"), await status());
  await page.keyboard.press("Escape");

  // digits don't type while filter empty; "=" makes them type
  await page.keyboard.type("=12");
  ok("digits after = go to filter", (await filter()).includes("=12"), await filter());
  await page.keyboard.press("Escape");

  // quicklink prompt
  await page.keyboard.type("yt lofi girl");
  ok("quicklink mode shows keyword", (await filter()).includes("YouTube") && (await filter()).includes("lofi girl"), await filter());
  ok("only the YouTube tile visible", (await visible()).join() === "YouTube", (await visible()).join());
  await page.keyboard.press("Enter"); await page.waitForTimeout(200);
  ok("quicklink fires (preview)", (await status()).includes("YouTube"), await status());
  await page.keyboard.press("Escape");

  // folder
  await page.click(".tile--k-folder");
  await page.waitForTimeout(200);
  ok("folder opens", await inBody("in-folder"));
  ok("folder shows children", (await visible()).join() === "Sign-off,Google,GitHub", (await visible()).join());
  ok("children renumbered from 1", (await numbered()).join("") === "123", (await numbered()).join(""));
  await page.keyboard.type("g any");
  ok("child quicklink from inside folder", (await filter()).includes("Google"), await filter());
  await page.keyboard.press("Escape");
  await page.keyboard.press("Backspace");
  ok("Backspace leaves folder", !(await inBody("in-folder")));

  // system action confirm
  const restart = await page.$(".tile[data-name='Restart']");
  await restart.click(); await page.waitForTimeout(100);
  ok("destructive tile arms on first click", await restart.evaluate((e) => e.classList.contains("is-confirm")));
  ok("status asks to press again", (await status()).includes("again"), await status());
  await page.keyboard.press("Escape"); // closes the deck (nothing typed)
  ok("Esc with nothing typed closes deck", !(await inBody("open")));
  await page.keyboard.press("Enter"); await page.waitForTimeout(200);

  // results modes (preview: providers return empty)
  await page.keyboard.type("> report");
  await page.waitForTimeout(400);
  ok("files mode shows results panel", await inBody("results"));
  await page.keyboard.press("Escape");
  ok("Esc leaves results mode", !(await inBody("results")));
  await page.keyboard.type("@ tauri");
  await page.waitForTimeout(200);
  ok("bookmarks mode ('@' must not be eaten as Shift+2)", (await filter()).includes("bookmarks"), await filter());
  await page.keyboard.press("Escape");

  // ask mode (preview: ai enabled w/o key → asks are logged, not sent)
  await page.keyboard.type("? what is this");
  ok("ask mode prompt line", (await filter()).includes("ask Fay"), await filter());
  await page.keyboard.press("Escape");
  await page.keyboard.type("game mode but keep audio");
  const hint = await filter();
  ok("no-match line offers Enter → ask", hint.includes("Enter asks Fay"), hint);
  await page.keyboard.press("Escape");

  // focus timer via tile
  await page.click(".tile--k-focus");
  await page.waitForTimeout(200);
  const timer = await page.$eval("#timer", (e) => e.textContent);
  ok("focus timer starts and shows countdown", /Focus 25 2[45]:\d\d/.test(timer), timer);
  ok("focus tile marked active", await page.$eval(".tile--k-focus", (e) => e.classList.contains("is-active")));
  await page.click(".tile--k-focus");
  ok("second click stops timer", (await page.$eval("#timer", (e) => e.textContent)) === "");

  // Shift+digit teardown on a scene with closes
  await page.keyboard.press("Control+Digit2"); await page.waitForTimeout(150);
  ok("Ctrl+2 tears down Game (preview)", (await status()).includes("Game"), await status());

  // close deck via Esc
  await page.keyboard.press("Escape");
  ok("Esc closes deck", !(await inBody("open")));

  ok("no page errors during flows", errors.length === 0, errors.join(" | "));
  await browser.close(); server.close();
  console.log(fails ? `\n${fails} FAILED` : "\nALL PASS");
  process.exit(fails ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(2); });

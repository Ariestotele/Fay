// Screenshot the real frontend (src/) in headless Chromium: the Heart at rest
// (with live stats + a focus arc) and with the deck open. Needs playwright-core
// (npm i playwright-core) and a Chromium binary (CHROME env var or the default).
// Run: node docs/render-preview.js
const fs = require("fs");
const path = require("path");
const { chromium } = require("playwright-core");

const SRC = path.join(__dirname, "..", "src");
const OUT = __dirname; // writes ui-preview.png / ui-deck.png next to this script
const MIME = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".png": "image/png" };

const server = http.createServer((req, res) => {
  const p = path.join(SRC, req.url === "/" ? "index.html" : decodeURIComponent(req.url.split("?")[0]));
  fs.readFile(p, (err, data) => {
    if (err) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { "content-type": MIME[path.extname(p)] || "application/octet-stream" });
    res.end(data);
  });
});

(async () => {
  await new Promise((r) => server.listen(0, r));
  const url = `http://127.0.0.1:${server.address().port}/`;
  const browser = await chromium.launch({
    executablePath: process.env.CHROME || "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
    args: ["--no-sandbox", "--use-gl=swiftshader", "--enable-unsafe-swiftshader"],
  });
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 }, deviceScaleFactor: 1 });
  await page.goto(url);
  await page.waitForTimeout(800);
  // A wallpaper-ish gradient behind the translucent backdrop so the dimming reads.
  await page.addStyleTag({ content: `html { background: radial-gradient(ellipse at 30% 20%, #1c2a4a 0%, #0b1020 45%, #05070d 100%) !important; }` });
  // Live stats + a running focus timer, as the app would set them.
  await page.evaluate(() => {
    window.Heart.setStats({ cpu: 23, ramUsed: 13.4 * 1073741824, ramTotal: 32 * 1073741824, gpu: 41, gpuTemp: 58, down: 2.3e6, up: 180e3 });
    window.Heart.setProgress(0.38);
    document.getElementById("timer").textContent = "◔ Focus 25 15:31";
    window.Heart.start();
  });
  await page.waitForTimeout(2500);
  await page.screenshot({ path: path.join(OUT, "ui-preview.png") });

  // Open the deck.
  await page.click("#core");
  await page.waitForTimeout(900);
  await page.screenshot({ path: path.join(OUT, "ui-deck.png") });

  // Type a calculation to show the filter line.
  await page.keyboard.type("= 5 mi to km");
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, "ui-calc.png") });

  await browser.close();
  server.close();
  console.log("done");
})().catch((e) => { console.error(e); process.exit(1); });

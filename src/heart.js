// Fay — the Heart: a canvas particle field (layered rings + inner set + a
// pulsing particle sphere). Pure particles, additive glow. Pauses while hidden.

(function () {
  const canvas = document.getElementById("heart");
  const ctx = canvas.getContext("2d");
  let W = 0, H = 0, cx = 0, cy = 0, R = 0;
  let accent = { r: 52, g: 230, b: 198 };
  let light = { r: 194, g: 255, b: 240 };
  let rings = [], sphere = [];
  let running = false, last = 0, t = 0;
  let stats = null;      // live readouts (CPU / RAM / GPU / NET) drawn at rest
  let progress = null;   // 0..1 focus-timer arc, or null
  let pulseT = 0;        // 1 → 0 burst (spoken reply, timer done, …)
  let showStats = true;
  let talkUntil = 0, listenUntil = 0; // voice: speaking wobble / listening breath

  function hexToRgb(h) {
    h = String(h || "").trim().replace("#", "");
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    if (!/^[0-9a-f]{6}$/i.test(h)) return { r: 52, g: 230, b: 198 }; // strict: "auto" must not parse as 0x0a
    const n = parseInt(h, 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
  }
  function lighten(c, f) {
    return {
      r: Math.round(c.r + (255 - c.r) * f),
      g: Math.round(c.g + (255 - c.g) * f),
      b: Math.round(c.b + (255 - c.b) * f),
    };
  }
  const col = (c, a) => `rgba(${c.r},${c.g},${c.b},${a})`;

  function build() {
    rings = []; sphere = [];
    const ring = (rFrac, thFrac, n, amp, k, speed, bright) => {
      for (let i = 0; i < n; i++) {
        const off = (Math.random() + Math.random() + Math.random() - 1.5) / 1.5;
        rings.push({
          a: Math.random() * Math.PI * 2, off, rFrac, thFrac, amp, k, speed, bright,
          sz: 0.7 + Math.random() * 1.5, b: Math.random(), lite: Math.random() < 0.16,
        });
      }
    };
    // Radii are fractions of R (= 34% of the shorter screen side), matching the
    // approved mockup: main band at ~R, inner set at ~0.7 R, sphere at 0.17 R.
    // main band — 5 stacked layers, different speeds/directions
    ring(0.94, 0.07, 360, 0.035, 7, 0.05, 0.9);
    ring(0.98, 0.08, 420, 0.045, 9, 0.085, 1.0);
    ring(1.02, 0.065, 360, 0.030, 12, -0.06, 0.7);
    ring(0.96, 0.06, 320, 0.040, 6, 0.105, 0.8);
    ring(1.00, 0.055, 320, 0.050, 10, -0.09, 0.65);
    // inner set — offset inward, lighter density
    ring(0.70, 0.045, 300, 0.05, 8, 0.12, 0.6);
    ring(0.76, 0.045, 300, 0.04, 11, -0.10, 0.55);
    ring(0.66, 0.04, 230, 0.06, 6, 0.14, 0.5);
    // particle sphere ball (volumetric)
    for (let i = 0; i < 1500; i++) {
      sphere.push({
        th: Math.acos(2 * Math.random() - 1),
        ph: Math.random() * Math.PI * 2,
        rr: Math.pow(Math.random(), 1 / 3),
        sz: 0.4 + Math.random() * 1.1, b: Math.random(),
        lite: Math.random() < 0.3, spin: 0.12 + Math.random() * 0.06,
      });
    }
  }

  function resize() {
    // Render at device resolution so particles stay crisp on HiDPI displays.
    const dpr = window.devicePixelRatio || 1;
    W = window.innerWidth; H = window.innerHeight;
    canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
    canvas.style.width = W + "px"; canvas.style.height = H + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    cx = W / 2; cy = H / 2; R = Math.min(W, H) * 0.34;
  }

  // heartbeat (double-thump) 0..~1
  function beat(time) {
    const x = (time % 1.15) / 1.15;
    const p1 = Math.exp(-Math.pow((x - 0.10) / 0.045, 2));
    const p2 = 0.55 * Math.exp(-Math.pow((x - 0.26) / 0.05, 2));
    return Math.min(1, p1 + p2);
  }

  // ---- extras: live stats, focus arc ----------------------------------------
  const gb = (b) => (b / 1073741824).toFixed(b >= 10 * 1073741824 ? 0 : 1);
  const rate = (bps) => (bps >= 1048576 ? `${(bps / 1048576).toFixed(1)}M` : bps >= 1024 ? `${Math.round(bps / 1024)}K` : `${Math.round(bps)}`);

  // Four readouts at the corners around the rings: a label line and a dotted
  // gauge (dots, not lines — same language as the rest of the Heart).
  function drawStats() {
    const items = [
      ["CPU", `${Math.round(stats.cpu)}%`, stats.cpu / 100],
      ["RAM", `${gb(stats.ramUsed)}/${gb(stats.ramTotal)}G`, stats.ramTotal ? stats.ramUsed / stats.ramTotal : 0],
      stats.gpu != null
        ? ["GPU", `${Math.round(stats.gpu)}%${stats.gpuTemp != null ? ` ${Math.round(stats.gpuTemp)}°` : ""}`, stats.gpu / 100]
        : null,
      ["NET", `↓${rate(stats.down)} ↑${rate(stats.up)}`, Math.min(1, (stats.down + stats.up) / 12.5e6)],
    ].filter(Boolean);
    const rad = R * 1.22;
    const angles = [-135, -45, 135, 45].map((d) => (d * Math.PI) / 180);
    ctx.font = "11px 'JetBrains Mono', 'Cascadia Code', Consolas, monospace";
    ctx.textBaseline = "middle";
    items.forEach((it, i) => {
      const a = angles[i];
      const x = cx + rad * Math.cos(a), y = cy + rad * Math.sin(a);
      const left = Math.cos(a) < 0;
      // "CPU 23%" reads left→right on both sides; the block hangs away from the rings.
      const label = it[0] + " ", lw = ctx.measureText(label).width, vw = ctx.measureText(it[1]).width;
      const x0 = left ? x - lw - vw : x;
      ctx.textAlign = "left";
      ctx.fillStyle = col(accent, 0.6);
      ctx.fillText(label, x0, y - 7);
      ctx.fillStyle = col(light, 0.92);
      ctx.fillText(it[1], x0 + lw, y - 7);
      const n = 16, filled = Math.round(Math.max(0, Math.min(1, it[2])) * n);
      for (let k = 0; k < n; k++) {
        const dx = (k * 5 + 2) * (left ? -1 : 1);
        ctx.fillStyle = col(accent, k < filled ? 0.85 : 0.16);
        ctx.beginPath(); ctx.arc(x + dx, y + 8, 1.3, 0, 6.283); ctx.fill();
      }
    });
  }

  // Focus timer: a dotted arc just outside the main ring fills clockwise.
  function drawProgress() {
    const n = 220, rr = R * 1.12;
    for (let k = 0; k < n; k++) {
      const f = k / n;
      const a = -Math.PI / 2 + f * Math.PI * 2;
      const on = f < progress;
      ctx.fillStyle = col(on ? light : accent, on ? 0.9 : 0.12);
      ctx.beginPath(); ctx.arc(cx + rr * Math.cos(a), cy + rr * Math.sin(a), on ? 1.5 : 1.0, 0, 6.283); ctx.fill();
    }
  }

  function draw(dt) {
    ctx.clearRect(0, 0, W, H);
    ctx.globalCompositeOperation = "lighter";
    t += dt;
    pulseT = Math.max(0, pulseT - dt * 0.9);
    if (t < talkUntil) pulseT = Math.max(pulseT, 0.25 + 0.35 * Math.abs(Math.sin(t * 11)));   // speaking
    if (t < listenUntil) pulseT = Math.max(pulseT, 0.3 + 0.25 * Math.sin(t * 3));             // listening
    const bt = Math.min(1, beat(t) + pulseT);

    // soft halo under the bands — gives the rings volume without any solid line
    const halo = (rr, w, a) => {
      const g = ctx.createRadialGradient(cx, cy, Math.max(0, rr - w), cx, cy, rr + w);
      g.addColorStop(0, col(accent, 0)); g.addColorStop(0.5, col(accent, a)); g.addColorStop(1, col(accent, 0));
      ctx.fillStyle = g; ctx.beginPath(); ctx.arc(cx, cy, rr + w, 0, 6.283); ctx.fill();
    };
    halo(R * 0.98, R * 0.16, 0.07 + 0.03 * bt);
    halo(R * 0.71, R * 0.10, 0.04);

    // rings
    for (const p of rings) {
      p.a += p.speed * dt;
      const baseR = R * p.rFrac;
      const r = baseR * (1 + p.amp * Math.sin(p.k * p.a)) + p.off * p.thFrac * R;
      const x = cx + r * Math.cos(p.a), y = cy + r * Math.sin(p.a);
      const close = 1 - Math.min(1, Math.abs(p.off));
      const a = (0.04 + 0.8 * Math.pow(close, 1.6)) * p.bright * (0.45 + 0.55 * p.b) * (1 + 0.5 * pulseT);
      ctx.fillStyle = col(p.lite ? light : accent, Math.min(1, a));
      ctx.beginPath(); ctx.arc(x, y, p.sz, 0, 6.283); ctx.fill();
    }

    if (progress != null) drawProgress();
    if (stats && showStats) drawStats();

    // soft glow behind the ball, plus a tight bloom at its core
    const Rs = R * 0.17 * (1 + 0.10 * bt + 0.25 * pulseT);
    ctx.fillStyle = col(accent, 0.05 + 0.04 * bt + 0.12 * pulseT);
    ctx.beginPath(); ctx.arc(cx, cy, Rs * 2.4, 0, 6.283); ctx.fill();
    const core = ctx.createRadialGradient(cx, cy, 0, cx, cy, Rs * 0.55);
    core.addColorStop(0, col(light, 0.22 + 0.18 * bt));
    core.addColorStop(1, col(accent, 0));
    ctx.fillStyle = core; ctx.beginPath(); ctx.arc(cx, cy, Rs * 0.55, 0, 6.283); ctx.fill();

    // particle sphere
    for (const s of sphere) {
      s.ph += s.spin * dt;
      const rad = Rs * s.rr;
      const sinth = Math.sin(s.th);
      const x3 = rad * sinth * Math.cos(s.ph);
      const y3 = rad * sinth * Math.sin(s.ph);
      const z3 = rad * Math.cos(s.th);
      const front = (z3 / Rs + 1) / 2;
      const a = Math.min(0.95, (0.10 + 0.8 * front) * (0.4 + 0.6 * s.b) * (0.8 + 0.4 * bt) * (1.25 - 0.45 * s.rr));
      ctx.fillStyle = col(s.lite ? light : accent, a);
      ctx.beginPath(); ctx.arc(cx + x3, cy + y3, s.sz * (0.5 + front), 0, 6.283); ctx.fill();
    }
    ctx.globalCompositeOperation = "source-over";
  }

  function frame(now) {
    if (!running) return;
    const dt = Math.min(0.05, (now - last) / 1000 || 0);
    last = now;
    draw(dt);
    requestAnimationFrame(frame);
  }
  function start() { if (!running) { running = true; last = performance.now(); requestAnimationFrame(frame); } }
  function stop() { running = false; }

  window.addEventListener("resize", resize);
  window.addEventListener("blur", stop);
  window.addEventListener("focus", start);
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));

  window.Heart = {
    setAccent(hex) { accent = hexToRgb(hex); light = lighten(accent, 0.6); },
    setStats(s) { stats = s || null; },
    setStatsVisible(v) { showStats = !!v; },
    setProgress(f) { progress = f == null ? null : Math.max(0, Math.min(1, f)); },
    pulse() { pulseT = 1; },
    talk(ms) { talkUntil = t + Math.max(0, ms) / 1000; },
    listen(ms) { listenUntil = t + Math.max(0, ms) / 1000; },
    quiet() { talkUntil = 0; listenUntil = 0; },
    start, stop,
  };

  // The window starts hidden: only animate once it's actually in front.
  // (focus/visibility listeners above resume it on summon.)
  resize(); build();
  if (document.hasFocus() && !document.hidden) start();
})();

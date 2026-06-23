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

  function hexToRgb(h) {
    h = (h || "").replace("#", "");
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    const n = parseInt(h, 16);
    if (isNaN(n)) return { r: 52, g: 230, b: 198 };
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
          sz: 0.4 + Math.random() * 1.2, b: Math.random(), lite: Math.random() < 0.16,
        });
      }
    };
    // main band — 5 stacked layers, different speeds/directions
    ring(0.235, 0.024, 360, 0.035, 7, 0.05, 0.9);
    ring(0.245, 0.026, 420, 0.045, 9, 0.085, 1.0);
    ring(0.255, 0.022, 360, 0.030, 12, -0.06, 0.7);
    ring(0.240, 0.020, 320, 0.040, 6, 0.105, 0.8);
    ring(0.252, 0.018, 320, 0.050, 10, -0.09, 0.65);
    // inner set — offset inward, lighter density
    ring(0.175, 0.014, 300, 0.05, 8, 0.12, 0.6);
    ring(0.190, 0.014, 300, 0.04, 11, -0.10, 0.55);
    ring(0.165, 0.012, 230, 0.06, 6, 0.14, 0.5);
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
    W = canvas.width = window.innerWidth;
    H = canvas.height = window.innerHeight;
    cx = W / 2; cy = H / 2; R = Math.min(W, H) * 0.34;
  }

  // heartbeat (double-thump) 0..~1
  function beat(time) {
    const x = (time % 1.15) / 1.15;
    const p1 = Math.exp(-Math.pow((x - 0.10) / 0.045, 2));
    const p2 = 0.55 * Math.exp(-Math.pow((x - 0.26) / 0.05, 2));
    return Math.min(1, p1 + p2);
  }

  function draw(dt) {
    ctx.clearRect(0, 0, W, H);
    ctx.globalCompositeOperation = "lighter";
    t += dt;
    const bt = beat(t);

    // rings
    for (const p of rings) {
      p.a += p.speed * dt;
      const baseR = R * p.rFrac;
      const r = baseR * (1 + p.amp * Math.sin(p.k * p.a)) + p.off * p.thFrac * R;
      const x = cx + r * Math.cos(p.a), y = cy + r * Math.sin(p.a);
      const close = 1 - Math.min(1, Math.abs(p.off));
      const a = (0.05 + 0.7 * close) * p.bright * (0.4 + 0.6 * p.b);
      ctx.fillStyle = col(p.lite ? light : accent, a);
      ctx.beginPath(); ctx.arc(x, y, p.sz, 0, 6.283); ctx.fill();
    }

    // soft glow behind the ball
    const Rs = R * 0.17 * (1 + 0.10 * bt);
    ctx.fillStyle = col(accent, 0.05 + 0.04 * bt);
    ctx.beginPath(); ctx.arc(cx, cy, Rs * 2.4, 0, 6.283); ctx.fill();

    // particle sphere
    for (const s of sphere) {
      s.ph += s.spin * dt;
      const rad = Rs * s.rr;
      const sinth = Math.sin(s.th);
      const x3 = rad * sinth * Math.cos(s.ph);
      const y3 = rad * sinth * Math.sin(s.ph);
      const z3 = rad * Math.cos(s.th);
      const front = (z3 / Rs + 1) / 2;
      const a = Math.min(0.95, (0.10 + 0.8 * front) * (0.4 + 0.6 * s.b) * (0.8 + 0.4 * bt));
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
    start, stop,
  };

  resize(); build(); start();
})();

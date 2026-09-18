# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-09-18
**Current phase:** Phases 1–14 merged to `main` (all CI-green). **v0.1.0 is
published on the Releases page** (`Fay_0.1.0_x64-setup.exe`). Phase 15 (README)
in PR; Phase 16 (Ctrl+Mouse5) code written, next PR. Owner has not yet
runtime-tested — install v0.1.0 (or any newer CI artifact) and report.

## ✅ Done (all merged)

| Phase | What |
| :-- | :-- |
| 0 | Repo scaffold, `.md` context system, locked stack (DECISIONS.md) |
| 1 | Tray icon, summon hotkey, hide-on-blur/Esc, elevated launch (UAC), `list_monitors` readout, icons |
| 2 | Real deck (Zen, Discord, Claude, LifeOS, Phone Link, Steam, Task Manager; scenes Focus/Game/Side Stack), UI polish |
| 3 | Per-scene audio output via `audioOut` (SoundVolumeView; Console+Multimedia roles only — Discord untouched) |
| 4 | Config-driven summon hotkey (`app.hotkey`, `set_summon_hotkey`) |
| 5 | Autostart at login (`app.autostart`, tauri-plugin-autostart) |
| 6 | **The Heart** — full-monitor translucent overlay, `<canvas>` particle HUD (layered multi-speed rings + inner set + pulsing particle sphere as the open button), `accent: "auto"` (Windows accent via `get_accent_color`), `backdrop` alpha |
| 7 | CI-built NSIS installer artifact on every push/PR (`Fay-windows-installer`) — test without a toolchain |
| 8 | Direct scene/app hotkeys — per-tile `hotkey` fires the tile (with `audioOut`) without opening Fay; unified `HotkeyState` |
| 9 | Installed-build readiness — user-editable config seeded to `%APPDATA%\com.fay.hub\apps.config.json` (tray: Open config / Reload), starts hidden (`startHidden`), amber `warn()` surfacing in the footer (in PR) |
| 10 | Number-key launch (1–9) + type-to-filter with Enter-to-fire, Esc clears; frontend only (in PR) |
| 11 | Release workflow — on push to `main`, publishes `v<version>` (from package.json) on the GitHub Releases page if not yet released; creates the tag itself (tag pushes aren't possible from the agent) |
| 12 | Real app icons — `get_app_icon` extracts + caches each target's icon; glyph fallback; `icon` override (in PR) |
| 13 | Running-app indicator — `list_running` + per-tile process-name guess (`process` override); glowing dot; polled only while open (in PR) |
| 14 | Per-machine profiles — `machines.<HOST>.tiles.<id>` overlay merged onto tiles; `get_hostname`; host shown in footer (next PR) |
| — | **Debug round** — no console flashes (`CREATE_NO_WINDOW`), blocking commands made `async` (no UI freeze), Heart idle while hidden, HiDPI-crisp canvas, no runaway polling, broken-config fallback (in PR) |
| 15 | README rewritten as the product front door — preview, 30-second install, feature table, full config reference (in PR) |
| 16 | Ctrl+Mouse5 summon — `app.mouseSummon`, WH_MOUSE_LL hook thread (`windows-sys`), swallows the click; `set_mouse_summon` (next PR) |

## ⚙️ CI (`.github/workflows/ci.yml`)

- **config** (Ubuntu) — JSON validation + `node -c` on `app.js` / `heart.js`.
- **build-check** (Windows) — `cargo check` compiles the Tauri app on every push/PR.
- **installer** (Windows) — `tauri build --bundles nsis` → uploads
  `Fay-windows-installer` artifact on every push/PR. Download it from the
  Actions run → no toolchain needed to test.

## 📌 Pending — owner tasks (can't be automated)

- [ ] **Run Fay** — install **v0.1.0 from the Releases page** (or any newer CI
      artifact) and give feedback on the Heart — density, pulse, backdrop
      dimness, ball size, tile layout — and whether Ctrl+Mouse5 works.
- [ ] Verify the exe paths for **Zen / Claude / LifeOS** — edit via tray › *Open
      config file* (`%APPDATA%\com.fay.hub\apps.config.json`), then *Reload config*.
- [ ] Create the three **PowerToys Workspaces** + desktop shortcuts
      (`Fay-Focus.lnk`, `Fay-Game.lnk`, `Fay-Side.lnk`). See SETUP.md.
- [ ] Install **SoundVolumeView.exe** (PATH or beside Fay) for scene `audioOut`.
- [ ] Runtime checks: tray appears, Ctrl+Alt+Space toggles, a UAC tile prompts,
      footer shows real monitor count, accent matches Windows.

## 🚀 Pipeline — buildable next (owner picks order)

| Feature | Value | Notes |
| :-- | :-- | :-- |
| *(build-day queue complete after Phase 16)* | | Next candidates, owner's pick: search + categories (deck > ~20 tiles), config validation with line numbers, cursor-monitor summon (show on the monitor under the cursor), code signing to silence SmartScreen |
| Per-machine profiles | med | different paths per PC; fixes absolute-path portability |
| Running-app indicator | med | dot on apps already open |
| First-run path picker | med | "browse…" to capture exe paths |

## 🧭 Later / parked

- **Ctrl+Mouse5 summon** — needs a low-level Windows mouse hook (WH_MOUSE_LL);
  keyboard-only plugin can't do it. Build when the owner can test interactively.
- Search + categories once the deck exceeds ~20 tiles.
- Repo default branch is still `claude/charming-ramanujan-shirak`; consider
  switching it to `main` in GitHub settings.

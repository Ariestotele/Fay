# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-09-18
**Current phase:** Phases 1–9 merged to `main` (all CI-green). **Phases 10–11
(keyboard launch/filter + Release workflow) in PR.** The Phase 9 merge's
installer is the first build the owner should install. Build-day queue in
progress (see Pipeline).

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
| 11 | Release workflow — `v*` tag builds the installer and publishes it on the GitHub Releases page (in PR; first tag `v0.1.0`) |
| 12 | Real app icons — `get_app_icon` extracts + caches each target's icon; glyph fallback; `icon` override (next PR) |

## ⚙️ CI (`.github/workflows/ci.yml`)

- **config** (Ubuntu) — JSON validation + `node -c` on `app.js` / `heart.js`.
- **build-check** (Windows) — `cargo check` compiles the Tauri app on every push/PR.
- **installer** (Windows) — `tauri build --bundles nsis` → uploads
  `Fay-windows-installer` artifact on every push/PR. Download it from the
  Actions run → no toolchain needed to test.

## 📌 Pending — owner tasks (can't be automated)

- [ ] **Run Fay** (install the CI artifact from the Phase 9 merge) and give
      feedback on the Heart — density, pulse, backdrop dimness, ball size, tile layout.
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
| Running-app indicator | med | dot on apps already open — **next** |
| Per-machine profiles | med | different paths per PC; fixes absolute-path portability |
| Running-app indicator | med | dot on apps already open |
| First-run path picker | med | "browse…" to capture exe paths |

## 🧭 Later / parked

- **Ctrl+Mouse5 summon** — needs a low-level Windows mouse hook (WH_MOUSE_LL);
  keyboard-only plugin can't do it. Build when the owner can test interactively.
- Search + categories once the deck exceeds ~20 tiles.
- Repo default branch is still `claude/charming-ramanujan-shirak`; consider
  switching it to `main` in GitHub settings.

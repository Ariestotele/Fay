# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-09-18
**Current phase:** Phases 1–7 merged to `main` (all CI-green). **Phase 8 (direct
scene/app hotkeys) in PR.** Build-day queue in progress (see Pipeline). Owner has
not yet runtime-tested; installer artifacts are available per push.

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
| 8 | Direct scene/app hotkeys — per-tile `hotkey` fires the tile (with `audioOut`) without opening Fay; unified `HotkeyState` (in PR) |

## ⚙️ CI (`.github/workflows/ci.yml`)

- **config** (Ubuntu) — JSON validation + `node -c` on `app.js` / `heart.js`.
- **build-check** (Windows) — `cargo check` compiles the Tauri app on every push/PR.
- **installer** (Windows) — `tauri build --bundles nsis` → uploads
  `Fay-windows-installer` artifact on every push/PR. Download it from the
  Actions run → no toolchain needed to test.

## 📌 Pending — owner tasks (can't be automated)

- [ ] **Run Fay** (install the CI artifact, or `npm run dev`) and give feedback
      on the Heart — density, pulse, backdrop dimness, ball size, tile layout.
- [ ] Verify the exe paths for **Zen / Claude / LifeOS** in `src/apps.config.json`.
- [ ] Create the three **PowerToys Workspaces** + desktop shortcuts
      (`Fay-Focus.lnk`, `Fay-Game.lnk`, `Fay-Side.lnk`). See SETUP.md.
- [ ] Install **SoundVolumeView.exe** (PATH or beside Fay) for scene `audioOut`.
- [ ] Runtime checks: tray appears, Ctrl+Alt+Space toggles, a UAC tile prompts,
      footer shows real monitor count, accent matches Windows.

## 🚀 Pipeline — buildable next (owner picks order)

| Feature | Value | Notes |
| :-- | :-- | :-- |
| Number-key launch + type-to-filter | med | frontend-only quick wins — **next** |
| GitHub Release workflow (v0.1.0) | med | installer attached to a Releases page |
| Real app icons | high | extract each app's icon instead of glyphs |
| Per-machine profiles | med | different paths per PC; fixes absolute-path portability |
| Running-app indicator | med | dot on apps already open |
| First-run path picker | med | "browse…" to capture exe paths |

## 🧭 Later / parked

- **Ctrl+Mouse5 summon** — needs a low-level Windows mouse hook (WH_MOUSE_LL);
  keyboard-only plugin can't do it. Build when the owner can test interactively.
- Search + categories once the deck exceeds ~20 tiles.
- Repo default branch is still `claude/charming-ramanujan-shirak`; consider
  switching it to `main` in GitHub settings.

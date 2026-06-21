# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-06-21
**Current phase:** Phases 1–3 merged. Phase 4 (config-driven hotkey) in PR.
Awaiting owner's Windows runtime check + PowerToys Workspace creation.

## ✅ Done

- **Phase 0** — repo scaffold, `.md` context system, locked stack (DECISIONS.md).
- Config-driven frontend prototype (`src/`) renders tiles from `apps.config.json`.
- **Phase 1 (code complete):**
  - System tray icon (show/hide + Quit) — `src-tauri/src/main.rs`.
  - Global summon hotkey **Ctrl+Alt+Space**; window hides on focus loss + Escape.
  - **Elevated apps:** `"elevated": true` tiles launch via `Start-Process -Verb RunAs`.
  - **Multi-monitor:** `list_monitors` command + live display readout in footer.
  - Real icons generated (`src-tauri/icons/`) so tray + bundling work.
  - Elevated tiles show an `ADMIN` badge in the UI.

## ⚙️ CI (added 2026-06-21)

`.github/workflows/ci.yml` runs on every push/PR:

- **config** (Ubuntu) — validates the JSON files + frontend JS syntax.
- **build-check** (Windows) — `cargo check` actually compiles the Tauri app, so
  the Rust/tray/global-shortcut code is verified automatically (no manual
  Windows machine needed). Runtime checks (tray appears, UAC prompt, hotkey)
  still need a real interactive session — see below.

## ⚠️ Needs verifying (interactive, can't be done in CI)

CI compiles the app, but these are runtime behaviors to confirm on Windows:

1. `npm install && npm run dev` — window opens.
2. Tray icon appears and Ctrl+Alt+Space toggles the window.
3. An `elevated` tile raises a UAC prompt.
4. The footer shows your real monitor count.

## ✅ Phase 2 (merged — PR #2)

- UI polish: staggered tile entrance, arrow-key nav, focus-visible ring.
- Real deck wired into `apps.config.json`: apps (Zen, Discord, Claude, LifeOS,
  Phone Link, Steam, Task Manager) + scenes (Focus, Game, Side Stack).
- Game/Side scenes match the owner's actual side-screen layout (quartered
  Discord + Task Manager + Zen + LifeOS, game on the ASUS main).
- SETUP.md documents which tiles work out of the box vs. need a path check, and
  how to build each scene's PowerToys Workspace.

## 📌 Pending — owner tasks (can't be automated)

- [ ] Verify the exe paths for **Zen / Claude / LifeOS** in `src/apps.config.json`.
- [ ] Create the three **PowerToys Workspaces** + desktop shortcuts
      (`Fay-Focus.lnk`, `Fay-Game.lnk`, `Fay-Side.lnk`). See SETUP.md.
- [ ] Run the interactive **Windows smoke test** (window opens, tray,
      Ctrl+Alt+Space toggle, a UAC tile, footer monitor count).
- [ ] Install **SoundVolumeView.exe** (PATH or beside Fay) for scene `audioOut`,
      and confirm the **Game** scene flips audio to the headset without moving Discord.

## ✅ Phase 3 (merged) — scene audio output

- Scenes can set the default playback device via `audioOut` (Console+Multimedia
  roles only, so Discord/Communications is untouched). Backend
  `set_audio_output` shells to NirSoft SoundVolumeView. Wired into scene clicks.
- Game scene set to switch audio to the Hyper X headset.

## ✅ Phase 4 (in PR) — config-driven summon hotkey

- `app.hotkey` in `apps.config.json` sets the summon combo; `set_summon_hotkey`
  parses + re-registers it at load. `Ctrl+Alt+Space` is the fallback.
- Keyboard-only; mouse-button summon still needs the deferred mouse hook.

## 🧭 Later

- **Ctrl+Mouse5 summon** — requested; needs a low-level Windows mouse hook
  (WH_MOUSE_LL), separate from the keyboard-only global-shortcut plugin. Own phase.
- Make the summon hotkey configurable from `apps.config.json`.
- Search + categories once tiles exceed ~20.
- `machine`/profile key so paths differ per computer.
- Autostart on boot.

# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-06-21
**Current phase:** Phase 2 in progress — Phase 1 merged to `main` (CI green)

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

## ✅ Phase 2 (in progress)

- UI polish: staggered tile entrance, arrow-key nav, focus-visible ring.
- Real deck wired into `apps.config.json`: apps (Zen, Discord, Claude, LifeOS,
  Phone Link, Steam, Task Manager) + scenes (Focus, Game, Side Stack).
- SETUP.md documents which tiles work out of the box vs. need a path check, and
  how to build each scene's PowerToys Workspace.

## 🔜 Phase 2 remaining / next

- **User action:** verify the exe paths for Zen / Claude / LifeOS, and create the
  three PowerToys Workspace shortcuts (Fay-Focus/Game/Side.lnk). See SETUP.md.
- Optional: make the summon hotkey configurable from `apps.config.json`.
- Deferred: Ctrl+Mouse5 summon (needs a low-level mouse hook — see DECISIONS.md).

## 🧭 Later (Phases 3–4)

- Search + categories once tiles exceed ~20.
- `machine`/profile key so paths differ per computer.
- Autostart on boot.

## ⚠️ Open questions for the user

- Which real apps/scenes do you want as the starting set?
- Keep summon hotkey at Ctrl+Alt+Space, or change it?

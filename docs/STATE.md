# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-06-21
**Current phase:** Phase 1 implemented (needs build verification on Windows)

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

## 🔜 Next (Phase 2)

- Polish dark UI (hover/animation/glow refinements).
- Wire your **real** apps + a real PowerToys Workspace `.lnk` into a scene.
- Optional: make the summon hotkey configurable from `apps.config.json`.

## 🧭 Later (Phases 3–4)

- Search + categories once tiles exceed ~20.
- `machine`/profile key so paths differ per computer.
- Autostart on boot.

## ⚠️ Open questions for the user

- Which real apps/scenes do you want as the starting set?
- Keep summon hotkey at Ctrl+Alt+Space, or change it?

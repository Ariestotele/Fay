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

## ⚠️ Needs verifying (couldn't build in the Linux dev container)

Phase 1 is written but **not yet compiled** — the dev container is Linux and
can't build a Windows Tauri app. On a Windows machine:

1. `npm install && npm run dev` — confirm the window opens.
2. Confirm tray icon appears and Ctrl+Alt+Space toggles the window.
3. Confirm an `elevated` tile raises a UAC prompt.
4. Confirm the footer shows your real monitor count.

If the Tauri v2 tray/global-shortcut API has drifted, the two spots to check are
the `.setup(...)` tray block and the `tauri_plugin_global_shortcut` calls in
`main.rs`.

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

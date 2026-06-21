# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-06-21
**Current phase:** Phase 0 complete → Phase 1 next

## ✅ Done

- Repo scaffolded: docs system, config, frontend prototype, Tauri skeleton.
- `.md` context system in place (CLAUDE.md + docs/).
- Locked stack: Tauri v2 + PowerToys Workspaces hybrid (see DECISIONS.md).
- Frontend prototype (`src/`) renders tiles from `src/apps.config.json` — works
  in a browser preview AND wires to the Tauri `launch` command.
- Backend skeleton: `launch(target)` command (`src-tauri/src/main.rs`).

## 🔜 Next (Phase 1)

1. Install toolchain + run `npm run dev` to confirm the Tauri window builds.
   (See SETUP.md — needs Rust + Node + WebView2 on a Windows machine.)
2. Add app icons (`npm run tauri icon`) so bundling works.
3. System tray icon + global hotkey to summon/hide the Fay window.
4. Real PowerToys Workspace shortcuts wired into a couple of `scenes`.

## 🧭 Later (Phases 2–4)

- Polish dark UI (animations, hover, glow).
- Search + categories once tiles exceed ~20.
- `machine`/profile key so paths differ per computer.
- Autostart on boot.

## ⚠️ Open questions for the user

- Which real apps/scenes do you want as the starting set? (Currently placeholder
  examples in `src/apps.config.json`.)
- Preferred summon hotkey? (Suggested: `Win+Alt+Space`.)

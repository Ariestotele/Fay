# Architecture

## The core idea

Fay separates the two halves of the problem:

1. **The hub UI** — a custom, dark landing page. Fully ours. Easy.
2. **Window positioning** — hard and OS-specific. We do **not** build this.
   PowerToys Workspaces does it; Fay just triggers it.

```
┌─ Tauri v2 app (later: system tray + global hotkey to summon) ─┐
│  Frontend (src/)  — the dark "techy" hub UI                   │
│     • grid of app tiles + scene tiles                         │
│     • 100% driven by src/apps.config.json                     │
│  Backend (src-tauri/, minimal Rust)                           │
│     • one command: launch(target) → starts an .exe or .lnk    │
└───────────────────────────────────────────────────────────────┘
        │ reads
        ▼
   src/apps.config.json   ← you edit this to add apps/scenes
        │ each tile's "target" points to …
        ▼
   .exe / command  OR  a PowerToys Workspace .lnk  → windows snap
```

## The trigger mechanism (the linchpin)

PowerToys Workspaces can save a layout and **generate a desktop shortcut** that
launches that whole app-group into position. Fay's "scene" tiles simply run that
`.lnk`. Single-app tiles run a plain command/exe.

So Fay never positions a window itself. The backend exposes a few small commands:

```rust
launch(target, elevated)  // un-elevated: cmd /C start "" <target>
                          // elevated:    powershell Start-Process -Verb RunAs
list_monitors()           // current display layout (for the footer readout)
hide_window()             // Escape-to-hide
```

`launch` handles `.exe`, plain commands on PATH (`code`, `spotify`), `.lnk`
files, and URLs uniformly. `elevated: true` tiles raise a UAC prompt so admin
apps work without Fay itself running elevated.

### Tray + hotkey (Phase 1)

- A **system tray** icon shows/hides Fay and offers Quit.
- A **global hotkey** (Ctrl+Alt+Space) summons/hides the window.
- The window **hides on focus loss** — launcher behavior; re-summon via hotkey/tray.

All of this is wired Rust-side in `src-tauri/src/main.rs`, so the frontend needs
no extra capability permissions beyond `core:default`.

## Config schema (`src/apps.config.json`)

```jsonc
{
  "app":   { "name": "Fay", "tagline": "command deck", "columns": 4 },
  "scenes": [
    { "id": "work", "name": "Work", "glyph": "▞",
      "target": "C:\\Users\\you\\Desktop\\Work.lnk",
      "hint": "Editor · Browser · Music across both screens" }
  ],
  "apps": [
    { "id": "code", "name": "VS Code", "glyph": "</>", "target": "code" }
  ]
}
```

- `scenes` = multi-window layouts (point at PowerToys Workspace shortcuts).
- `apps`   = single launches (exe / command / path / URL).
- `glyph`  = the little mono symbol shown on the tile (keeps it techy, no icon assets).

## Why config lives in `src/` (not a separate `config/`)

The frontend `fetch()`es the config at runtime, so it must sit inside the
bundled frontend dir (`src/`). Keeping one copy avoids a build-time copy step —
fewer moving parts, fewer bugs. (See DECISIONS.md.)

## Where it can go (scalability)

- Search + categories when tiles exceed ~20.
- `machine`/profile key so absolute paths can differ per computer.
- Tray icon + global hotkey to summon Fay (Phase 1).
- If we ever outgrow PowerToys: swap the trigger for `komorebic` (komorebi CLI)
  without touching the UI — the UI only knows "run this target".

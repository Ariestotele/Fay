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
set_audio_output(device)  // set default playback device (Console+Multimedia roles)
set_summon_hotkey(accel)  // re-register the summon hotkey from config
set_autostart(enabled)    // launch-at-login toggle
get_accent_color()        // Windows accent color (#rrggbb) for accent: "auto"
fire(action)              // perform any tile: launch / system / media / snippet
set_clipboard(text)       // native CF_UNICODETEXT (calculator "Enter copies")
set_summon_monitor(mode)  // "cursor" (monitor under the mouse) or "current"
list_commands(dir?)       // scripts in the commands folder → tiles
get_stats()               // CPU / RAM / NET via sysinfo (+ GPU via nvidia-smi)
notify(title, body)       // Windows toast (focus timer done while hidden)
```

The Heart exposes `setStats`, `setStatsVisible`, `setProgress` (focus arc) and
`pulse()` (a one-second burst used for "done" and, later, spoken replies).
The focus timer itself is frontend state (`window.__fayFocus`); a hotkey
reaches it through a `kind: "focus"` action that the backend turns into an
`eval` on the main window.

`TileAction` is recursive: `multi` carries `steps` (each a TileAction, or
`wait`), `close` carries `closes`. Folders are frontend-only (`children`); the
deck swaps the three groups for the folder's grid and a breadcrumb.

### Tile kinds (Phase 17)

Every tile resolves to one `TileAction` `{kind, target, elevated, audioOut,
action, text, paste}`; the frontend builds it (`actionOf`) both for clicks
(`fire`) and for direct hotkeys (`register_item_hotkeys`), so the two paths are
identical. `system` actions are a fixed command table in Rust (nothing from
the config is interpolated into a shell); `media` and the snippet paste use
`SendInput`; the clipboard is written natively. Preset tiles come from
`src/packs.json` (`packs: [...]` in the config) and land in a third group,
`system`. The filter bar doubles as a calculator / unit converter (`= …`) and
as the quicklink prompt (`yt lofi`) — all in `app.js`, no backend.

### The Heart (HUD)

The frontend is a full-monitor translucent overlay (`fill_active_monitor` sizes
the window to the active display; `transparent` + a `backdrop` alpha let the
wallpaper show through, dimmed). The centerpiece is a `<canvas>` particle field
(`heart.js`): layered multi-speed rings + an inner set + a pulsing particle
sphere. The sphere is the **button** — click it to bloom the tiles (`body.open`);
Esc / clicking the backdrop collapses back to rest. Rendering pauses on window
blur, so it costs nothing while hidden. Accent comes from `app.accent` —
`"auto"` pulls the live Windows accent via `get_accent_color`.

### Scene audio output (optional)

A scene (or app) tile may carry an `audioOut` field naming a playback device.
When clicked, Fay calls `set_audio_output` *before* launching the layout. It sets
the **Console + Multimedia** default roles only and leaves **Communications**
alone — so games/media move to the chosen device while Discord (which uses its
own pinned device, or the Communications default) is untouched. This relies on
NirSoft `SoundVolumeView.exe` since Windows ships no CLI for it (see SETUP.md).

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

## Where the config lives

`src/apps.config.json` is the **bundled default** (it must sit inside the
frontend dir so it ships with the app). Installed builds don't read it directly:
on first run the frontend seeds a **user copy** in the app config dir
(`%APPDATA%\com.fay.hub\apps.config.json`, via `load_config` / `save_config`)
and loads from there thereafter, so the installed app is customizable without a
rebuild. Tray › *Open config file* / *Reload config* edit and re-apply it. In
browser preview (no Tauri) the bundled file is used as-is.

## Where it can go (scalability)

- Search + categories when tiles exceed ~20.
- `machine`/profile key so absolute paths can differ per computer.
- Tray icon + global hotkey to summon Fay (Phase 1).
- If we ever outgrow PowerToys: swap the trigger for `komorebic` (komorebi CLI)
  without touching the UI — the UI only knows "run this target".

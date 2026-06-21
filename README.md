# Fay

> A minimal, dark, techy **command deck** for your Windows desktop.
> One landing page → open your apps, or snap whole window-layouts across both monitors.

Fay is a personal launcher hub. It does **not** reinvent window management — it
shows a beautiful grid of tiles, and each tile either launches an app or fires a
[PowerToys Workspaces](https://learn.microsoft.com/en-us/windows/powertoys/workspaces)
layout that positions a group of windows exactly where you want them.

```
┌──────────────────────────────────────────────┐
│  FAY · command deck                           │
│                                               │
│   ▞ Work      ◳ Focus     ⧉ Comms             │   ← scenes (multi-window layouts)
│                                               │
│   </> Code   ◯ Browser   ▸_ Term   ▤ Files   │   ← single apps
│   ♪ Spotify  ✎ Notion                         │
└──────────────────────────────────────────────┘
```

## Why it's built this way

- **Tauri v2** — tiny (~3MB), low memory, perfect for an always-available hub.
- **Config-driven** — add apps/scenes by editing one JSON file, no rebuild.
- **Hybrid positioning** — PowerToys Workspaces does the hard window-snapping;
  Fay is the pretty front door that triggers it.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full picture and
[`docs/DECISIONS.md`](docs/DECISIONS.md) for why each choice was made.

## Status

Early scaffold. See [`docs/STATE.md`](docs/STATE.md) for current progress.

## Quick start

See [`docs/SETUP.md`](docs/SETUP.md).

# Setup

## Prerequisites (Windows)

1. **PowerToys** — install from the Microsoft Store or
   https://learn.microsoft.com/windows/powertoys/install , then enable the
   **Workspaces** utility.
2. **Node.js** (LTS) — https://nodejs.org
3. **Rust** — https://rustup.rs
4. **WebView2** — preinstalled on Windows 11; on Windows 10 the Tauri installer
   handles it.

## First-time init

This repo ships a hand-written Tauri skeleton. If the toolchain complains about
missing generated files, run a one-time icon/scaffold pass:

```
npm install
npm run tauri icon assets/icon.png   # generates src-tauri/icons/* (optional until you bundle)
```

## Run (dev)

```
npm install
npm run dev          # opens the Fay window with hot reload
```

## Build (release)

```
npm run build        # produces an installer in src-tauri/target/release/bundle/
```

## Preview the UI without Tauri

The frontend is plain HTML/CSS/JS. Serve `src/` with any static server to see the
hub (launching is a no-op outside Tauri):

```
npx serve src
```

## Adding apps / scenes

Edit **`src/apps.config.json`**. No rebuild needed in dev — refresh the window.

- **App tile** → `target` is an exe/command/path/URL (e.g. `"code"`, `"spotify"`).
- **Scene tile** → `target` is a PowerToys Workspace shortcut, e.g.
  `"C:\\Users\\you\\Desktop\\Work.lnk"`.

### Creating a scene (PowerToys side)

1. Open the **Workspaces editor** (`Win+Ctrl+`` `).
2. Arrange your apps across both monitors, save the workspace.
3. Use "Create desktop shortcut".
4. Put that `.lnk` path into a `scenes[].target` in `apps.config.json`.

## Caveats (read before relying on it)

- PowerToys must stay installed — it owns the layouts, Fay only triggers them.
- Scenes launch **fresh** app instances; they don't re-snap already-open windows.
- Some apps (Store/UWP, custom launchers, multi-window browsers) restore imperfectly.
- Absolute paths aren't portable across machines/users — plan profiles for that.
- Unplugging a monitor / changing DPI can push saved positions off-screen.
- Admin-elevated apps can't be launched by a normal-privilege Fay.

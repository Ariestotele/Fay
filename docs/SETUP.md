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

## Summoning Fay

- **Ctrl+Alt+Space** toggles the Fay window (registered globally).
- Left-click the **tray icon** (or its menu) to show/hide; tray menu has Quit.
- **Escape** or clicking away hides the window (it keeps running in the tray).

## Elevated (admin) apps

Set `"elevated": true` on a tile in `apps.config.json`:

```json
{ "id": "adminps", "name": "Admin PS", "glyph": "⛨", "target": "powershell", "elevated": true }
```

Fay launches it via `Start-Process -Verb RunAs`, so you'll get a normal **UAC
prompt**. Fay itself stays un-elevated.

### Want no UAC prompt for a specific admin app?

Use a scheduled task (standard Windows trick):

1. Task Scheduler → Create Task → check **Run with highest privileges**.
2. Action = the program you want elevated.
3. In `apps.config.json`, point the tile at the task instead:
   `"target": "schtasks", "elevated": false` won't pass args — instead use a
   `.lnk`/`.bat` that runs `schtasks /run /tn "YourTaskName"` and target that.

> Note: PowerToys can only *position* an elevated app's window if PowerToys is
> also running elevated. If your admin apps must be snapped into a scene, run
> PowerToys as administrator.

## Multi-monitor

Fay shows your live display layout in the footer (e.g. `⧉ 2 displays · 5120×1440`).
This is intentional: if a scene opens windows off-screen, the footer tells you
why (your display setup differs from when the scene was saved).

To make scenes resilient:

- Save a PowerToys Workspace **per physical setup** (e.g. `Work-2mon`, `Work-laptop`)
  and add a tile for each.
- If windows land off-screen after a monitor change, press **Win+Shift+←/→** to
  pull a focused window back onto the active display.

## Your starter deck

`apps.config.json` ships pre-filled with your apps and scenes. A few need a
one-time check because paths are install-specific (see the `hint` on each tile):

**Apps**
- **Phone Link** (`ms-phone:`) and **Steam** (`steam://open/main`) work as-is —
  they use protocol handlers, no path needed.
- **Task Manager** (`taskmgr`) works as-is. Add `"elevated": true` if you want it
  to start with admin rights.
- **Zen**, **Claude**, **LifeOS** — verify the path. Launch the app once, then in
  Task Manager → right-click → *Open file location* to get the real `.exe` path,
  and paste it into the tile's `target`. LifeOS can also be a URL if it's a web app.
- **Discord** points at its Start-Menu shortcut, which survives Discord's
  auto-updates better than the versioned `app-x.x.x` folder.

**Scenes** point at PowerToys Workspace shortcuts you create once:

| Tile | Build this workspace |
|------|----------------------|
| **Focus** | Zen maximized on the ASUS main screen; Discord + Task Manager + LifeOS quartered on the side screen. |
| **Game** | Leave the ASUS main screen for your game; side screen quartered with Discord (top-left), Task Manager (top-right), Zen (bottom-left), LifeOS (bottom-right). |
| **Side Stack** | Side screen only — quarter Discord, Task Manager, Zen, LifeOS. |

For each: open the Workspaces editor, arrange the windows, **Save**, then
**Create desktop shortcut**. Rename the shortcut to match the config
(`Fay-Focus.lnk`, `Fay-Game.lnk`, `Fay-Side.lnk`) or edit the tile `target` to
whatever PowerToys named it.

## Changing the summon hotkey

Set `app.hotkey` in `apps.config.json` — no rebuild needed, just refresh:

```json
"app": { "name": "Fay", "hotkey": "Ctrl+Alt+Space" }
```

Accepted modifiers: `Ctrl`/`Control`, `CmdOrCtrl`/`CommandOrControl`, `Alt`,
`Shift`, `Super` (the Win key). Combine with `+` and a key, e.g.
`CmdOrCtrl+Shift+Space`, `Alt+Backquote`. **Keyboard only** — mouse buttons such
as Mouse5 aren't supported (see "Summon by mouse button" below). An invalid
string is ignored and Fay falls back to the built-in `Ctrl+Alt+Space`.

## Launch at login (autostart)

Set `app.autostart` in `apps.config.json`:

```json
"app": { "name": "Fay", "autostart": true }
```

`true` registers Fay to start when you log in; `false` removes it. The setting is
applied each time Fay loads, so flipping the value and reopening Fay is all it
takes. (In `npm run dev` this registers the *dev* build path — fine for testing,
but enable it for real from an installed build.)

## Switching audio output per scene

A scene can change your **default playback device** when you click it — e.g. the
**Game** scene flips audio to your headset. Discord is left alone (see below).

Windows has no built-in command to set the default device, so Fay shells out to
**NirSoft SoundVolumeView** (free, portable):

1. Download SoundVolumeView from https://www.nirsoft.net/utils/sound_volume_view.html
2. Put `SoundVolumeView.exe` either on your PATH or in the same folder as Fay.
3. Add an `audioOut` field to the scene with the **exact device name** as Windows
   shows it (the parenthesised full name in the sound flyout), e.g.:

   ```json
   { "id": "game", "name": "Game", "target": "...",
     "audioOut": "Hyper X Cloud 3 Wireless (HyperX Cloud III Wireless)" }
   ```

   Tip: run `SoundVolumeView.exe` once to see the precise names to copy.

**Why Discord isn't affected.** Fay sets only the **Console** and **Multimedia**
default roles, not **Communications**. Games and media follow Console/Multimedia,
so they switch; Discord uses its own device selection (or the Communications
default), so it keeps playing/recording where it was. If you *want* Discord to
follow too, pin a device in Discord's voice settings rather than using "Default".

## Summon by mouse button (Ctrl+Mouse5)?

Not supported yet. Tauri's global-shortcut system is **keyboard-only**, so mouse
buttons can't be bound the way `Ctrl+Alt+Space` is. Doing it properly needs a
low-level Windows mouse hook (raw input) running alongside the app — tracked as a
future enhancement. For now the summon hotkey stays **Ctrl+Alt+Space**.

## Reference: app targets & links

The source of truth is `src/apps.config.json` — edit each tile's `target` there.
This table tracks every target and whether you still need to supply a real value.

**App tiles**

| Tile | Current `target` | Status |
| :-- | :-- | :-- |
| Phone Link | `ms-phone:` | ✅ works (protocol) |
| Steam | `steam://open/main` | ✅ works (protocol) |
| Task Manager | `taskmgr` | ✅ works (on PATH) |
| Discord | `%APPDATA%\…\Discord Inc\Discord.lnk` | ✅ works (verify folder name) |
| **Zen** | `%LOCALAPPDATA%\Programs\zen\zen.exe` | ⚠️ **verify the real exe path** |
| **Claude** | `%LOCALAPPDATA%\AnthropicClaude\claude.exe` | ⚠️ **verify the real exe path** |
| **LifeOS** | `lifeos` (placeholder) | ⚠️ **set exe path OR web URL** |

To get a real path: launch the app → Task Manager → right-click it → *Open file
location* → copy the full `.exe` path into the tile's `target`.

**Scene tiles** (each needs a PowerToys Workspace shortcut you create)

| Scene | `target` to create | Extra |
| :-- | :-- | :-- |
| Focus | `%USERPROFILE%\Desktop\Fay-Focus.lnk` | — |
| Game | `%USERPROFILE%\Desktop\Fay-Game.lnk` | `audioOut` → Hyper X headset |
| Side Stack | `%USERPROFILE%\Desktop\Fay-Side.lnk` | — |

**Tool downloads** (only what you don't already have)

| Tool | For | Link |
| :-- | :-- | :-- |
| Git | clone the repo | https://git-scm.com/download/win |
| Python | quick UI preview server | https://www.python.org/downloads/ |
| Node.js | preview alt + full app | https://nodejs.org |
| Rust | full app build | https://rustup.rs |
| PowerToys | scene window layouts | https://learn.microsoft.com/windows/powertoys/install |
| SoundVolumeView | per-scene audio switching | https://www.nirsoft.net/utils/sound_volume_view.html |

## Caveats (read before relying on it)

- PowerToys must stay installed — it owns the layouts, Fay only triggers them.
- Scenes launch **fresh** app instances; they don't re-snap already-open windows.
- Some apps (Store/UWP, custom launchers, multi-window browsers) restore imperfectly.
- Absolute paths aren't portable across machines/users — plan profiles for that.
- Unplugging a monitor / changing DPI can push saved positions off-screen.
- Admin-elevated apps can't be launched by a normal-privilege Fay.

# Setup

## Easiest: install from the CI build (no toolchain)

Every push to `main` builds a Windows installer automatically:

1. Open the repo's **Actions** tab → the latest **CI** run on `main`.
2. Scroll to **Artifacts** → download **`Fay-windows-installer`** (a zip).
3. Unzip and run `Fay_…_x64-setup.exe`. Fay installs and starts (tray icon).
4. Summon with **Ctrl+Alt+Space**.

That's it — no Rust or Node needed. Use the sections below only if you want to
develop or run from source.

**Even easier:** released versions are published on the **Releases** page with
the installer attached — https://github.com/Ariestotele/Fay/releases — no need
to open the Actions tab at all. A release is created automatically whenever
`main` carries a new `version` (bump it in `package.json`,
`src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, merge, done).

## Where your config lives (installed build)

The installer bakes a **default** `apps.config.json` into the app. On first run
Fay copies it to a **user-editable** file and loads from there from then on:

```
%APPDATA%\com.fay.hub\apps.config.json
```

- **Tray › Open config file** opens it in Notepad. Edit, save.
- **Tray › Reload config** applies your changes (no restart, no rebuild).
- Delete the file to reset to the bundled default on next launch.

Fay **starts hidden** in the tray (so autostart doesn't cover your screen at
login) — summon it with **Ctrl+Alt+Space**. It shows itself on the very first
run so you know it's there. Set `"startHidden": false` in the config to always
open on launch.

Problems (a hotkey that failed to bind, a missing SoundVolumeView, a config
typo) show in amber in the footer instead of failing silently.

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

## App icons

Tiles show the app's **real icon** automatically — Fay extracts it from the
target (exe, `.lnk` shortcut, or a command on PATH) and caches it. Nothing to
configure. Protocol targets (`steam://`, `ms-phone:`) have no icon, so those
tiles keep their glyph. To force a specific image, set `"icon"` on the tile to a
URL or a `data:` URL.

## Per-machine profiles

Same config on two PCs but different install paths? Add a `machines` block keyed
by the computer name (shown in Fay's footer, and in Settings › About). Anything
under `tiles.<id>` is merged onto that tile on that machine only:

```json
"machines": {
  "DESKTOP-GAMING": {
    "tiles": {
      "zen":  { "target": "D:\\Apps\\Zen\\zen.exe" },
      "game": { "audioOut": "Speakers (Realtek Audio)" }
    }
  }
}
```

Any tile field can be overridden (`target`, `audioOut`, `hotkey`, `elevated`,
`process`, `icon`, `hint`). Machine names are matched case-insensitively.

## Running-app indicator

Apps that are already open show a small glowing dot in the tile's corner
(refreshed when the deck opens and every 5s while it's open). Fay guesses the
process name from the target (`…\zen.exe` → `zen`, `Discord.lnk` → `discord`,
`steam://` → `steam`). If a guess is wrong, set `"process"` on the tile to the
name shown in Task Manager's *Details* tab (without `.exe`).

## Keyboard (when the deck is open)

- **1–9** — fire the tile with that number (the first nine visible tiles are numbered).
- **Type** — just start typing to filter tiles by name; **Enter** fires the first match.
- **Backspace** edits the filter; **Esc** clears it (press again to close the deck).
- **Arrows** move focus between tiles; **Enter** fires the focused one.
- **`= …`** — start with `=` for a calculation or a conversion (below). Digits
  fire tiles only while nothing is typed, so `=` is how a line that starts with
  a number gets in.

## Calculator & unit conversion

Type an expression into the open deck and the answer appears on the filter
line; **Enter copies it** to the clipboard.

| You type | You get |
| :-- | :-- |
| `= 1440*0.62` | `892.8` |
| `sqrt(2)*3`, `2^10`, `round(pi*100)/100` | `4.2426…`, `1024`, `3.14` |
| `= 5 mi to km`, `= 72 f in c`, `= 1 gb to mib` | `8.04672 km`, `22.2222 c`, `953.674 mib` |

Operators `+ - * / ^ %`, parentheses, `pi`, `e`, and `sqrt abs sin cos tan log
ln round floor ceil min max`. Units: length, mass, volume, time, data, speed,
area, temperature (`c f k`). Currency needs the network and is not built in.

## Quicklinks — `yt lofi`

A tile whose `target` contains `{query}` is a quicklink. Give it a `keyword`
(defaults to its `id`), then type the keyword, a space, and your search:

```json
{ "id": "yt", "name": "YouTube", "keyword": "yt",
  "target": "https://www.youtube.com/results?search_query={query}" }
```

`yt lofi` + **Enter** opens YouTube's search for "lofi" (the query is URL
encoded). Clicking a quicklink tile just pre-fills its keyword. The `links`
pack ships YouTube / Google / GitHub quicklinks.

## System, media and snippet tiles (`kind`)

Besides launching things, a tile can be one of three built-in kinds:

```json
{ "id": "lock", "name": "Lock",         "kind": "system",  "action": "lock" },
{ "id": "next", "name": "Next track",   "kind": "media",   "action": "next", "hotkey": "Ctrl+Alt+N" },
{ "id": "sig",  "name": "Sign-off",     "kind": "snippet", "text": "Best regards,\nMe" }
```

| `kind` | `action` / field | Notes |
| :-- | :-- | :-- |
| `system` | `lock` `sleep` `hibernate` `restart` `shutdown` `logoff` `recycle` (empty Recycle Bin) `darkmode` (toggle Windows dark/light) | `restart` / `shutdown` / `logoff` / `hibernate` need a **second press within 3 s** (set `"confirm": false` to skip, or `true` on any tile to require it). |
| `media` | `playpause` `next` `prev` `stop` `mute` `volup` `voldown` | Sent as real multimedia keys, so Spotify / browser / game respond. Firing one keeps the deck open. |
| `snippet` | `text` | Copies `text` and pastes it (Ctrl+V) into the app that was in front; Fay hides first. `"paste": false` copies only. |

All kinds work with a direct `hotkey` too. Tiles go in `scenes`, `apps` or the
third group, `system`.

### Packs — ready-made tile sets

`"packs": ["power", "media"]` in the config appends preset tiles (defined in
`src/packs.json`). Available: **power** (Lock, Sleep, Restart, Shut down),
**media** (Play/Pause, Next, Previous, Mute, Volume ±), **tools** (Empty
Recycle Bin, Toggle dark mode, Hibernate, Sign out), **settings** (Bluetooth,
Sound, Display, Network, Windows Update, Installed apps via `ms-settings:`),
**links** (YouTube / Google / GitHub quicklinks). A pack tile is skipped when
your config already has a tile with the same id, so you can override any of
them (e.g. give `sys-lock` a hotkey).

## Folders — keep the deck under nine keys

A tile with `children` opens a **sub-deck** instead of launching anything:

```json
{ "id": "web", "name": "Web", "glyph": "▤", "children": [
  { "id": "g",  "name": "Google", "keyword": "g", "target": "https://www.google.com/search?q={query}" },
  { "id": "gh", "name": "GitHub", "target": "https://github.com" }
] }
```

Inside a folder the number keys, typing and Enter work on its children;
**Esc** or **Backspace** (with nothing typed) goes back up. Folders nest.
Quicklinks and hotkeys on children work from anywhere, folder or not.

## Multi-action tiles — one press, several steps

`actions` runs steps in order. Each step is any tile action (`kind`, `target`,
`action`, `text`, `audioOut`…) or a pause `{ "wait": ms }`:

```json
{ "id": "winddown", "name": "Wind down", "actions": [
  { "kind": "media", "action": "playpause" },
  { "wait": 400 },
  { "kind": "system", "action": "lock" }
] }
```

Steps stop at the first failure (reported as *step N: …*). A direct `hotkey`
runs the whole sequence too.

## Scene teardown — close what a scene opened

Give a scene (or any tile) a `closes` list of process names, as shown in Task
Manager's *Details* tab without `.exe`:

```json
{ "id": "game", "name": "Game", "target": "…Fay-Game.lnk",
  "closes": ["Discord", "zen", "Taskmgr"], "closeHotkey": "Ctrl+Alt+Shift+2" }
```

Then **Shift+click** the tile (or **Ctrl+digit**, **Shift+Enter**, or the
**×** that appears on hover) to close them. Windows asks each app to close
politely; add `!` to force one (`"Discord!"`) — note Discord's default "close
to tray" means a polite close only hides it. Apps that weren't running are
listed as a footer note, not an error. `closeHotkey` binds the teardown
globally. A standalone tile can be `"kind": "close"` with a `closes` list.

## Commands folder — drop scripts in, get tiles out

Tray › **Open commands folder** opens `%APPDATA%\com.fay.hub\commands`. Any
`.ps1`, `.bat`, `.cmd`, `.exe` or `.lnk` you put there shows up as a tile in
the *system* group on the next reload, named after the file. `.ps1` scripts run
hidden with `-ExecutionPolicy Bypass`; `.bat` files open their console as usual.
`"commands": false` turns this off; `"commands": "D:\\scripts"` uses another
folder (`%VAR%` allowed).

## Live stats around the Heart

At rest, four readouts orbit the rings: **CPU**, **RAM** (used / total),
**GPU** (load + temperature, NVIDIA only via `nvidia-smi`; hidden otherwise)
and **NET** (down / up per second), each with a dotted gauge. They refresh
every 2.5 s only while Fay is in front and disappear behind the open deck.
`"stats": false` in `app` turns them off; `"statsInterval": 5000` slows them.

## Focus timer (pomodoro)

A `focus` tile starts a countdown; the Heart grows a dotted arc that fills as
time passes and the remaining time sits in the footer:

```json
{ "id": "focus", "name": "Focus 25", "kind": "focus", "minutes": 25, "hotkey": "Ctrl+Alt+F" }
```

Clicking a running focus tile (or its hotkey) stops it; `"action": "stop"`
makes a dedicated stop tile. When time is up the Heart pulses, a Windows toast
appears (so you see it even with Fay hidden) and, once voice is enabled, Fay
says so. The `focus` pack has Focus 25 / Focus 50 / Stop.

## Search: files, bookmarks, clipboard history

With the deck open, a prefix turns the filter line into a results list. Arrows
move, **Enter** opens the selected row, **Shift+Enter** does the row's second
action, Esc backs out.

| Type | Searches | Enter | Shift+Enter |
| :-- | :-- | :-- | :-- |
| `> report 2025` | Files via [Everything](https://www.voidtools.com/) | open | reveal in Explorer |
| `@ tauri docs` | Browser bookmarks (Zen · Firefox · LibreWolf · Floorp · Chrome · Edge · Brave · Vivaldi) | open in default browser | copy URL |
| Clipboard tile / `Ctrl+Alt+V` | Clipboard history (type to filter) | paste into the app in front | copy only |

**Files** need Everything running plus its command-line tool `es.exe`
(download *ES* from the Everything site; put it on PATH or set
`"everything": "C:\\tools\\es.exe"` in `app`). Results are sorted by date
modified, newest first.

**Bookmarks** are read from the browsers' own files: Chromium browsers'
`Bookmarks` JSON, and for Zen / Firefox the daily `bookmarkbackups` file (so a
bookmark added today may show up tomorrow). Cached for five minutes.
`"bookmarks": false` turns it off.

**Clipboard history** keeps the last 50 text copies **in memory only** (never
on disk, gone when Fay quits); entries a password manager marks as
"exclude from monitoring" are skipped. `"clipboard": false` disables the
watcher, `"clipboardMax": 100` changes the size. A `kind: "clipboard"` tile (or
its hotkey, even while Fay is hidden) opens the list.

## Voice — Fay talks, and listens (offline)

Built on Windows' own speech engine (`System.Speech`): nothing to install, no
cloud, works offline. `"voice": true` in `app` turns it on (default in the
starter config); `"voiceRate": 0` is speed (−10…10); `"voiceName"` picks an
installed voice (Settings › Time & language › Speech lists them, e.g.
`"Microsoft Zira Desktop"`); `"voiceConfirm": false` stops the spoken
"Opening …" acknowledgement.

**Spoken replies.** Any tile can carry `"say": "Game mode ready"` — spoken
after it fires (click or hotkey). A `kind: "say"` tile / multi-action step
speaks its `text`. The focus timer announces when it ends. The Heart's sphere
wobbles while Fay speaks.

**Tap-to-talk.** Press the *Listen* tile (or its hotkey, `Ctrl+Alt+L` in the
starter config, which works even while Fay is hidden), wait for the Heart to
breathe, and say a tile name: *"game"*, *"open discord"*, *"start focus 25"*,
*"close game"* (for scenes with `closes`), *"clipboard"*, *"stop focus"*,
*"close fay"*, *"never mind"*. Recognition uses a fixed grammar built from your
tile names, so it's accurate even with a laptop mic, and Fay only listens for
six seconds after you press — never continuously. The first *Listen* after
start takes a second longer while the engine loads.

## AI — talk to Fay in plain language

Type what you want instead of a tile name. If nothing on the deck matches
what you typed, **Enter** hands the line to Fay's model; `? …` asks
explicitly:

| You type | Fay does |
| :-- | :-- |
| `game mode but keep audio on speakers` | fires the Game scene with the audio switch overridden |
| `open zen and discord, then focus 25` | three actions in order |
| `close the game stuff` | runs the Game scene's `closes` list |
| `? what's a good name for this project` | just answers (no actions) |

The reply is shown (and spoken, if voice is on) and the actions run through
the same path as clicks. If a step fails Fay tells the model what went wrong
and lets it adjust once or twice (bounded). Shutdown / restart / sign-out /
hibernate never run unless *your own* message clearly asked for or confirmed
them — the model can only ask. Conversation memory lasts until the deck
closes, so you can answer a follow-up question by just typing.

**Setup.** In `app.ai`:

```json
"ai": { "provider": "anthropic", "model": "claude-sonnet-5", "apiKey": "sk-ant-…" }
```

The key lives only in your local config (`%APPDATA%\com.fay.hub\`), never in
the repo; the `ANTHROPIC_API_KEY` environment variable works too. For a fully
local setup run [Ollama](https://ollama.com) and use
`"ai": { "provider": "ollama", "model": "llama3.2" }` (a vision-capable model
such as `llama3.2-vision` for screen questions). `"ai": false` turns it off.

### Ask about the screen

The **Ask about screen** tile (`Ctrl+Alt+A` in the starter deck, works while
Fay is hidden) captures the window in front *before* Fay appears, then opens
the ask line with 📷 attached: *"what does this error mean?"*, *"summarize
this page"*. The capture is sent once, with that question only, and never
stored on disk. Nothing is captured unless you press it.

## Add app — browse for a program

The dashed **+ Add app** tile opens a normal file picker; the chosen `.exe`,
`.lnk`, `.bat`, `.cmd` or `.ps1` is appended to `apps` in your config and Fay
reloads. `"addTile": false` hides it. (Editing the config file stays the way
to rename, reorder or give it a hotkey.)

## Doctor — the first-run self-check

Press the **Doctor** tile (in the starter deck; also in the `tools` pack, or
add `{ "id": "doctor", "name": "Doctor", "kind": "doctor" }` anywhere). It
runs every check CI can't and shows one report:

- config problems (typos in keys get a *did you mean* hint);
- every launch tile's `target`: file found / on PATH / not found (protocol
  links like `steam://` are just noted);
- helper tools: PowerShell, PowerToys, SoundVolumeView, Everything `es.exe`,
  `nvidia-smi`;
- the clipboard watcher, bookmark count, voice host state, and whether AI has
  a key (for Ollama it also pings the server).

**Enter copies the whole report** to the clipboard — paste it into a chat or an
issue instead of describing what happens tile by tile. Esc closes it.

### Press F to repair broken paths

When an app tile's `target` is missing, Doctor goes looking for the app: Start
Menu shortcuts first (they survive updates, which guessed `.exe` paths don't),
then the usual install roots. Rows that found something say `→ found: …`, and
the footer offers **F fixes N paths**. Pressing **F** writes those paths into
your config and reloads — no hand-editing.

Scene tiles are reported but never searched: their `target` is a PowerToys
Workspace shortcut *you* create, so a same-named app found elsewhere would be
the wrong file. Those rows point you at the Workspaces editor instead.

## Config validation

Every key is checked on load. Unknown or misspelled keys (`"hotkeys"`,
`"audioOutput"`, `"apikey"`) are reported with the nearest real key; values are
type- and range-checked (`accent` must be `"auto"` or `#rrggbb`, `backdrop`
0–1, `voiceRate` −10…10, `ai.provider` `anthropic`/`ollama`, …). Problems show
in the footer and in the Doctor report, and never stop Fay from starting. Keys
starting with `_` are ignored (use them for comments).

## Which monitor Fay opens on

`"summonOn": "cursor"` (default) shows Fay on the monitor under the mouse;
`"current"` keeps it on the monitor it was last on.

## Direct hotkeys — fire a scene or app without opening Fay

Give any scene or app a `hotkey` and pressing it launches the tile instantly,
Fay stays hidden. The default deck binds the three scenes:

```json
{ "id": "game", "name": "Game", "hotkey": "Ctrl+Alt+2", "target": "...", "audioOut": "..." }
```

- Works for apps too (`"hotkey": "Ctrl+Alt+D"` on Discord, say).
- A scene's `audioOut` is applied as well, exactly like clicking the tile.
- Same accelerator format as the summon hotkey (keyboard only). A combo already
  taken by another program is skipped silently; an unparseable one is logged.
- The tile shows its hotkey in the corner so you can see what's bound.

## Changing the summon hotkey

Set `app.hotkey` in `apps.config.json` — no rebuild needed, just refresh:

```json
"app": { "name": "Fay", "hotkey": "Ctrl+Alt+Space" }
```

Accepted modifiers: `Ctrl`/`Control`, `CmdOrCtrl`/`CommandOrControl`, `Alt`,
`Shift`, `Super` (the Win key). Combine with `+` and a key, e.g.
`CmdOrCtrl+Shift+Space`, `Alt+Backquote`. Keyboard combos only here — for a
mouse button use `mouseSummon` (see "Summon by mouse button" below). An invalid
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

## Summon by mouse button (Ctrl+Mouse5)

Set `app.mouseSummon` in the config:

```json
"app": { "mouseSummon": "Ctrl+Mouse5" }
```

`Mouse4` (back) or `Mouse5` (forward), optionally with `Ctrl` / `Alt` / `Shift`.
Pressing the combo toggles Fay exactly like the keyboard hotkey; the click is
swallowed so the app under the cursor doesn't also react to it. Set it to `""`
to turn it off. (Implemented with a low-level Windows mouse hook, since the
keyboard-hotkey system can't bind mouse buttons.)

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

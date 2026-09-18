# Decisions (append-only)

Each entry is a locked decision. Don't silently rewrite — append a new dated
entry that supersedes an old one if something changes.

---

### 2026-06-21 — Name
The app is called **Fay**. Repo is `Fay`.

### 2026-06-21 — Framework: Tauri v2 (not Electron)
Fay is an always-available hub, so footprint matters. Tauri ships ~3MB and uses
the OS WebView vs Electron's ~150MB bundled Chromium. Frontend stays plain web,
so the dark UI is just as easy. Chosen for size + memory.

### 2026-06-21 — Positioning: hybrid via PowerToys Workspaces (not a custom engine)
Window positioning is hard and OS-specific. PowerToys Workspaces already
launches+positions app groups and can emit a desktop shortcut. Fay triggers that
shortcut. Fay does NOT build a positioning engine. Upgrade path if outgrown:
komorebi's `komorebic` CLI.

### 2026-06-21 — Backend is one command
The Rust side exposes a single `launch(target)` command that runs
`cmd /C start "" <target>`. This uniformly handles exe / command / .lnk / URL.
No shell plugin needed → smaller permission surface.

### 2026-06-21 — Config lives in `src/apps.config.json`
The frontend fetches config at runtime, so it must be inside the bundled
frontend dir. Single copy, no build-time copy step. Editing this file is how you
add apps/scenes — it should never require a code change.

### 2026-06-21 — Config-driven UI, always
The UI renders entirely from `apps.config.json`. Never hardcode an app/scene in
JS. This is what keeps routine changes a 10-second JSON edit instead of an AI
session (token-budget goal).

### 2026-06-21 — Known limitations accepted (documented)
PowerToys dependency, no official launch-by-name CLI (use generated .lnk),
launches fresh instances, absolute paths not portable, multi-monitor fragility,
admin apps, WebView2 requirement, hotkey collisions. See docs/SETUP.md "Caveats".

### 2026-06-21 — Phase 1: summon hotkey is Ctrl+Alt+Space
The Win (Super) key is heavily reserved by Windows and registration is flaky, so
the default summon hotkey is **Ctrl+Alt+Space**. Configurable later; registered
Rust-side in `src-tauri/src/main.rs`.

### 2026-06-21 — Phase 1: elevated apps via Start-Process -Verb RunAs
A tile may set `"elevated": true`. Fay (running un-elevated) launches it through
`powershell Start-Process -Verb RunAs`, which raises a normal UAC prompt. Fay
itself stays un-elevated so normal launches and drag/UX aren't affected. The
no-prompt alternative (a scheduled task set to highest privileges) is documented
in SETUP.md for users who want it.

### 2026-06-21 — Phase 1: multi-monitor = visibility, not control
Fay does not move windows. To address monitor fragility it exposes a
`list_monitors` command and shows the live layout in the footer, so a scene that
lands off-screen is immediately explainable ("you have 1 display now, scene was
saved for 2"). Actual positioning stays with PowerToys.

### 2026-06-21 — Phase 1: window hides on focus loss
Launcher behavior — the Fay window hides when it loses focus and is summoned
again via hotkey/tray. Escape also hides it.

### 2026-06-21 — Phase 2: real deck wired in
`apps.config.json` is now filled with the owner's actual apps (Zen, Discord,
Claude, LifeOS, Phone Link, Steam, Task Manager) and three scenes (Focus, Game,
Side Stack). Protocol-handler targets (`ms-phone:`, `steam://`) and `taskmgr`
work out of the box; exe-path tiles carry a `hint` to verify the install path.

### 2026-06-21 — Mouse-button summon (Ctrl+Mouse5) deferred
Requested, but `tauri-plugin-global-shortcut` is keyboard-only — it cannot bind
mouse buttons. A global mouse-button trigger requires a low-level Windows mouse
hook (WH_MOUSE_LL / raw input) running beside the app. Deferred as a future
enhancement; summon stays **Ctrl+Alt+Space** for now.

### 2026-06-21 — Phase 2: UI polish
Tiles get a staggered entrance animation (honors `prefers-reduced-motion`),
arrow-key navigation across the grid, and a focus-visible ring matching hover.

### 2026-06-21 — Phase 3: scene audio output via SoundVolumeView
Scenes may set the default playback device (`audioOut` field). Windows has no
native CLI, so we shell out to NirSoft `SoundVolumeView.exe` (portable, no
install). Fay sets only the **Console (0)** + **Multimedia (1)** default roles
and deliberately leaves **Communications (2)** alone, so changing the "main"
output does not move Discord. This keeps Fay's own code tiny and stays in scope
(we trigger a tool, we don't write an audio engine). SoundVolumeView is a runtime
dependency the user installs, documented in SETUP.md.

### 2026-06-21 — Phase 4: summon hotkey is config-driven
The summon hotkey now comes from `app.hotkey` in `apps.config.json`. The frontend
passes it to a `set_summon_hotkey` command that parses the accelerator
(`Shortcut::from_str`) and re-registers it; `Ctrl+Alt+Space` stays the built-in
default/fallback. Keeps with the "routine changes are JSON edits" rule. Still
keyboard-only (mouse buttons need the deferred mouse hook).

### 2026-06-21 — Communication: requests rendered distinctly
Per owner preference, chat replies must visually separate **action items /
requests directed at the user** from explanation and status text. Convention:
put asks in a blockquote led by 📌, never buried in prose. Owner tasks are also
tracked as a checklist in STATE.md.

### 2026-06-21 — Communication convention revised (supersedes above)
Owner finds shaded blockquotes hard to read. Requests/action items should be
visually **structured** instead — a dedicated heading plus a numbered list or
table, not a blockquote. Keep them separate from explanation/status.

### 2026-06-21 — Phase 5: autostart at login
Launch-at-login via `tauri-plugin-autostart`, toggled by `app.autostart` in
`apps.config.json` (default `false`). A `set_autostart` command flips it through
the plugin's Rust `ManagerExt` (so no extra capability permission needed). Stays
config-driven and in scope.

### 2026-06-21 — Communication convention: boxed panel (supersedes above)
Owner wants any request, question, or expected instruction rendered as a
**highlighted boxed panel** — a bordered Markdown table with a titled header row
(🔔) — for maximum visibility. This is the current convention.

### 2026-06-23 — Phase 6: the Heart (JARVIS/bubble HUD)
Owner-approved redesign into a centered, organic "Heart": a full-monitor
translucent overlay (wallpaper shows through, `backdrop` alpha) with a `<canvas>`
particle field — layered multi-speed rings + inner set + a pulsing particle
sphere. The sphere is the open/close button (rest ↔ tiles bloom). Built in plain
canvas (no libs) for a light footprint; rendering pauses while hidden. Accent is
config-driven with `accent: "auto"` matching the live Windows accent color
(`get_accent_color`), `backdrop` controls see-through. Window is now
alwaysOnTop + skipTaskbar + transparent, sized to the active monitor on show.
Mouse-button summon (Mouse5) still parked. Tile grid kept (centered chips) for
usability rather than fully orbital.

### 2026-09-18 — Phase 7: CI-built installer as the primary test path
The owner never ran Fay because it needed Rust + Node. CI now builds an NSIS
installer (`tauri build --bundles nsis`) on every push/PR and uploads it as the
`Fay-windows-installer` artifact, so each change proves it still builds before
merge. Installing
that artifact is the documented first-choice way to try Fay; source builds are
for development only. NSIS only (no MSI) to keep the job fast and avoid the WiX
download.

### 2026-09-18 — Phase 8: direct scene/app hotkeys
Any tile may carry a `hotkey`; pressing it fires the tile (target + `audioOut`)
without opening Fay. All global shortcuts now live in one `HotkeyState` (summon
+ item bindings) and are re-registered together via `apply_hotkeys`, so
changing the summon combo no longer wipes item hotkeys (the old
`set_summon_hotkey` unregistered everything). The global handler matches the
pressed shortcut against the bindings first and falls back to toggling the
window. Conflicting combos are skipped, unparseable ones reported. Default:
scenes on `Ctrl+Alt+1/2/3`.

### 2026-09-18 — Phase 9: installed-build readiness
Three fixes so the installer build is actually usable day-to-day:
1. **Editable config.** The installer bakes `src/apps.config.json` into the exe,
   which froze the deck — the opposite of "routine changes are JSON edits". On
   first run the bundled default is seeded to `%APPDATA%\com.fay.hub\apps.config.json`
   (`load_config`/`save_config`) and loaded from there; tray gets *Open config
   file* (Notepad, always editable) and *Reload config* (`location.reload()`).
   Supersedes the earlier "config lives in src/" reasoning for installed builds.
2. **Start hidden.** Window is `visible: false`; the frontend shows it on first
   run or when `app.startHidden` is `false`. Avoids a full-screen overlay at
   login with autostart.
3. **Visible failures.** A `warn()` status (amber, 7s) in the footer surfaces
   hotkey-bind failures, missing SoundVolumeView, config typos, etc. On a config
   error the window is shown so the message can be seen.

### 2026-09-18 — Phase 10: number-key launch + type-to-filter
With the deck open, digits 1–9 fire the Nth visible tile (tiles are numbered
inline before the name) and typing filters tiles by name (substring, case-
insensitive) with Enter firing the first match; Esc clears the filter before it
closes the deck. Frontend-only. Digits are excluded from the filter text so the
two never conflict. Keeps the deck keyboard-fast as it grows, ahead of a full
search/categories feature.

### 2026-09-18 — Phase 11: GitHub Releases on tag
A `release.yml` workflow runs on `v*` tags: builds the NSIS installer and
publishes a GitHub Release (softprops/action-gh-release, auto-generated notes)
with the `.exe` attached. Gives the owner a clean, permanent download page
instead of Actions artifacts (which expire). Per-push artifacts stay for testing
in-between. Workflow needs `contents: write`. Versions come from the existing
`0.1.0` in tauri.conf/Cargo/package; first tag is `v0.1.0`.

### 2026-09-18 — Phase 12: real app icons
Tiles swap their glyph for the target's actual icon. A `get_app_icon` command
resolves the target (env vars, `.lnk` via WScript.Shell, PATH commands via
`Get-Command`), extracts it with `System.Drawing.Icon::ExtractAssociatedIcon`
in PowerShell, and returns a PNG data URL; results are cached as PNGs in the app
cache dir keyed by a hash of the target. Protocol targets have no icon and keep
the glyph. An explicit `icon` on a tile overrides. Base64 is hand-rolled (~40
lines) rather than adding a crate for one use.

### 2026-09-18 — Phase 13: running-app indicator
A `list_running` command returns running process names (PowerShell
`Get-Process`, lowercase, unique). The frontend derives an expected process name
per tile from its target (basename without extension; protocol scheme for
`steam://`-style targets) with an explicit `process` override, and marks matches
with a glowing dot. Polled only when the deck opens and every 5s while open, so
it costs nothing at rest. Heuristic by design — the override covers the misses.

### 2026-09-18 — Phase 11 revised: releases on version bump, not tags
Tag pushes are rejected by the session's git integration (only the designated
branch is accepted), and `workflow_dispatch` is unavailable to the integration
token, so a tag-triggered release could never be fired from here. `release.yml`
now runs on pushes to `main`: a `check` job reads `version` from package.json
and asks `gh release view v<version>`; if absent, the `release` job builds the
installer and publishes the release, creating the tag itself
(`softprops/action-gh-release` with `tag_name` + `target_commitish`). Shipping is
therefore "bump the version, merge to main". Supersedes the tag-trigger entry.

### 2026-09-18 — Phase 14: per-machine profiles
Absolute paths differ per PC, so the config gains a `machines` block keyed by
computer name (`get_hostname` → `COMPUTERNAME`, uppercase; matched
case-insensitively). `machines.<name>.tiles.<id>` is shallow-merged onto the
tile with that id before rendering, hotkey registration, etc. Any field can be
overridden. The host name is shown in the footer so it's obvious which profile
applies. Kept as an overlay rather than separate config files so the deck stays
one document.

### 2026-09-18 — Debug round (static review, all phases)
Fixed before anyone ran the app: (1) every `cmd`/`powershell` child flashed a
console window — all child processes now go through a `cmd()` helper that sets
`CREATE_NO_WINDOW`; (2) Tauri runs sync commands on the main thread, so icon
extraction, the running-process check, the accent read and the audio switch
froze the Heart — those commands are now `async`, and hotkey-fired scene
actions run on a thread; (3) the Heart animated while the window was hidden at
startup — it now starts only when the window has focus; (4) the canvas ignored
`devicePixelRatio` (blurry on HiDPI) — it renders at device resolution;
(5) if the window hid while the deck was open, the 5s process poll ran forever —
blur returns the deck to rest and the poll is gated on focus; (6) a syntax error
in the user config emptied the deck — it now falls back to the bundled default
without touching the user's file and shows the window with the warning;
(7) hotkey badge overlapped long hints.

### 2026-09-18 — Phase 15: README as the front door
README rewritten as the product page: Heart preview up top, 30-second install
from the Releases page (no toolchain), a feature table, the config with a
complete key reference, scenes in three steps, and pointers into `docs/`. The
deep material stays in SETUP/ARCHITECTURE/DECISIONS/STATE; the README only has
to get someone from "what is this" to "it's running" fast.

### 2026-09-18 — Phase 16: Ctrl+Mouse5 summon via WH_MOUSE_LL
Supersedes the "deferred" entry. A `mouse_summon` module installs a low-level
mouse hook on its own thread (with the message loop LL hooks require) using
`windows-sys`. On WM_XBUTTONDOWN it compares the X button and live modifier
state (`GetAsyncKeyState`) against `app.mouseSummon` (parsed from strings like
"Ctrl+Mouse5"; Mouse4 = XBUTTON1, Mouse5 = XBUTTON2), toggles the window on the
main thread via `run_on_main_thread`, and returns 1 to swallow the click. The
combo lives in a static `Mutex` set by `set_mouse_summon`; the hook is
installed once at startup and is inert until a combo is set. Windows-only
(`cfg(windows)` dependency and module).

### 2026-09-18 — Phase 17 (batch 1): tile kinds, packs, quicklinks, calculator
Nine small features in one PR, grouped because they share one change: a tile
is no longer just a `target`. Every tile now resolves to a `TileAction`
(`kind` = launch | system | media | snippet) that both the click path (`fire`)
and direct hotkeys go through, so nothing can behave differently between the
two. Decisions: (1) `system` actions are a **fixed table** in Rust — the config
picks a name, never a command string, so a shared config can't run arbitrary
shell; (2) shutdown / restart / logoff / hibernate need a **second press within
3 s** by default (`confirm`), because number keys make a mis-fire too easy;
(3) media keys and the snippet paste use `SendInput` with the real virtual
keys, and the clipboard is written natively (no PowerShell start-up lag);
(4) the paste waits for the user's modifiers to be released so a snippet
hotkey doesn't turn Ctrl+V into Ctrl+Alt+V; (5) presets live in
`src/packs.json` and are opted into with `packs: [...]`, appended to a third
group `system`, and a pack tile yields to a user tile with the same id;
(6) quicklinks are any `target` containing `{query}` plus a `keyword`
(default: id) — typed as `kw query`; (7) the calculator / converter is a small
hand-written parser (no `eval`) and lines that start with a digit need a `=`
prefix because bare digits fire tiles; currency is deliberately left out
(needs network); (8) structural config validation runs on every load and, like
a syntax error (now reported with line/col), shows the window so the message
is seen; (9) the overlay opens on the monitor **under the cursor** by default
(`summonOn`), using `cursor_position` + `monitor_from_point`.

### 2026-09-18 — Phase 18 (batch 2): folders, multi-action, teardown, commands folder
Folders are **frontend-only**: a tile with `children` swaps the three groups
for the folder's grid plus a breadcrumb; Esc/Backspace goes up; number keys,
typing and Enter scope to the visible container. Children are ordinary tiles,
so their hotkeys and quicklink keywords work from anywhere (all tile walks are
now recursive). Multi-action tiles reuse `TileAction` recursively (`steps`,
plus `wait` pauses) so a sequence can mix every kind and run from a hotkey;
they stop at the first failing step. Scene teardown is a `closes` list of
process names run through graceful `taskkill /IM` (WM_CLOSE) with a `!` suffix
to force — chosen over "second press closes" because an active scene isn't
detectable and a mis-press would kill apps; Shift+click / Shift+digit / a
hover × / `closeHotkey` are the explicit triggers, and "not running" is a
footer note rather than an error. The commands folder is
`<config dir>\commands` (created on first run, tray item to open it); files
become tiles at load, and `.ps1` targets now run via `powershell -File` with
Bypass because the .ps1 shell association is "edit". Rust gains `expand_env`
for paths it hands to PowerShell (cmd.exe expanded `%VAR%` for us before).

### 2026-09-18 — Phase 19 (batch 3): live stats + focus timer on the Heart
Stats come from the `sysinfo` crate kept in Tauri state (CPU and network are
deltas between polls, so the first sample is empty) rather than PowerShell
counters, which cost a ~1 s sample each; GPU is `nvidia-smi` when present and
simply absent otherwise. Polled every 2.5 s only while Fay is in front, drawn
by the Heart at rest and hidden behind the open deck, and rendered as text
plus **dotted gauges** — no solid lines, per the Heart's earlier design
decision. The focus timer is frontend state so it survives the window being
hidden (JS timers keep running); the Heart draws a dotted progress arc just
outside the main ring, the remaining time sits in the footer, and completion
pulses the Heart, raises a Windows toast (via PowerShell + WinRT using
PowerShell's own AppUserModelId, since Fay isn't registered for toasts) and
calls `window.__faySay` if voice is present. A `focus` hotkey reaches the
frontend through `eval` from the backend (`APP` handle), the one action that
doesn't run in Rust.

### 2026-09-18 — Phase 20 (batch 4): clipboard history, Everything search, bookmarks
One "results mode" serves all three: a prefix (`>` files, `@` bookmarks) or a
clipboard toggle replaces the tile grid with rows from an async provider;
arrows + Enter pick, Shift+Enter is the row's second action. Clipboard history
is **memory only** — never written to disk, cleared on quit — polled via
`GetClipboardSequenceNumber` (cheap, no clipboard open) every 400 ms, and it
honours the `ExcludeClipboardContentFromMonitorProcessing` format password
managers set. Picking an entry re-sets the clipboard and pastes through the
same path as snippets. File search shells out to Everything's `es.exe` (no
indexing of our own — an explicit non-goal), sorted by date modified.
Bookmarks avoid SQLite (Zen/Firefox `places.sqlite` is locked and would need
rusqlite): Firefox-family browsers are read from their daily `jsonlz4`
backups (mozlz4 = 8-byte magic + LE size + LZ4 block, decoded with
`lz4_flex`), Chromium browsers from the plain `Bookmarks` JSON; cached five
minutes. The one-day lag on Firefox-family bookmarks is accepted and
documented.

### 2026-09-18 — Phase 21 (batch 5): voice via Windows System.Speech
Chosen over Whisper / cloud TTS because it needs nothing installed, works
offline, and starts instantly: a single persistent PowerShell process hosts
both the `SpeechSynthesizer` and the `SpeechRecognitionEngine` (engine
start-up is ~1 s, so it is paid once, not per command), driven by Rust over a
line protocol on stdin with results forwarded to the frontend by eval.
Recognition uses a **fixed grammar** of tile names (+ open/start/launch,
close <scene>, a few built-ins) rather than dictation, which makes it reliable
on ordinary mics; it is **tap-to-talk** (one six-second `Recognize` per
press), never always-on — a privacy and CPU decision. Replies are opt-in per
tile (`say`) plus the focus-timer announcement and a spoken "Opening …"
acknowledgement that `voiceConfirm: false` silences. The Heart wobbles while
speaking and breathes while listening via `talk()` / `listen()`. Upgrade path
if wanted later: swap the recognizer for Whisper behind the same protocol.

### 2026-09-18 — Phase 22 (batch 6): AI bar, screen-aware ask, agentic loop, add-app picker
The model is a **planner, not an executor**: it receives the deck as JSON and
may only return a bounded vocabulary of action objects (tile ids, close,
system/media/focus, wait, say, open), which the frontend maps onto the same
`TileAction` path every click uses — so nothing the model says can reach a
shell that a config tile couldn't. Destructive actions are gated on the
user's own last message containing an explicit ask/confirmation, independent
of what the model claims; the voice grammar likewise omits shutdown-class
tiles. "Agentic" means the failure of a step is reported back and the model
may re-plan up to two more times — no open-ended loops. Triggering is
low-friction: Enter on a line that matches no tile asks the model, `?` asks
explicitly, and conversation memory lives only while the deck is open.
Providers: Anthropic (key from the local config or `ANTHROPIC_API_KEY`; the
bundled default config ships an empty key) or Ollama for fully local use.
Screen awareness is strictly on demand: the foreground window is captured
before Fay appears, downscaled, held in memory, attached to one message, and
never written to disk — an explicit rejection of the always-watching
overlays. HTTP is `ureq` (blocking, in an async command) rather than reqwest
to keep the binary small. The add-app picker is a WinForms dialog from STA
PowerShell (no dialog plugin/capability needed) and writes the user's config
back re-serialized with 2-space indentation.

### 2026-09-18 — Debug round 2 (after batches 1–6)
Static review of the whole tree: (1) the ask flow cleared its own "thinking…"
panel because the filter was reset after the request started — order fixed,
and typing after an answer now continues the conversation; (2) the Anthropic
API requires alternating roles starting with `user` — history is normalized
(neighbouring same-role turns merged, leading assistant turn dropped);
(3) AI-planned and voice-triggered shutdown/restart bypassed the press-twice
confirm — now gated (AI) or excluded (voice); (4) pack tiles could collide
with folder children ids — dedupe is now deep; (5) every multi-line
PowerShell script is passed as `-EncodedCommand` (UTF-16LE base64) instead of
`-Command`, removing the `"`/`$`/backtick quoting risk that the toast, voice,
capture and picker scripts carried; `-STA` goes before it for the dialog.

### 2026-09-18 — Debug round 3: the first real render of the Heart
Rendering `src/` in headless Chromium (`docs/render-preview.js`, uses the
Playwright Chromium bundled in the dev environment) replaced the SVG mockup
and exposed four things static review missed: (1) the rings were drawn at a
quarter of the approved size — `rFrac` values were fractions of a radius that
was itself already 34 % of the screen — so the whole Heart was a small blob;
radii now follow the mockup (main band ≈ R, inner ≈ 0.7 R, sphere 0.17 R,
focus arc 1.12 R, stats at 1.22 R); (2) `accent: "auto"` outside Tauri (and
on any failed registry read) parsed as `0x0a` → near-black; accent is now
validated as a hex color in both `app.js` and `heart.js`; (3) the starter
config had a duplicate id (`focus` scene vs `focus` timer tile); (4) long
hints made tiles tall and the deck overflowed a 900 px screen — hints clamp
to two lines (full on hover), the deck scrolls within the viewport, and
groups whose tiles are all filtered out hide their label. `docs/ui-preview.png`
is now the real render, not a mockup. Lesson recorded: a screenshot of the
real frontend is cheap here and should be part of every UI-touching PR.

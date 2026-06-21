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

### 2026-06-21 — Communication: requests rendered distinctly
Per owner preference, chat replies must visually separate **action items /
requests directed at the user** from explanation and status text. Convention:
put asks in a blockquote led by 📌, never buried in prose. Owner tasks are also
tracked as a checklist in STATE.md.

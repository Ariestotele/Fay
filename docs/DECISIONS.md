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

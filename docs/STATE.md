# State

> Update this at the end of every session. New chats read this first.

**Last updated:** 2026-09-18
**Current phase:** **The whole researched pipeline is built — Phases 1–22.**
The "do them all" run shipped six batches (PRs #18–#22): quick wins, deck
structure, Heart extras, search, voice, AI + picker, plus a second static
debug round. Everything compiles on Windows CI; **none of it has been run by
a human yet** — the next thing that matters is the owner installing the
newest CI artifact and reporting what actually works. Expect a fix-up round.
v0.1.0 on Releases is far behind `main`; bump `package.json` to 0.2.0 to
publish everything as a Release.

## ✅ Done (all merged)

| Phase | What |
| :-- | :-- |
| 0 | Repo scaffold, `.md` context system, locked stack (DECISIONS.md) |
| 1 | Tray icon, summon hotkey, hide-on-blur/Esc, elevated launch (UAC), `list_monitors` readout, icons |
| 2 | Real deck (Zen, Discord, Claude, LifeOS, Phone Link, Steam, Task Manager; scenes Focus/Game/Side Stack), UI polish |
| 3 | Per-scene audio output via `audioOut` (SoundVolumeView; Console+Multimedia roles only — Discord untouched) |
| 4 | Config-driven summon hotkey (`app.hotkey`, `set_summon_hotkey`) |
| 5 | Autostart at login (`app.autostart`, tauri-plugin-autostart) |
| 6 | **The Heart** — full-monitor translucent overlay, `<canvas>` particle HUD (layered multi-speed rings + inner set + pulsing particle sphere as the open button), `accent: "auto"` (Windows accent via `get_accent_color`), `backdrop` alpha |
| 7 | CI-built NSIS installer artifact on every push/PR (`Fay-windows-installer`) — test without a toolchain |
| 8 | Direct scene/app hotkeys — per-tile `hotkey` fires the tile (with `audioOut`) without opening Fay; unified `HotkeyState` |
| 9 | Installed-build readiness — user-editable config seeded to `%APPDATA%\com.fay.hub\apps.config.json` (tray: Open config / Reload), starts hidden (`startHidden`), amber `warn()` surfacing in the footer (in PR) |
| 10 | Number-key launch (1–9) + type-to-filter with Enter-to-fire, Esc clears; frontend only (in PR) |
| 11 | Release workflow — on push to `main`, publishes `v<version>` (from package.json) on the GitHub Releases page if not yet released; creates the tag itself (tag pushes aren't possible from the agent) |
| 12 | Real app icons — `get_app_icon` extracts + caches each target's icon; glyph fallback; `icon` override (in PR) |
| 13 | Running-app indicator — `list_running` + per-tile process-name guess (`process` override); glowing dot; polled only while open (in PR) |
| 14 | Per-machine profiles — `machines.<HOST>.tiles.<id>` overlay merged onto tiles; `get_hostname`; host shown in footer (next PR) |
| — | **Debug round** — no console flashes (`CREATE_NO_WINDOW`), blocking commands made `async` (no UI freeze), Heart idle while hidden, HiDPI-crisp canvas, no runaway polling, broken-config fallback (in PR) |
| 15 | README rewritten as the product front door — preview, 30-second install, feature table, full config reference |
| 16 | Ctrl+Mouse5 summon — `app.mouseSummon`, WH_MOUSE_LL hook thread (`windows-sys`), swallows the click; `set_mouse_summon` |
| 17 | **Batch 1 / quick wins** — tile `kind`s (system / media / snippet) via one `TileAction` + `fire`; packs (`power media tools settings links`) into a `system` group; quicklinks (`{query}` + `keyword`); calculator + unit conversion on the filter line (`= …`, Enter copies); config validation with line/col; `summonOn: "cursor"`; press-twice confirm for destructive actions |
| 18 | **Batch 2 / deck structure** — folders (`children`, sub-deck + breadcrumb, Esc/Backspace up); multi-action tiles (`actions` steps + `wait`); scene teardown (`closes`, Shift+click / × / `closeHotkey`, `!` = force); commands folder (`commands`, tray › Open commands folder, `.ps1` via `powershell -File`) |
| 19 | **Batch 3 / Heart extras** — live CPU · RAM · GPU · NET readouts with dotted gauges around the rings (`get_stats`, sysinfo + nvidia-smi, `app.stats`); focus timer (`kind: "focus"`, `minutes`, dotted progress arc, footer countdown, toast + `Heart.pulse()` on done, `focus` pack) |
| 20 | **Batch 4 / search** — results mode (rows replace the grid): `>` files via Everything `es.exe` (`app.everything`), `@` browser bookmarks (Zen/Firefox jsonlz4 + Chromium JSON, `lz4_flex`), clipboard history (memory only, `kind: "clipboard"` tile + `Ctrl+Alt+V`, Enter pastes) (in PR) |
| 21 | **Batch 5 / voice** — persistent `System.Speech` PowerShell process (`voice_config`, `say`, `listen`, `set_voice_grammar`); per-tile `say`, `kind: "say"` / `"listen"` tiles; grammar from tile names; `Heart.talk/listen`; `app.voice*` |
| 22 | **Batch 6 / AI + picker** — natural-language bar (Enter on no-match or `?`), JSON plan → `TileAction` multi, bounded retry loop, destructive gate; screen-aware ask (`kind: "ask"`, `Ctrl+Alt+A`, capture-before-show); Anthropic or Ollama (`app.ai`); `+ Add app` picker (`pick_file`, `addTile`); debug round 2 (`-EncodedCommand`, history normalization, confirm gates) (in PR) |

## ⚙️ CI (`.github/workflows/ci.yml`)

- **config** (Ubuntu) — JSON validation + `node -c` on `app.js` / `heart.js`.
- **build-check** (Windows) — `cargo check` compiles the Tauri app on every push/PR.
- **installer** (Windows) — `tauri build --bundles nsis` → uploads
  `Fay-windows-installer` artifact on every push/PR. Download it from the
  Actions run → no toolchain needed to test.

## 📌 Pending — owner tasks (can't be automated)

- [ ] **Run Fay** — install **v0.1.0 from the Releases page** (or any newer CI
      artifact) and give feedback on the Heart — density, pulse, backdrop
      dimness, ball size, tile layout — and whether Ctrl+Mouse5 works.
- [ ] Verify the exe paths for **Zen / Claude / LifeOS** — edit via tray › *Open
      config file* (`%APPDATA%\com.fay.hub\apps.config.json`), then *Reload config*.
- [ ] Create the three **PowerToys Workspaces** + desktop shortcuts
      (`Fay-Focus.lnk`, `Fay-Game.lnk`, `Fay-Side.lnk`). See SETUP.md.
- [ ] Install **SoundVolumeView.exe** (PATH or beside Fay) for scene `audioOut`.
- [ ] Install **Everything** + its `es.exe` CLI (PATH, or `app.everything`) for `>` file search.
- [ ] Try voice: press `Ctrl+Alt+L`, say *"game"*. If the mic isn't picked up, check Windows › Privacy › Microphone.
- [ ] For AI: put your Anthropic key in `app.ai.apiKey` in the **local** config (tray › Open config file), or run Ollama and set `"provider": "ollama"`. Then type *"game mode but keep audio on speakers"* and press Enter; try `Ctrl+Alt+A` on an error dialog.
- [ ] Decide on code signing (#20): a certificate costs money; without it SmartScreen shows "More info → Run anyway" once per install.
- [ ] Runtime checks: tray appears, Ctrl+Alt+Space toggles, a UAC tile prompts,
      footer shows real monitor count, accent matches Windows.

## 🚀 Pipeline — the "do them all" queue

Researched 2026-09-18 from Raycast / Flow Launcher / PowerToys Command Palette /
Keypirinha / Listary / ueli / Wox, Stream Deck-style macro software, and the
"AI Jarvis" launcher wave (Raycast AI, Open.Jarvis, Cluely/Highlight). Effort:
S = frontend/config only, M = a Rust command or two, L = new subsystem.
**Owner said "do them all"** — batches: 1 = #1 2 3 8 10 12 16 17 18 (✅ Phase 17);
2 = #4 5 6 11 (✅ Phase 18); 3 = #13 14 (✅ Phase 19); 4 = #7 9 15 (✅ Phase 20);
5 = C B (✅ Phase 21); 6 = A D E + #19 (✅ Phase 22). **All done except #20**,
which needs a purchased code-signing certificate (owner decision). Currency
conversion (#16, network part) is still out — the AI bar answers those.

### AI — "talk to Fay" (the viral category)

| # | Feature | What it feels like | How | Effort |
| :-- | :-- | :-- | :-- | :-- |
| A | **Natural-language bar** | Type *"game mode but keep audio on speakers"* → the right tiles fire with overrides. | Tile list sent as tools to an LLM (Claude API key from config, or local Ollama); model returns tiles/commands to run. | M |
| B | **Push-to-talk voice** | Hold a key/Mouse5, say *"start focus"* → Heart pulses while listening → scene fires. | Windows built-in `System.Speech` recognition via PowerShell (offline), grammar built from tile names. Upgrade path: Whisper. | M |
| C | **Spoken replies + Heart reacts** | *"Game mode ready — audio on headset."* Sphere swells while it speaks. | Windows TTS (`System.Speech`) + `Heart.pulse()` hook. | S |
| D | **Screen-aware ask** | Summon → *"what's this error?"* → reads the focused window. | Screenshot active window → vision model. Opt-in, explicit hotkey, never always-on. | M |
| E | **Agentic tasks** | One request → multi-step plan with retries. | Built on A; let the model chain tools. After A is solid. | M–L |

### Classic launcher features

| # | Feature | What it feels like | How | Effort |
| :-- | :-- | :-- | :-- | :-- |
| 1 | **System-command tiles** | Sleep, Lock, Restart, Shutdown, Empty Recycle Bin, Toggle dark mode as tiles. | `kind: "system"` + fixed command table in Rust. | S |
| 2 | **Quicklinks with `{query}`** | Type `yt lofi` → opens YouTube search. | Tile `target` with `{query}` placeholder; filter bar passes the rest. | S |
| 3 | **Inline calculator** | Type `1440*0.62` → answer in the footer, Enter copies. | Safe expression evaluator in JS. | S |
| 4 | **Multi-action / macro tiles** | One tile runs a sequence: audio → app → app → wait → app (Stream Deck "Multi-Action"). | `actions: [...]` array per tile, run in order with optional delays. | M |
| 5 | **Scene close / teardown** | Second press of an active scene closes its apps. | `closes: ["discord","zen"]` → `taskkill` by process name. | M |
| 6 | **Folders / pages** | Tile opens a sub-deck (Games, Work, Tools) — Stream Deck nested folders, keeps the deck ≤ 9 keys. | `children: [...]` tiles; back = Esc. Replaces "search + categories". | M |
| 7 | **Clipboard history** | `Ctrl+Alt+V` style list of last N copies, Enter pastes. | Poll clipboard in Rust; stored in memory only. | M |
| 8 | **Snippets** | Tile pastes a saved text (email, address, prompt). | `kind: "snippet"`; set clipboard + send Ctrl+V. | S |
| 9 | **Everything file search** | Type `>report` → results from voidtools Everything. | Shell out to `es.exe` if present. | M |
| 10 | **Windows Settings pack** | Tiles for Bluetooth, Sound, Display, Network, Update via `ms-settings:` URIs. | Config-only preset, opt-in block. | S |
| 11 | **Command folder** | Drop `.ps1` / `.bat` files in a folder → they become tiles. | Scan a config folder on load. | S |
| 12 | **Media controls** | Play/pause, next, mute, volume as tiles or keys. | Send media virtual keys via `SendInput`. | S |
| 13 | **Live stats in the Heart** | CPU / GPU / RAM / net readouts orbiting the rings. | Poll counters (PowerShell / `sysinfo` crate) only while shown. | M |
| 14 | **Focus timer** | Pomodoro on the Heart; ring fills as time passes. | Frontend timer + optional TTS ping. | S |
| 15 | **Browser bookmarks search** | Type a bookmark name → opens in Zen. | Read the browser's bookmarks JSON. | M |
| 16 | **Unit / currency conversion** | `12 usd to eur`, `5 mi to km`. | Extends the calculator; currency needs a fetch. | S |
| 17 | **Config validation** | Broken config → footer says *line 23: missing comma*. | JSON parse with position → line/col. | S |
| 18 | **Cursor-monitor summon** | Fay appears on the monitor under the mouse. | Use cursor position in `fill_active_monitor`. | S |
| 19 | **First-run path picker** | "Browse…" to capture an exe path into the config. | Tauri dialog plugin. | M |
| 20 | **Code signing** | No SmartScreen warning. | Needs a certificate (cost) — owner decision. | — |

**Skip (out of scope / not worth it):** full file indexing, plugin SDK,
window snapping (PowerToys does it), always-on screen capture (Cluely-style).

## 🧭 Later / parked

- Repo default branch is still `claude/charming-ramanujan-shirak`; consider
  switching it to `main` in GitHub settings.
- Bump `package.json` to 0.2.0 on `main` to publish Phases 14–16 as a Release
  (currently only CI artifacts).

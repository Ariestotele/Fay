// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::Shortcut;

#[derive(serde::Serialize)]
struct MonitorInfo {
    name: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: f64,
}

/// A global hotkey bound directly to a tile (scene or app): pressing it fires
/// the tile's launch without opening Fay.
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HotkeyBinding {
    accelerator: String,
    target: String,
    #[serde(default)]
    elevated: bool,
    #[serde(default)]
    audio_out: Option<String>,
}

/// All registered global shortcuts: the summon combo plus per-tile bindings.
#[derive(Default)]
struct Hotkeys {
    summon: Option<Shortcut>,
    items: Vec<(Shortcut, HotkeyBinding)>,
}
type HotkeyState = std::sync::Mutex<Hotkeys>;

/// Build a child-process command that never flashes a console window. Fay is a
/// GUI app; without CREATE_NO_WINDOW every `cmd`/`powershell` call would pop a
/// black console for a moment.
#[allow(unused_mut)]
fn cmd(program: &str) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Resize/move the window to fill the monitor it's currently on (HUD overlay).
fn fill_active_monitor(window: &WebviewWindow) {
    if let Ok(Some(m)) = window.current_monitor() {
        let p = m.position();
        let s = m.size();
        let _ = window.set_position(tauri::PhysicalPosition { x: p.x, y: p.y });
        let _ = window.set_size(tauri::PhysicalSize { width: s.width, height: s.height });
    }
}

/// Show the window if hidden, hide it if visible. Used by the hotkey + tray.
fn toggle_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        fill_active_monitor(window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Launch a target: an exe, a command on PATH, a file path, a .lnk, or a URL.
///
/// - Normal:   `cmd /C start "" <target>` handles exe / command / .lnk / URL.
/// - Elevated: `Start-Process -Verb RunAs` triggers a UAC prompt and runs the
///   target with admin rights. This is how Fay launches elevated apps without
///   itself running elevated.
fn do_launch(target: &str, elevated: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if elevated {
            // Single-quote escaping for PowerShell ('' is a literal quote).
            let safe = target.replace('\'', "''");
            cmd("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    &format!("Start-Process -Verb RunAs -FilePath '{safe}'"),
                ])
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            cmd("cmd")
                .args(["/C", "start", "", target])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (target, elevated);
        Err("launch() is only implemented on Windows".into())
    }
}

#[tauri::command]
fn launch(target: String, elevated: Option<bool>) -> Result<(), String> {
    do_launch(&target, elevated.unwrap_or(false))
}

/// Report the current monitor layout so the UI can show it and flag when a
/// saved scene may no longer match the physical setup.
#[tauri::command]
fn list_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("no main window")?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            MonitorInfo {
                name: m.name().cloned().unwrap_or_else(|| "display".into()),
                width: size.width,
                height: size.height,
                x: pos.x,
                y: pos.y,
                scale: m.scale_factor(),
            }
        })
        .collect())
}

/// Let the frontend hide the window (e.g. on Escape).
#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// Set the system's default playback device by name, for the Console (0) and
/// Multimedia (1) roles only — leaving the Communications (2) role untouched.
///
/// That split is deliberate: games/media follow Console/Multimedia, so they move
/// to the new device, while chat apps that follow the Communications default
/// (e.g. Discord on "Default") are NOT moved. Discord pinned to a specific
/// device is likewise unaffected.
///
/// Requires NirSoft SoundVolumeView.exe on PATH or beside Fay (Windows has no
/// built-in CLI for this). See docs/SETUP.md.
fn do_set_audio_output(device: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        for role in ["0", "1"] {
            let status = cmd("SoundVolumeView.exe")
                .args(["/SetDefault", device, role])
                .status()
                .map_err(|e| format!("SoundVolumeView.exe not found: {e}"))?;
            if !status.success() {
                return Err(format!("SoundVolumeView exited with {status}"));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = device;
        Err("audio switching is only implemented on Windows".into())
    }
}

// The commands below shell out and block; they are `async` so Tauri runs them
// off the main thread instead of freezing the UI/animation.
#[tauri::command]
async fn set_audio_output(device: String) -> Result<(), String> {
    do_set_audio_output(&device)
}

/// (Re)register every global shortcut from state: the summon combo + item
/// bindings. Item conflicts are skipped rather than failing the whole set.
fn apply_hotkeys(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let state = app.state::<HotkeyState>();
    let hk = state.lock().map_err(|e| e.to_string())?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Some(s) = hk.summon {
        gs.register(s).map_err(|e| e.to_string())?;
    }
    for (s, _) in &hk.items {
        let _ = gs.register(*s);
    }
    Ok(())
}

/// Re-register the global summon hotkey from a config accelerator string, e.g.
/// "Ctrl+Alt+Space" or "CmdOrCtrl+Shift+Space". Keyboard combos only — mouse
/// buttons aren't supported by the global-shortcut system (see DECISIONS.md).
#[tauri::command]
fn set_summon_hotkey(app: tauri::AppHandle, accelerator: String) -> Result<(), String> {
    use std::str::FromStr;
    let shortcut =
        Shortcut::from_str(&accelerator).map_err(|e| format!("bad hotkey '{accelerator}': {e}"))?;
    {
        let state = app.state::<HotkeyState>();
        state.lock().map_err(|e| e.to_string())?.summon = Some(shortcut);
    }
    apply_hotkeys(&app)
}

/// Register direct hotkeys for tiles (e.g. Ctrl+Alt+1 → the Game scene).
/// Returns the accelerators that could not be parsed (they are skipped).
#[tauri::command]
fn register_item_hotkeys(
    app: tauri::AppHandle,
    bindings: Vec<HotkeyBinding>,
) -> Result<Vec<String>, String> {
    use std::str::FromStr;
    let mut items = Vec::new();
    let mut bad = Vec::new();
    for b in bindings {
        match Shortcut::from_str(&b.accelerator) {
            Ok(s) => items.push((s, b)),
            Err(_) => bad.push(b.accelerator.clone()),
        }
    }
    {
        let state = app.state::<HotkeyState>();
        state.lock().map_err(|e| e.to_string())?.items = items;
    }
    apply_hotkeys(&app)?;
    Ok(bad)
}

/// Enable or disable launching Fay at login (config: `app.autostart`).
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

/// Read the Windows accent color as `#rrggbb` so the UI can match the system theme.
#[tauri::command]
async fn get_accent_color() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let ps = r#"$c=(Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\DWM' -Name AccentColor -ErrorAction Stop).AccentColor; '#{0:x2}{1:x2}{2:x2}' -f ($c -band 255),(($c -shr 8) -band 255),(($c -shr 16) -band 255)"#;
        let out = cmd("powershell")
            .args(["-NoProfile", "-Command", ps])
            .output()
            .map_err(|e| e.to_string())?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.len() >= 7 {
            Ok(s)
        } else {
            Err("could not read accent color".into())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("accent color is only available on Windows".into())
    }
}

/// The user-editable config lives in the app config dir (e.g.
/// `%APPDATA%\com.fay.hub\apps.config.json`), seeded from the bundled default on
/// first run — so the installed build can be customized without rebuilding.
fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("apps.config.json"))
}

/// Read the user config. `None` means it doesn't exist yet (the frontend seeds it).
#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let p = config_path(&app)?;
    match std::fs::read_to_string(&p) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", p.display())),
    }
}

/// Write the user config (used to seed it on first run). Returns the path.
#[tauri::command]
fn save_config(app: tauri::AppHandle, text: String) -> Result<String, String> {
    let p = config_path(&app)?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(p.display().to_string())
}

#[tauri::command]
fn config_file_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(config_path(&app)?.display().to_string())
}

/// Open the user config in Notepad (always editable, unlike the .json default
/// handler which is often a browser).
fn open_config_in_editor(app: &tauri::AppHandle) {
    if let Ok(p) = config_path(app) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("notepad").arg(&p).spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = p;
        }
    }
}

/// Show the window (used when the config says not to start hidden, or on first run).
#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        fill_active_monitor(&w);
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Extract the real icon for a tile's target and return it as a PNG data URL.
///
/// Resolves env vars, `.lnk` shortcuts (via WScript.Shell) and bare commands on
/// PATH (`Get-Command`), then uses `Icon::ExtractAssociatedIcon`. Protocol
/// targets (`steam://`, `ms-phone:`) have no icon and return Err so the UI
/// keeps the glyph. Results are cached as PNG files in the app cache dir.
#[tauri::command]
async fn get_app_icon(app: tauri::AppHandle, target: String) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // cache key: a cheap stable hash of the target string
        let mut h: u64 = 1469598103934665603;
        for b in target.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("icons");
        let cached = cache_dir.join(format!("{h:016x}.png"));
        if let Ok(bytes) = std::fs::read(&cached) {
            return Ok(format!("data:image/png;base64,{}", b64(&bytes)));
        }

        let safe = target.replace('\'', "''");
        let ps = format!(
            r#"$t=[Environment]::ExpandEnvironmentVariables('{safe}')
if ($t -match '^[a-zA-Z][a-zA-Z0-9+.-]*:' -and -not (Test-Path -LiteralPath $t)) {{ exit 2 }}
if ($t -like '*.lnk' -and (Test-Path -LiteralPath $t)) {{ $s=(New-Object -ComObject WScript.Shell).CreateShortcut($t); if ($s.TargetPath) {{ $t=$s.TargetPath }} }}
if (-not (Test-Path -LiteralPath $t)) {{ $c=Get-Command $t -ErrorAction SilentlyContinue; if ($c -and $c.Source) {{ $t=$c.Source }} }}
if (-not (Test-Path -LiteralPath $t)) {{ exit 3 }}
Add-Type -AssemblyName System.Drawing
$ico=[System.Drawing.Icon]::ExtractAssociatedIcon($t)
if (-not $ico) {{ exit 4 }}
$bmp=$ico.ToBitmap(); $ms=New-Object IO.MemoryStream
$bmp.Save($ms,[System.Drawing.Imaging.ImageFormat]::Png)
[Convert]::ToBase64String($ms.ToArray())"#
        );
        let out = cmd("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("no icon for {target}"));
        }
        let b64s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if b64s.is_empty() {
            return Err(format!("no icon for {target}"));
        }
        // cache it (best effort)
        if let Ok(bytes) = b64_decode(&b64s) {
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = std::fs::write(&cached, bytes);
        }
        Ok(format!("data:image/png;base64,{b64s}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, target);
        Err("icons are only available on Windows".into())
    }
}

// Tiny base64 helpers (avoid pulling in a crate for one use).
fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.len();
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        s.push(T[(v >> 18) as usize & 63] as char);
        s.push(T[(v >> 12) as usize & 63] as char);
        s.push(if n > 1 { T[(v >> 6) as usize & 63] as char } else { '=' });
        s.push(if n > 2 { T[v as usize & 63] as char } else { '=' });
    }
    s
}
fn b64_decode(s: &str) -> Result<Vec<u8>, ()> {
    fn val(c: u8) -> Result<u32, ()> {
        Ok(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(()),
        } as u32)
    }
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return Err(());
        }
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let mut v = 0u32;
        for &c in chunk {
            v = (v << 6) | if c == b'=' { 0 } else { val(c)? };
        }
        out.push((v >> 16) as u8);
        if pad < 2 {
            out.push((v >> 8) as u8);
        }
        if pad < 1 {
            out.push(v as u8);
        }
    }
    Ok(out)
}

/// Names of running processes (lowercase, no extension) so the UI can mark
/// tiles whose app is already open.
#[tauri::command]
async fn list_running() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let out = cmd("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-Process | Select-Object -ExpandProperty ProcessName -Unique",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        let s = String::from_utf8_lossy(&out.stdout);
        Ok(s
            .lines()
            .map(|l| l.trim().to_lowercase())
            .filter(|l| !l.is_empty())
            .collect())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

/// Machine name (uppercase) for per-machine config profiles (`machines` block).
#[tauri::command]
fn get_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
        .trim()
        .to_uppercase()
}

fn main() {
    tauri::Builder::default()
        .manage(HotkeyState::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    // A tile binding? Fire it directly, without opening Fay.
                    let binding = {
                        let state = app.state::<HotkeyState>();
                        let hk = state.lock().ok();
                        hk.and_then(|hk| {
                            hk.items
                                .iter()
                                .find(|(s, _)| s == shortcut)
                                .map(|(_, b)| b.clone())
                        })
                    };
                    if let Some(b) = binding {
                        // Off the event loop: the audio switch blocks for a moment.
                        std::thread::spawn(move || {
                            if let Some(dev) = &b.audio_out {
                                let _ = do_set_audio_output(dev);
                            }
                            let _ = do_launch(&b.target, b.elevated);
                        });
                        return;
                    }
                    // Otherwise it's the summon combo.
                    if let Some(w) = app.get_webview_window("main") {
                        toggle_window(&w);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            launch,
            list_monitors,
            hide_window,
            set_audio_output,
            set_summon_hotkey,
            register_item_hotkeys,
            set_autostart,
            get_accent_color,
            load_config,
            save_config,
            config_file_path,
            show_window,
            get_app_icon,
            list_running,
            get_hostname
        ])
        .setup(|app| {
            // Default summon hotkey: Ctrl+Alt+Space (avoids the reserved Win key).
            // The frontend may override it from config via set_summon_hotkey.
            use tauri_plugin_global_shortcut::{Code, Modifiers};
            {
                let state = app.state::<HotkeyState>();
                state.lock().unwrap().summon =
                    Some(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space));
            }
            if let Err(e) = apply_hotkeys(app.handle()) {
                eprintln!("hotkeys: {e}");
            }

            // System tray.
            let show_i = MenuItem::with_id(app, "show", "Show / Hide Fay", true, None::<&str>)?;
            let config_i = MenuItem::with_id(app, "config", "Open config file", true, None::<&str>)?;
            let reload_i = MenuItem::with_id(app, "reload", "Reload config", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Fay", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &config_i, &reload_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("fay-tray")
                .tooltip("Fay — command deck")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            toggle_window(&w);
                        }
                    }
                    "config" => open_config_in_editor(app),
                    "reload" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.eval("location.reload()");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            toggle_window(&w);
                        }
                    }
                })
                .build(app)?;

            // Launcher behavior: hide when focus is lost.
            if let Some(w) = app.get_webview_window("main") {
                let wc = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = wc.hide();
                    }
                });
            }

            // Make the initial window a full-monitor overlay.
            if let Some(w) = app.get_webview_window("main") {
                fill_active_monitor(&w);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Fay");
}

// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};

#[derive(serde::Serialize)]
struct MonitorInfo {
    name: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: f64,
}

/// Show the window if hidden, hide it if visible. Used by the hotkey + tray.
fn toggle_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
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
#[tauri::command]
fn launch(target: String, elevated: Option<bool>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if elevated.unwrap_or(false) {
            // Single-quote escaping for PowerShell ('' is a literal quote).
            let safe = target.replace('\'', "''");
            std::process::Command::new("powershell")
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
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &target])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // Dev convenience so the project compiles/runs off-Windows.
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (target, elevated);
        Err("launch() is only implemented on Windows".into())
    }
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
#[tauri::command]
fn set_audio_output(device: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        for role in ["0", "1"] {
            let status = std::process::Command::new("SoundVolumeView.exe")
                .args(["/SetDefault", &device, role])
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

/// Re-register the global summon hotkey from a config accelerator string, e.g.
/// "Ctrl+Alt+Space" or "CmdOrCtrl+Shift+Space". Keyboard combos only — mouse
/// buttons aren't supported by the global-shortcut system (see DECISIONS.md).
#[tauri::command]
fn set_summon_hotkey(app: tauri::AppHandle, accelerator: String) -> Result<(), String> {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let shortcut =
        Shortcut::from_str(&accelerator).map_err(|e| format!("bad hotkey '{accelerator}': {e}"))?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(shortcut).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(w) = app.get_webview_window("main") {
                            toggle_window(&w);
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            launch,
            list_monitors,
            hide_window,
            set_audio_output,
            set_summon_hotkey
        ])
        .setup(|app| {
            // Summon hotkey: Ctrl+Alt+Space (avoids the reserved Win key).
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
            let summon = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
            app.global_shortcut().register(summon)?;

            // System tray.
            let show_i = MenuItem::with_id(app, "show", "Show / Hide Fay", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Fay", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Fay");
}

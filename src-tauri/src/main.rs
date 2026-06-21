// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Launch a target: an exe, a command on PATH, a file path, a .lnk, or a URL.
/// On Windows we shell through `cmd /C start` so all of those work uniformly —
/// including PowerToys Workspaces shortcuts, which is how Fay triggers a
/// multi-window layout across both monitors.
#[tauri::command]
fn launch(target: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &target])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // Dev convenience so the project at least compiles/runs off-Windows.
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target;
        Err("launch() is only implemented on Windows".into())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![launch])
        .run(tauri::generate_context!())
        .expect("error while running Fay");
}

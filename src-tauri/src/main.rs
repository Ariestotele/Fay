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

/// What a tile does when fired. One shape for every tile kind, shared by clicks
/// (the `fire` command) and direct hotkeys, so both paths behave identically.
///
/// - `launch` (default): `target` is an exe / command / .lnk / URL.
/// - `system`:  `action` is lock / sleep / hibernate / restart / shutdown /
///              logoff / recycle / darkmode.
/// - `media`:   `action` is playpause / next / prev / stop / mute / volup / voldown.
/// - `snippet`: `text` is copied to the clipboard and (unless `paste: false`)
///              pasted into the foreground app with Ctrl+V.
/// - `multi`:   `steps` run in order (each a TileAction; `wait` = pause in ms).
/// - `close`:   `closes` names processes to close (taskkill; `name!` = force).
#[derive(Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TileAction {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    elevated: bool,
    #[serde(default)]
    audio_out: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    paste: Option<bool>,
    #[serde(default)]
    steps: Vec<TileAction>,
    #[serde(default)]
    wait: Option<u64>,
    #[serde(default)]
    closes: Vec<String>,
    #[serde(default)]
    minutes: Option<u32>,
    /// Spoken after the action runs (any kind), if voice is enabled.
    #[serde(default)]
    say: Option<String>,
}

/// Handle to the app for code that runs outside commands (hotkey actions).
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// A global hotkey bound directly to a tile: pressing it performs the tile's
/// action without opening Fay.
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HotkeyBinding {
    accelerator: String,
    #[serde(flatten)]
    action: TileAction,
}

/// Perform a tile action. The optional `audioOut` switch runs first and is
/// best-effort: its error is returned as a warning (`Ok(Some(..))`) so the
/// launch itself still happens.
fn perform(a: &TileAction) -> Result<Option<String>, String> {
    let mut warning = None;
    if let Some(dev) = a.audio_out.as_deref().filter(|d| !d.is_empty()) {
        if let Err(e) = do_set_audio_output(dev) {
            warning = Some(format!("audio: {e}"));
        }
    }
    match a.kind.as_str() {
        "system" => do_system_action(a.action.as_deref().unwrap_or(""))?,
        "media" => do_media_key(a.action.as_deref().unwrap_or(""))?,
        "snippet" => do_paste_text(a.text.as_deref().unwrap_or(""), a.paste.unwrap_or(true))?,
        "wait" => std::thread::sleep(std::time::Duration::from_millis(a.wait.unwrap_or(500).min(60_000))),
        // Clipboard history UI lives in the frontend: summon Fay and open it.
        "clipboard" => {
            let w = APP.get().and_then(|app| app.get_webview_window("main"));
            match w {
                Some(w) => {
                    let inner = w.clone();
                    let _ = w.run_on_main_thread(move || {
                        if !inner.is_visible().unwrap_or(false) {
                            fill_active_monitor(&inner);
                            let _ = inner.show();
                        }
                        let _ = inner.set_focus();
                        let _ = inner.eval("window.__fayClipboard && window.__fayClipboard()");
                    });
                }
                None => return Err("no window".into()),
            }
        }
        // The focus timer lives in the frontend; a hotkey reaches it via eval.
        "focus" => {
            let minutes = a.minutes.unwrap_or(0);
            let action = match a.action.as_deref() {
                Some("stop") => "stop",
                Some("toggle") => "toggle",
                _ => "start",
            };
            let w = APP.get().and_then(|app| app.get_webview_window("main"));
            match w {
                Some(w) => w
                    .eval(&format!("window.__fayFocus && window.__fayFocus({minutes}, '{action}')"))
                    .map_err(|e| e.to_string())?,
                None => return Err("no window for the focus timer".into()),
            }
        }
        "close" => {
            if let Some(w) = do_close(&a.closes)? {
                warning.get_or_insert(w);
            }
        }
        "multi" => {
            for (i, step) in a.steps.iter().enumerate() {
                match perform(step) {
                    Ok(Some(w)) => {
                        warning.get_or_insert(w);
                    }
                    Ok(None) => {}
                    Err(e) => return Err(format!("step {}: {e}", i + 1)),
                }
            }
        }
        "say" => voice_say(a.text.as_deref().unwrap_or(""))?,
        "listen" => voice_listen()?,
        // Screen-aware ask: capture the window in front *before* Fay covers it.
        "ask" => {
            let w = APP.get().and_then(|app| app.get_webview_window("main"));
            let Some(w) = w else { return Err("no window".into()) };
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            let shot = capture_foreground();
            let ok = shot.is_ok();
            let inner = w.clone();
            let _ = w.run_on_main_thread(move || {
                fill_active_monitor(&inner);
                let _ = inner.show();
                let _ = inner.set_focus();
                let _ = inner.eval(&format!("window.__fayAsk && window.__fayAsk({ok})"));
            });
            shot?;
        }
        _ => match a.target.as_deref().filter(|t| !t.is_empty()) {
            Some(t) => do_launch(t, a.elevated)?,
            None => return Err("tile has no target".into()),
        },
    }
    if let Some(s) = a.say.as_deref().filter(|s| !s.trim().is_empty()) {
        let _ = voice_say(s);
    }
    Ok(warning)
}

// ---- voice: Windows System.Speech (offline) --------------------------------
// One persistent PowerShell process hosts both the synthesizer and the
// recognizer so replies are instant and listening doesn't pay the ~1 s engine
// start-up each time. Protocol (stdin, one command per line):
//   say <url-encoded text> · grammar <url-encoded phrases, newline-separated>
//   listen · stop
// It answers on stdout: `heard\t<text>\t<confidence>` or `error\t<message>`,
// which a reader thread forwards to the frontend (window.__fayHeard).

struct VoiceProc {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
}
static VOICE: std::sync::Mutex<Option<VoiceProc>> = std::sync::Mutex::new(None);
static VOICE_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static VOICE_RATE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static VOICE_NAME: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

const VOICE_SCRIPT: &str = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$tts = New-Object System.Speech.Synthesis.SpeechSynthesizer
$tts.Rate = __RATE__
$vn = '__VOICE__'
if ($vn -ne '') { try { $tts.SelectVoice($vn) } catch { } }
$script:rec = $null
$script:phrases = @('fay')
function Ensure-Rec {
  if (-not $script:rec) {
    try {
      $script:rec = New-Object System.Speech.Recognition.SpeechRecognitionEngine
      $script:rec.SetInputToDefaultAudioDevice()
    } catch { $script:rec = $null }
  }
}
function Load-Grammar {
  Ensure-Rec
  if (-not $script:rec) { return }
  $script:rec.UnloadAllGrammars()
  $c = New-Object System.Speech.Recognition.Choices
  $c.Add([string[]]$script:phrases)
  $gb = New-Object System.Speech.Recognition.GrammarBuilder
  $gb.Append($c)
  $script:rec.LoadGrammar((New-Object System.Speech.Recognition.Grammar($gb)))
}
Write-Output "ready`t"
while ($true) {
  $line = [Console]::In.ReadLine()
  if ($null -eq $line) { break }
  if ($line.StartsWith('say ')) {
    $t = [System.Uri]::UnescapeDataString($line.Substring(4))
    $tts.SpeakAsyncCancelAll()
    [void]$tts.SpeakAsync($t)
    continue
  }
  if ($line.StartsWith('grammar ')) {
    $j = [System.Uri]::UnescapeDataString($line.Substring(8))
    $p = @($j -split "`n" | Where-Object { $_.Trim() -ne '' })
    if ($p.Count -gt 0) { $script:phrases = $p; Load-Grammar }
    continue
  }
  if ($line -eq 'listen') {
    Ensure-Rec
    if (-not $script:rec) { Write-Output "error`tno microphone / speech engine"; continue }
    if ($script:rec.Grammars.Count -eq 0) { Load-Grammar }
    $tts.SpeakAsyncCancelAll()
    $res = $null
    try { $res = $script:rec.Recognize([TimeSpan]::FromSeconds(6)) } catch { Write-Output ("error`t" + $_.Exception.Message); continue }
    if ($res) { Write-Output ("heard`t{0}`t{1}" -f $res.Text, $res.Confidence) } else { Write-Output "heard`t`t0" }
    continue
  }
  if ($line -eq 'stop') { $tts.SpeakAsyncCancelAll(); continue }
}
"#;

/// Push a line to the frontend as a `window.__fayHeard(text, confidence, error)` call.
fn voice_forward(line: &str) {
    let mut parts = line.splitn(3, '\t');
    let kind = parts.next().unwrap_or("");
    let a = parts.next().unwrap_or("").to_string();
    let b = parts.next().unwrap_or("").to_string();
    let js = match kind {
        "heard" => format!(
            "window.__fayHeard && window.__fayHeard({}, {}, null)",
            serde_json::to_string(&a).unwrap_or_default(),
            b.trim().parse::<f32>().unwrap_or(0.0)
        ),
        "error" => format!(
            "window.__fayHeard && window.__fayHeard('', 0, {})",
            serde_json::to_string(&a).unwrap_or_default()
        ),
        _ => return,
    };
    if let Some(w) = APP.get().and_then(|app| app.get_webview_window("main")) {
        let _ = w.eval(&js);
    }
}

fn spawn_voice() -> Result<VoiceProc, String> {
    #[cfg(target_os = "windows")]
    {
        use std::io::BufRead;
        let name = VOICE_NAME.lock().map(|n| n.clone()).unwrap_or_default().replace('\'', "''");
        let script = VOICE_SCRIPT
            .replace("__RATE__", &VOICE_RATE.load(std::sync::atomic::Ordering::Relaxed).clamp(-10, 10).to_string())
            .replace("__VOICE__", &name);
        let mut child = powershell_script(&script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("voice: {e}"))?;
        let stdin = child.stdin.take().ok_or("voice: no stdin")?;
        let stdout = child.stdout.take().ok_or("voice: no stdout")?;
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
                voice_forward(&line);
            }
        });
        Ok(VoiceProc { child, stdin })
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("voice is only available on Windows".into())
    }
}

/// Send one protocol line, starting (or restarting) the voice process as needed.
fn voice_send(line: &str) -> Result<(), String> {
    use std::io::Write;
    if !VOICE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("voice is off (app.voice)".into());
    }
    let mut guard = VOICE.lock().map_err(|e| e.to_string())?;
    // Drop a process that has exited.
    if let Some(v) = guard.as_mut() {
        if matches!(v.child.try_wait(), Ok(Some(_))) {
            *guard = None;
        }
    }
    if guard.is_none() {
        *guard = Some(spawn_voice()?);
    }
    let v = guard.as_mut().unwrap();
    if writeln!(v.stdin, "{line}").and_then(|_| v.stdin.flush()).is_err() {
        // Restart once and retry.
        *guard = Some(spawn_voice()?);
        let v = guard.as_mut().unwrap();
        writeln!(v.stdin, "{line}").and_then(|_| v.stdin.flush()).map_err(|e| format!("voice: {e}"))?;
    }
    Ok(())
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn voice_say(text: &str) -> Result<(), String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(());
    }
    voice_send(&format!("say {}", url_encode(t)))
}

fn voice_listen() -> Result<(), String> {
    voice_send("listen")?;
    if let Some(w) = APP.get().and_then(|app| app.get_webview_window("main")) {
        let _ = w.eval("window.__fayListening && window.__fayListening()");
    }
    Ok(())
}

/// Configure voice from `app.voice` / `voiceRate` / `voiceName`; starts the
/// engine right away when enabled so the first reply is instant.
#[tauri::command]
async fn voice_config(enabled: bool, rate: Option<i32>, name: Option<String>) -> Result<(), String> {
    VOICE_RATE.store(rate.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut n) = VOICE_NAME.lock() {
        *n = name.unwrap_or_default();
    }
    VOICE_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    // Restart so rate / voice changes apply.
    if let Ok(mut g) = VOICE.lock() {
        if let Some(mut v) = g.take() {
            let _ = v.child.kill();
        }
    }
    if enabled {
        voice_send("stop")?;
    }
    Ok(())
}

#[tauri::command]
async fn say(text: String) -> Result<(), String> {
    voice_say(&text)
}

#[tauri::command]
async fn listen() -> Result<(), String> {
    voice_listen()
}

#[tauri::command]
async fn set_voice_grammar(phrases: Vec<String>) -> Result<(), String> {
    let joined = phrases
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    voice_send(&format!("grammar {}", url_encode(&joined)))
}

/// Ask processes to close (graceful `taskkill /IM`, which sends WM_CLOSE). A
/// trailing `!` on a name forces it (`/F`). Names that weren't running come
/// back as a warning, not an error, so a scene teardown never "fails".
fn do_close(names: &[String]) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut missed = Vec::new();
        for raw in names {
            let (name, force) = match raw.trim().strip_suffix('!') {
                Some(n) => (n.trim(), true),
                None => (raw.trim(), false),
            };
            if name.is_empty() {
                continue;
            }
            let image = if name.to_ascii_lowercase().ends_with(".exe") {
                name.to_string()
            } else {
                format!("{name}.exe")
            };
            let mut c = cmd("taskkill");
            if force {
                c.arg("/F");
            }
            let ok = c
                .args(["/IM", &image])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ok {
                missed.push(name.to_string());
            }
        }
        Ok(if missed.is_empty() {
            None
        } else {
            Some(format!("not running: {}", missed.join(", ")))
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = names;
        Err("closing processes is only implemented on Windows".into())
    }
}

/// Expand `%VAR%` references (for paths we hand to PowerShell, which does not).
fn expand_env(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[derive(serde::Serialize)]
struct CommandEntry {
    name: String,
    path: String,
    ext: String,
}

/// The "commands folder": drop `.ps1` / `.bat` / `.cmd` / `.exe` / `.lnk` files
/// in it and they become tiles. Default location is `<config dir>\commands`
/// (created so it's easy to find; tray › Open commands folder).
fn commands_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("commands"))
}

#[tauri::command]
async fn list_commands(app: tauri::AppHandle, dir: Option<String>) -> Result<Vec<CommandEntry>, String> {
    let custom = dir.as_deref().map(str::trim).filter(|d| !d.is_empty());
    let path = match custom {
        Some(d) => std::path::PathBuf::from(expand_env(d)),
        None => {
            let p = commands_dir(&app)?;
            let _ = std::fs::create_dir_all(&p);
            p
        }
    };
    let mut out = Vec::new();
    let rd = match std::fs::read_dir(&path) {
        Ok(rd) => rd,
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !["ps1", "bat", "cmd", "exe", "lnk"].contains(&ext.as_str()) {
            continue;
        }
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("command")
            .to_string();
        out.push(CommandEntry { name, path: p.display().to_string(), ext });
    }
    out.sort_by_key(|c| c.name.to_lowercase());
    Ok(out)
}

/// Live readouts drawn around the Heart. CPU / RAM / network from `sysinfo`
/// (kept in state so usage is a delta since the previous poll); GPU load and
/// temperature from `nvidia-smi` when present.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    cpu: f32,
    ram_used: u64,
    ram_total: u64,
    gpu: Option<f32>,
    gpu_temp: Option<f32>,
    down: f64,
    up: f64,
}
struct StatsState {
    sys: sysinfo::System,
    nets: sysinfo::Networks,
    last: std::time::Instant,
}
type StatsMutex = std::sync::Mutex<StatsState>;

/// Set once nvidia-smi has failed: on an AMD/Intel machine it is absent, and
/// the stats poll would otherwise spawn a doomed process every few seconds for
/// as long as Fay runs. The GPU readouts simply stay blank instead.
static NO_NVIDIA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn gpu_stats() -> (Option<f32>, Option<f32>) {
    if NO_NVIDIA.load(std::sync::atomic::Ordering::Relaxed) {
        return (None, None);
    }
    let out = cmd("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,temperature.gpu", "--format=csv,noheader,nounits"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let mut it = s
                .lines()
                .next()
                .unwrap_or("")
                .split(',')
                .map(|x| x.trim().parse::<f32>().ok());
            (it.next().flatten(), it.next().flatten())
        }
        // Not installed (spawn failed) — never try again this run. A non-zero
        // exit is left retryable: the driver may just be busy.
        Err(_) => {
            NO_NVIDIA.store(true, std::sync::atomic::Ordering::Relaxed);
            (None, None)
        }
        _ => (None, None),
    }
}

#[tauri::command]
async fn get_stats(state: tauri::State<'_, StatsMutex>) -> Result<Stats, String> {
    let (cpu, ram_used, ram_total, down, up) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let now = std::time::Instant::now();
        let dt = now.duration_since(s.last).as_secs_f64().max(0.25);
        s.last = now;
        s.sys.refresh_cpu_usage();
        s.sys.refresh_memory();
        s.nets.refresh(true);
        let (mut rx, mut tx) = (0u64, 0u64);
        for (_, d) in s.nets.iter() {
            rx += d.received();
            tx += d.transmitted();
        }
        (
            s.sys.global_cpu_usage(),
            s.sys.used_memory(),
            s.sys.total_memory(),
            rx as f64 / dt,
            tx as f64 / dt,
        )
    };
    let (gpu, gpu_temp) = gpu_stats();
    Ok(Stats { cpu, ram_used, ram_total, gpu, gpu_temp, down, up })
}

// ---- clipboard history -----------------------------------------------------
// In memory only (never written to disk). A poller thread watches the
// clipboard sequence number and records new text; entries marked by password
// managers as "exclude from monitoring" are skipped.

static CLIP_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CLIP_MAX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(50);
static CLIP_HISTORY: std::sync::Mutex<std::collections::VecDeque<String>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
static CLIP_THREAD: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// Current clipboard text, or None if it isn't text / is excluded / is busy.
#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        let excl: Vec<u16> = "ExcludeClipboardContentFromMonitorProcessing\0".encode_utf16().collect();
        let excl_fmt = RegisterClipboardFormatW(excl.as_ptr());
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return None;
        }
        if excl_fmt != 0 && IsClipboardFormatAvailable(excl_fmt) != 0 {
            return None;
        }
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT);
        let mut out = None;
        if !h.is_null() {
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let mut len = 0usize;
                while len < 200_000 && *p.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(p, len);
                out = Some(String::from_utf16_lossy(slice));
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        out
    }
}

fn clip_push(text: String) {
    if text.trim().is_empty() {
        return;
    }
    if let Ok(mut h) = CLIP_HISTORY.lock() {
        h.retain(|t| t != &text);
        h.push_front(text);
        let max = CLIP_MAX.load(std::sync::atomic::Ordering::Relaxed).max(1);
        while h.len() > max {
            h.pop_back();
        }
    }
}

fn start_clipboard_watch() {
    CLIP_THREAD.get_or_init(|| {
        std::thread::spawn(|| {
            #[cfg(target_os = "windows")]
            {
                use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
                let mut last = unsafe { GetClipboardSequenceNumber() };
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    if !CLIP_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let seq = unsafe { GetClipboardSequenceNumber() };
                    if seq == last {
                        continue;
                    }
                    // Only advance once we managed to read (the owner may still hold it).
                    if let Some(t) = read_clipboard_text() {
                        last = seq;
                        clip_push(t);
                    } else {
                        last = seq;
                    }
                }
            }
        });
    });
}

/// Turn the clipboard watcher on/off (config `app.clipboard`), with a max size.
#[tauri::command]
fn set_clipboard_watch(enabled: bool, max: Option<usize>) {
    if let Some(m) = max {
        CLIP_MAX.store(m.clamp(1, 500), std::sync::atomic::Ordering::Relaxed);
    }
    CLIP_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    if enabled {
        start_clipboard_watch();
    }
}

#[tauri::command]
fn clipboard_history() -> Vec<String> {
    CLIP_HISTORY.lock().map(|h| h.iter().cloned().collect()).unwrap_or_default()
}

#[tauri::command]
fn clipboard_clear() {
    if let Ok(mut h) = CLIP_HISTORY.lock() {
        h.clear();
    }
}

/// Put a history entry back on the clipboard and (optionally) paste it.
#[tauri::command]
async fn clipboard_pick(text: String, paste: bool) -> Result<(), String> {
    do_paste_text(&text, paste)
}

// ---- file search (voidtools Everything) ------------------------------------

#[derive(serde::Serialize)]
struct FileHit {
    name: String,
    path: String,
    dir: bool,
}

/// Query Everything through its `es.exe` CLI. `es` may be a full path from the
/// config; otherwise `es.exe` must be on PATH.
#[tauri::command]
async fn search_files(query: String, max: Option<u32>, es: Option<String>) -> Result<Vec<FileHit>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let exe = es
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(expand_env)
        .unwrap_or_else(|| "es.exe".to_string());
    let n = max.unwrap_or(30).clamp(1, 200).to_string();
    let out = cmd(&exe)
        .args(["-n", &n, "-sort", "date-modified-descending", q])
        .output()
        .map_err(|_| "Everything's es.exe not found — install Everything + es.exe (see SETUP)".to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "Everything is not running".into() } else { err });
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let p = std::path::Path::new(l);
            FileHit {
                name: p.file_name().and_then(|n| n.to_str()).unwrap_or(l).to_string(),
                path: l.to_string(),
                dir: p.is_dir(),
            }
        })
        .collect())
}

/// Open Explorer with the file selected.
#[tauri::command]
async fn reveal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        cmd("explorer.exe")
            .arg(format!("/select,{path}"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("reveal is only implemented on Windows".into())
    }
}

// ---- browser bookmarks -----------------------------------------------------

#[derive(Clone, serde::Serialize)]
struct Bookmark {
    title: String,
    url: String,
    browser: String,
}

static BOOKMARK_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<Bookmark>)>> =
    std::sync::Mutex::new(None);

fn collect_moz(node: &serde_json::Value, out: &mut Vec<Bookmark>, browser: &str) {
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for c in children {
            collect_moz(c, out, browser);
        }
    } else if let Some(uri) = node.get("uri").and_then(|u| u.as_str()) {
        if uri.starts_with("http") {
            out.push(Bookmark {
                title: node.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                url: uri.to_string(),
                browser: browser.to_string(),
            });
        }
    }
}

fn collect_chromium(node: &serde_json::Value, out: &mut Vec<Bookmark>, browser: &str) {
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for c in children {
            collect_chromium(c, out, browser);
        }
    } else if node.get("type").and_then(|t| t.as_str()) == Some("url") {
        if let Some(url) = node.get("url").and_then(|u| u.as_str()) {
            out.push(Bookmark {
                title: node.get("name").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                url: url.to_string(),
                browser: browser.to_string(),
            });
        }
    }
}

/// Newest `bookmarkbackups/*.jsonlz4` under a Firefox-family profiles root.
fn newest_moz_backup(profiles_root: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for prof in std::fs::read_dir(profiles_root).ok()?.flatten() {
        let dir = prof.path().join("bookmarkbackups");
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for f in rd.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) != Some("jsonlz4") {
                continue;
            }
            let Ok(m) = f.metadata().and_then(|m| m.modified()) else { continue };
            if best.as_ref().map(|(t, _)| m > *t).unwrap_or(true) {
                best = Some((m, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn read_mozlz4(path: &std::path::Path) -> Option<serde_json::Value> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 12 || &data[..8] != b"mozLz40\0" {
        return None;
    }
    let size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let raw = lz4_flex::block::decompress(&data[12..], size).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn load_bookmarks() -> Vec<Bookmark> {
    let mut out = Vec::new();
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    if appdata.is_empty() {
        return out;
    }
    // Firefox family: daily jsonlz4 backups (may lag the live DB by a day).
    for (name, sub) in [
        ("Zen", "zen\\Profiles"),
        ("Firefox", "Mozilla\\Firefox\\Profiles"),
        ("LibreWolf", "librewolf\\Profiles"),
        ("Floorp", "Floorp\\Profiles"),
    ] {
        let root = std::path::Path::new(&appdata).join(sub);
        if let Some(p) = newest_moz_backup(&root) {
            if let Some(json) = read_mozlz4(&p) {
                collect_moz(&json, &mut out, name);
            }
        }
    }
    // Chromium family: plain JSON.
    if !local.is_empty() {
        for (name, sub) in [
            ("Chrome", "Google\\Chrome\\User Data\\Default\\Bookmarks"),
            ("Edge", "Microsoft\\Edge\\User Data\\Default\\Bookmarks"),
            ("Brave", "BraveSoftware\\Brave-Browser\\User Data\\Default\\Bookmarks"),
            ("Vivaldi", "Vivaldi\\User Data\\Default\\Bookmarks"),
        ] {
            let p = std::path::Path::new(&local).join(sub);
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
            if let Some(roots) = json.get("roots").and_then(|r| r.as_object()) {
                for (_, root) in roots {
                    collect_chromium(root, &mut out, name);
                }
            }
        }
    }
    out
}

/// All bookmarks from every browser found, cached for five minutes.
#[tauri::command]
async fn list_bookmarks(refresh: Option<bool>) -> Result<Vec<Bookmark>, String> {
    if refresh != Some(true) {
        if let Ok(c) = BOOKMARK_CACHE.lock() {
            if let Some((t, list)) = c.as_ref() {
                if t.elapsed() < std::time::Duration::from_secs(300) {
                    return Ok(list.clone());
                }
            }
        }
    }
    let list = load_bookmarks();
    if let Ok(mut c) = BOOKMARK_CACHE.lock() {
        *c = Some((std::time::Instant::now(), list.clone()));
    }
    Ok(list)
}

// ---- AI: natural-language deck control + screen-aware questions -------------
// Provider is Anthropic (API key from config or ANTHROPIC_API_KEY) or a local
// Ollama. The frontend owns the prompt and the plan execution; this side only
// makes the HTTP call and holds the last screenshot.

#[derive(Clone, Default)]
struct AiConfig {
    provider: String,
    model: String,
    api_key: String,
    ollama_url: String,
}
static AI: std::sync::Mutex<Option<AiConfig>> = std::sync::Mutex::new(None);
static SCREEN: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[tauri::command]
fn ai_config(provider: Option<String>, model: Option<String>, api_key: Option<String>, ollama_url: Option<String>) -> Result<String, String> {
    let provider = provider.unwrap_or_default().trim().to_lowercase();
    let provider = if provider.is_empty() { "anthropic".to_string() } else { provider };
    let api_key = api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .unwrap_or_default()
        .trim()
        .to_string();
    let cfg = AiConfig {
        model: model.filter(|m| !m.trim().is_empty()).unwrap_or_else(|| {
            if provider == "ollama" { "llama3.2".into() } else { "claude-sonnet-5".into() }
        }),
        ollama_url: ollama_url
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| "http://localhost:11434".into())
            .trim_end_matches('/')
            .to_string(),
        api_key,
        provider,
    };
    let status = if cfg.provider == "ollama" {
        format!("ollama · {}", cfg.model)
    } else if cfg.api_key.is_empty() {
        "no API key".to_string()
    } else {
        format!("anthropic · {}", cfg.model)
    };
    if let Ok(mut g) = AI.lock() {
        *g = Some(cfg);
    }
    Ok(status)
}

#[derive(serde::Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Capture the foreground window (or the primary screen) as a PNG data string.
fn capture_foreground() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const PS: &str = r#"
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Runtime.InteropServices;
public class FayWin {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L; public int T; public int R; public int B; }
}
"@
[void][FayWin]::SetProcessDPIAware()
$h = [FayWin]::GetForegroundWindow()
$r = New-Object FayWin+RECT
[void][FayWin]::GetWindowRect($h, [ref]$r)
$x = $r.L; $y = $r.T; $w = $r.R - $r.L; $hh = $r.B - $r.T
if ($w -lt 80 -or $hh -lt 80) { $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $x = $b.X; $y = $b.Y; $w = $b.Width; $hh = $b.Height }
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
$scale = [Math]::Min(1.0, 1400.0 / $w)
if ($scale -lt 1) { $bmp = New-Object System.Drawing.Bitmap $bmp, ([int]($w * $scale)), ([int]($hh * $scale)) }
$ms = New-Object IO.MemoryStream
$bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
[Convert]::ToBase64String($ms.ToArray())
"#;
        let out = powershell_script(PS).output().map_err(|e| e.to_string())?;
        let b64 = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() || b64.len() < 100 {
            return Err("screen capture failed".into());
        }
        if let Ok(mut g) = SCREEN.lock() {
            *g = Some(b64);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("screen capture is only implemented on Windows".into())
    }
}

#[tauri::command]
async fn capture_screen() -> Result<(), String> {
    capture_foreground()
}

/// One round-trip to the model. `with_screen` attaches the last capture to
/// the final user message. Returns the assistant's text.
#[tauri::command]
async fn ai_ask(system: String, messages: Vec<ChatMessage>, with_screen: bool) -> Result<String, String> {
    let cfg = AI.lock().ok().and_then(|g| g.clone()).ok_or("AI is not configured (app.ai)")?;
    let shot = if with_screen { SCREEN.lock().ok().and_then(|g| g.clone()) } else { None };
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(90))
        .build();
    let n = messages.len();
    if cfg.provider == "ollama" {
        let mut msgs = vec![serde_json::json!({ "role": "system", "content": system })];
        for (i, m) in messages.iter().enumerate() {
            let mut v = serde_json::json!({ "role": m.role, "content": m.content });
            if i + 1 == n {
                if let Some(s) = &shot {
                    v["images"] = serde_json::json!([s]);
                }
            }
            msgs.push(v);
        }
        let body = serde_json::json!({ "model": cfg.model, "messages": msgs, "stream": false });
        let resp = agent
            .post(&format!("{}/api/chat", cfg.ollama_url))
            .send_json(body)
            .map_err(|e| ai_err("ollama", e))?;
        let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        return v["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "ollama: empty reply".into());
    }
    if cfg.api_key.is_empty() {
        return Err("no API key — set app.ai.apiKey in your config (or ANTHROPIC_API_KEY)".into());
    }
    let mut msgs = Vec::new();
    for (i, m) in messages.iter().enumerate() {
        let content = if i + 1 == n && shot.is_some() {
            serde_json::json!([
                { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": shot.as_ref().unwrap() } },
                { "type": "text", "text": m.content }
            ])
        } else {
            serde_json::json!(m.content)
        };
        msgs.push(serde_json::json!({ "role": m.role, "content": content }));
    }
    let body = serde_json::json!({
        "model": cfg.model,
        "max_tokens": 700,
        "system": system,
        "messages": msgs
    });
    let resp = agent
        .post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &cfg.api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(body)
        .map_err(|e| ai_err("anthropic", e))?;
    let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    let text = v["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if text.is_empty() {
        Err("anthropic: empty reply".into())
    } else {
        Ok(text)
    }
}

fn ai_err(who: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
                .unwrap_or(body);
            format!("{who}: HTTP {code} — {}", msg.chars().take(200).collect::<String>())
        }
        ureq::Error::Transport(t) => format!("{who}: {t}"),
    }
}

/// Native "browse for a program" dialog (WinForms, STA PowerShell). Returns
/// the chosen path, or None if cancelled.
#[tauri::command]
async fn pick_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        const PS: &str = r#"Add-Type -AssemblyName System.Windows.Forms
$d = New-Object System.Windows.Forms.OpenFileDialog
$d.Title = 'Fay — choose a program, shortcut or script'
$d.Filter = 'Programs & shortcuts|*.exe;*.lnk;*.bat;*.cmd;*.ps1|All files|*.*'
$d.InitialDirectory = [Environment]::GetFolderPath('ProgramFiles')
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { $d.FileName }"#;
        let out = powershell_script_with(&["-STA"], PS).output().map_err(|e| e.to_string())?;
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(if p.is_empty() { None } else { Some(p) })
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("file picker is only implemented on Windows".into())
    }
}

/// A Windows toast (used when the focus timer ends while Fay is hidden).
#[tauri::command]
async fn notify(title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;")
                .replace('`', "``")
                .replace('$', "`$")
        };
        let (t, b) = (esc(&title), esc(&body));
        // PowerShell's own AppUserModelId is registered for toasts on every PC.
        let ps = format!(
            r#"$aumid='{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\WindowsPowerShell\v1.0\powershell.exe'
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>{t}</text><text>{b}</text></binding></visual></toast>")
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($aumid).Show([Windows.UI.Notifications.ToastNotification]::new($xml))"#
        );
        powershell_script(&ps).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (title, body);
        Err("notifications are only implemented on Windows".into())
    }
}

// ---- doctor: first-run self-check ------------------------------------------
// One tile runs every environment check that no CI can: do the tile targets
// exist, are the helper tools on PATH, is the voice host up, is AI reachable.
// The frontend renders the rows and copies the report on Enter.

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorTarget {
    #[serde(default)]
    id: String,
    name: String,
    target: String,
    /// Search the machine for this app when its target is missing. Off for
    /// scene tiles: those point at a PowerToys Workspace shortcut the owner
    /// creates, so any same-named app found elsewhere would be the wrong file.
    #[serde(default)]
    find: bool,
    /// Appended to the row when the target is missing and nothing was found.
    #[serde(default)]
    missing_hint: Option<String>,
}

#[derive(serde::Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct DoctorRow {
    /// Check label, e.g. "tile Zen" or "tool es.exe".
    name: String,
    /// true = ok, false = problem, None = informational.
    ok: Option<bool>,
    detail: String,
    /// A working path found for a tile whose target is missing, and the tile
    /// it belongs to: together these let the frontend repair the config.
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tile_id: Option<String>,
}

fn row(name: &str, ok: Option<bool>, detail: impl Into<String>) -> DoctorRow {
    DoctorRow { name: name.to_string(), ok, detail: detail.into(), ..Default::default() }
}

/// How well a file's stem matches the tile name: 0 = exact, 1 = prefix,
/// 2 = contains. `None` = not a match at all, or an uninstaller.
fn match_score(stem: &str, want: &str) -> Option<u8> {
    let s = stem.trim().to_ascii_lowercase();
    let w = want.trim().to_ascii_lowercase();
    if w.is_empty() || s.is_empty() || s.contains("uninstall") {
        return None;
    }
    if s == w {
        Some(0)
    } else if s.starts_with(&w) {
        Some(1)
    } else if s.contains(&w) {
        Some(2)
    } else {
        None
    }
}

/// Walk `dir` for a file matching `want`, keeping the best score seen.
/// Depth- and entry-budgeted so scanning Program Files can't run away.
fn walk_for(
    dir: &std::path::Path,
    want: &str,
    exts: &[&str],
    depth: u32,
    budget: &mut u32,
    best: &mut Option<(u8, String)>,
) {
    if depth == 0 || *budget == 0 || matches!(best, Some((0, _))) {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let Ok(ft) = entry.file_type() else { continue };
        let p = entry.path();
        if ft.is_dir() {
            walk_for(&p, want, exts, depth - 1, budget, best);
            if matches!(best, Some((0, _))) {
                return;
            }
            continue;
        }
        let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
        if !exts.contains(&ext.as_str()) {
            continue;
        }
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let Some(score) = match_score(stem, want) else { continue };
        if best.as_ref().map_or(true, |(b, _)| score < *b) {
            *best = Some((score, p.display().to_string()));
            if score == 0 {
                return;
            }
        }
    }
}

/// Find where an app actually lives, by tile name. Start Menu shortcuts are
/// tried first: they carry the right working directory and arguments, which is
/// why the Discord tile works while guessed `.exe` paths go stale on update.
fn find_app(name: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let mut best: Option<(u8, String)> = None;
        let mut budget: u32 = 40_000;
        for (var, sub) in [
            ("APPDATA", "Microsoft\\Windows\\Start Menu\\Programs"),
            ("ProgramData", "Microsoft\\Windows\\Start Menu\\Programs"),
        ] {
            if let Ok(d) = std::env::var(var) {
                walk_for(&std::path::Path::new(&d).join(sub), name, &["lnk"], 4, &mut budget, &mut best);
            }
        }
        if best.is_some() {
            return best.map(|(_, p)| p);
        }
        // No shortcut: look for the exe in the usual install roots.
        for (var, sub) in [
            ("LOCALAPPDATA", "Programs"),
            ("LOCALAPPDATA", ""),
            ("ProgramFiles", ""),
            ("ProgramFiles(x86)", ""),
        ] {
            if let Ok(d) = std::env::var(var) {
                let root = std::path::Path::new(&d).join(sub);
                walk_for(&root, name, &["exe"], 3, &mut budget, &mut best);
            }
        }
        best.map(|(_, p)| p)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        None
    }
}

/// Where a command resolves on PATH (Windows `where.exe`), if anywhere.
fn which(exe: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let out = cmd("where.exe").arg(exe).output().ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout).lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = exe;
        None
    }
}

/// Classify a tile target: URL / protocol, existing file, command on PATH, or missing.
fn check_target(target: &str) -> (Option<bool>, String) {
    let t = target.trim();
    if t.is_empty() {
        return (Some(false), "empty target".into());
    }
    // A protocol (steam://, ms-settings:, https://) — nothing to check on disk.
    let proto = t.split_once(':').map(|(p, _)| p);
    let is_drive = t.len() > 1 && t.as_bytes()[1] == b':' && t.as_bytes()[0].is_ascii_alphabetic();
    if let (Some(p), false) = (proto, is_drive) {
        if !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return (None, format!("{p}: link — opened by Windows"));
        }
    }
    let expanded = expand_env(t);
    if expanded.contains('%') {
        return (Some(false), format!("unknown %VAR% in {expanded}"));
    }
    let p = std::path::Path::new(&expanded);
    if p.exists() {
        return (Some(true), expanded);
    }
    if !expanded.contains(['\\', '/']) {
        return match which(&expanded).or_else(|| which(&format!("{expanded}.exe"))) {
            Some(w) => (Some(true), format!("on PATH: {w}")),
            None => (Some(false), format!("\"{expanded}\" is not a file and not on PATH")),
        };
    }
    (Some(false), format!("not found: {expanded}"))
}

fn tool_row(label: &str, exe: &str, why: &str) -> DoctorRow {
    match which(exe) {
        Some(w) => row(label, Some(true), w),
        None => row(label, Some(false), format!("{exe} not on PATH — {why}")),
    }
}

#[tauri::command]
async fn doctor(app: tauri::AppHandle, targets: Vec<DoctorTarget>, es: Option<String>) -> Result<Vec<DoctorRow>, String> {
    let mut rows = Vec::new();

    // Where things live.
    rows.push(row("machine", None, format!("{} · {}", get_hostname(), std::env::consts::OS)));
    match config_path(&app) {
        Ok(p) => rows.push(row("config file", Some(p.exists()), p.display().to_string())),
        Err(e) => rows.push(row("config file", Some(false), e)),
    }
    if let Ok(d) = commands_dir(&app) {
        let n = std::fs::read_dir(&d).map(|rd| rd.flatten().filter(|e| e.path().is_file()).count()).unwrap_or(0);
        rows.push(row("commands folder", None, format!("{} · {n} file(s)", d.display())));
    }

    // Tile targets: the checks the owner otherwise does by clicking each tile.
    // A missing one triggers a search, so the report says where the app really
    // is instead of only that the guess was wrong.
    for t in &targets {
        let (ok, detail) = check_target(&t.target);
        let mut r = row(&format!("tile {}", t.name), ok, detail);
        if ok == Some(false) {
            match if t.find { find_app(&t.name) } else { None } {
                Some(found) => {
                    r.detail = format!("{} → found: {found}", r.detail);
                    r.fix = Some(found);
                    r.tile_id = Some(t.id.clone());
                }
                None => {
                    let hint = t.missing_hint.as_deref().unwrap_or(if t.find {
                        "not installed anywhere I can see"
                    } else {
                        "nothing to search for"
                    });
                    r.detail = format!("{} ({hint})", r.detail);
                }
            }
        }
        rows.push(r);
    }

    // Helper tools.
    rows.push(tool_row("PowerShell", "powershell.exe", "needed for voice, toasts, system actions"));
    let pt = ["LOCALAPPDATA", "ProgramFiles"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|d| std::path::Path::new(&d).join("PowerToys").join("PowerToys.exe"))
        .find(|p| p.exists());
    rows.push(match pt {
        Some(p) => row("PowerToys", Some(true), p.display().to_string()),
        None => row("PowerToys", Some(false), "not found — Workspaces scenes need it (see SETUP)"),
    });
    rows.push(tool_row("SoundVolumeView", "SoundVolumeView.exe", "scene audioOut needs it (NirSoft)"));
    let es_exe = es.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(expand_env).unwrap_or_else(|| "es.exe".into());
    rows.push(if std::path::Path::new(&es_exe).exists() {
        row("Everything es.exe", Some(true), es_exe)
    } else {
        tool_row("Everything es.exe", &es_exe, "'>' file search needs it (voidtools)")
    });
    rows.push(match which("nvidia-smi") {
        Some(w) => row("nvidia-smi", Some(true), w),
        None => row("nvidia-smi", None, "not found — GPU readouts stay blank (fine on AMD/Intel)"),
    });

    // Runtime state of the long-lived helpers.
    let clip = CLIP_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
    let n = CLIP_HISTORY.lock().map(|h| h.len()).unwrap_or(0);
    rows.push(row("clipboard watcher", None, if clip { format!("on · {n} entries so far") } else { "off (app.clipboard)".into() }));
    let bm = list_bookmarks(Some(false)).await.map(|b| b.len()).unwrap_or(0);
    rows.push(row("bookmarks", None, format!("{bm} found across browsers")));
    let venabled = VOICE_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
    let vrunning = VOICE.lock().map(|v| v.is_some()).unwrap_or(false);
    rows.push(row(
        "voice host",
        if venabled { Some(vrunning) } else { None },
        match (venabled, vrunning) {
            (false, _) => "off (app.voice)".to_string(),
            (true, true) => "System.Speech process running".to_string(),
            (true, false) => "not started yet — starts on the first say/listen".to_string(),
        },
    ));

    // AI: configured? For Ollama also ping the server (cheap, local).
    let ai = AI.lock().ok().and_then(|g| (*g).clone());
    rows.push(match ai {
        None => row("AI", Some(false), "not configured"),
        Some(c) if c.provider == "ollama" => {
            let url = format!("{}/api/tags", c.ollama_url);
            let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(2)).build();
            match agent.get(&url).call() {
                Ok(_) => row("AI", Some(true), format!("ollama · {} · {} reachable", c.model, c.ollama_url)),
                Err(e) => row("AI", Some(false), format!("ollama at {} not reachable: {e}", c.ollama_url)),
            }
        }
        Some(c) if c.api_key.is_empty() => row("AI", Some(false), "no API key — set app.ai.apiKey in the local config or ANTHROPIC_API_KEY"),
        Some(c) => row("AI", Some(true), format!("anthropic · {} · key set ({} chars)", c.model, c.api_key.len())),
    });

    Ok(rows)
}

fn open_commands_folder(app: &tauri::AppHandle) {
    if let Ok(dir) = commands_dir(app) {
        let _ = std::fs::create_dir_all(&dir);
        #[cfg(target_os = "windows")]
        {
            let _ = cmd("explorer.exe").arg(&dir).spawn();
        }
    }
}

/// Fire a tile from the UI. Async: every branch shells out or sleeps.
#[tauri::command]
async fn fire(action: TileAction) -> Result<Option<String>, String> {
    perform(&action)
}

/// Fixed table of system commands. Nothing here takes user input, so a tile
/// can only ever run one of these exact commands.
fn do_system_action(action: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        fn ps(script: &str) -> (&str, Vec<&str>) {
            ("powershell", vec!["-NoProfile", "-NonInteractive", "-Command", script])
        }
        let (prog, args): (&str, Vec<&str>) = match action {
            "lock" => ("rundll32.exe", vec!["user32.dll,LockWorkStation"]),
            // The .NET call sleeps reliably; the popular rundll32 SetSuspendState
            // one-liner hibernates instead when hibernation is enabled.
            "sleep" => ps("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Application]::SetSuspendState('Suspend',$false,$false)"),
            "hibernate" => ("shutdown", vec!["/h"]),
            "restart" => ("shutdown", vec!["/r", "/t", "0"]),
            "shutdown" => ("shutdown", vec!["/s", "/t", "0"]),
            "logoff" => ("shutdown", vec!["/l"]),
            "recycle" => ps("Clear-RecycleBin -Force -ErrorAction SilentlyContinue"),
            "darkmode" => ps(r"$k='HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'; $n=1-[int](Get-ItemProperty -Path $k -Name AppsUseLightTheme -ErrorAction SilentlyContinue).AppsUseLightTheme; Set-ItemProperty -Path $k -Name AppsUseLightTheme -Value $n -Type DWord; Set-ItemProperty -Path $k -Name SystemUsesLightTheme -Value $n -Type DWord"),
            _ => return Err(format!("unknown system action '{action}'")),
        };
        cmd(prog).args(args).spawn().map_err(|e| format!("{action}: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = action;
        Err("system actions are only implemented on Windows".into())
    }
}

/// Is a virtual key currently held? (modifier checks for the hook + paste)
#[cfg(target_os = "windows")]
fn key_down(vk: u16) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

/// Synthesize one key press/release with SendInput.
#[cfg(target_os = "windows")]
fn send_vk(vk: u16, down: bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if down { 0 } else { KEYEVENTF_KEYUP },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Tap a key (down + up).
#[cfg(target_os = "windows")]
fn tap_vk(vk: u16) {
    send_vk(vk, true);
    send_vk(vk, false);
}

/// Media / volume keys, sent as the real multimedia virtual keys so whatever is
/// playing responds (Spotify, browser, game) exactly like a keyboard's keys.
fn do_media_key(action: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP,
            VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
        };
        let vk = match action {
            "playpause" | "play" | "pause" => VK_MEDIA_PLAY_PAUSE,
            "next" => VK_MEDIA_NEXT_TRACK,
            "prev" | "previous" => VK_MEDIA_PREV_TRACK,
            "stop" => VK_MEDIA_STOP,
            "mute" => VK_VOLUME_MUTE,
            "volup" => VK_VOLUME_UP,
            "voldown" => VK_VOLUME_DOWN,
            _ => return Err(format!("unknown media action '{action}'")),
        };
        tap_vk(vk);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = action;
        Err("media keys are only implemented on Windows".into())
    }
}

/// Put text on the clipboard as CF_UNICODETEXT (native; no PowerShell start-up).
fn set_clipboard_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
        const CF_UNICODETEXT: u32 = 13;
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            // Another app may hold the clipboard for a moment; retry briefly.
            let mut opened = false;
            for _ in 0..10 {
                if OpenClipboard(std::ptr::null_mut()) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            if !opened {
                return Err("clipboard is busy".into());
            }
            EmptyClipboard();
            let h = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
            if h.is_null() {
                CloseClipboard();
                return Err("clipboard: allocation failed".into());
            }
            let p = GlobalLock(h) as *mut u16;
            if p.is_null() {
                CloseClipboard();
                return Err("clipboard: lock failed".into());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
            GlobalUnlock(h);
            // On success the system owns the memory; we must not free it.
            let ok = !SetClipboardData(CF_UNICODETEXT, h).is_null();
            CloseClipboard();
            if !ok {
                return Err("clipboard: set failed".into());
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        Err("clipboard is only implemented on Windows".into())
    }
}

#[tauri::command]
async fn set_clipboard(text: String) -> Result<(), String> {
    set_clipboard_text(&text)
}

/// Snippet: copy `text`, then (optionally) paste it into whatever app is in
/// front with Ctrl+V. Waits for the user's modifier keys to be released first,
/// otherwise a hotkey like Ctrl+Alt+S would turn the paste into Ctrl+Alt+V.
fn do_paste_text(text: &str, paste: bool) -> Result<(), String> {
    set_clipboard_text(text)?;
    #[cfg(target_os = "windows")]
    {
        if paste {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
            };
            for _ in 0..40 {
                if ![VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN].iter().any(|&k| key_down(k)) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            // Let the previous window take focus back after Fay hides.
            std::thread::sleep(std::time::Duration::from_millis(150));
            send_vk(VK_CONTROL, true);
            tap_vk(0x56); // V
            send_vk(VK_CONTROL, false);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = paste;
    }
    Ok(())
}

/// Which monitor the overlay appears on: under the cursor (default) or the one
/// the window was last on. Set from `app.summonOn` via `set_summon_monitor`.
static SUMMON_AT_CURSOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[tauri::command]
fn set_summon_monitor(mode: String) {
    SUMMON_AT_CURSOR.store(mode.trim() != "current", std::sync::atomic::Ordering::Relaxed);
}

/// All registered global shortcuts: the summon combo plus per-tile bindings.
#[derive(Default)]
struct Hotkeys {
    summon: Option<Shortcut>,
    items: Vec<(Shortcut, HotkeyBinding)>,
}
type HotkeyState = std::sync::Mutex<Hotkeys>;

/// PowerShell with a multi-line script passed as `-EncodedCommand` (UTF-16LE
/// base64), which sidesteps every command-line quoting rule for `"`, `$`, etc.
fn powershell_script(script: &str) -> std::process::Command {
    powershell_script_with(&[], script)
}
fn powershell_script_with(flags: &[&str], script: &str) -> std::process::Command {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut c = cmd("powershell");
    c.args(["-NoProfile", "-NonInteractive"]);
    c.args(flags);
    c.args(["-EncodedCommand", &b64(&utf16)]);
    c
}

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

/// Resize/move the window to fill a whole monitor (HUD overlay): the one under
/// the mouse cursor by default, else the one the window is currently on.
fn fill_active_monitor(window: &WebviewWindow) {
    let under_cursor = if SUMMON_AT_CURSOR.load(std::sync::atomic::Ordering::Relaxed) {
        window
            .cursor_position()
            .ok()
            .and_then(|p| window.monitor_from_point(p.x, p.y).ok().flatten())
    } else {
        None
    };
    let monitor = under_cursor.or_else(|| window.current_monitor().ok().flatten());
    if let Some(m) = monitor {
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
        // PowerShell scripts: the .ps1 file association is "edit", so run them
        // explicitly (hidden console; scripts that need a window can use a .bat).
        if target.to_ascii_lowercase().ends_with(".ps1") {
            let script = expand_env(target);
            if elevated {
                let safe = script.replace('\'', "''");
                cmd("powershell")
                    .args([
                        "-NoProfile",
                        "-WindowStyle",
                        "Hidden",
                        "-Command",
                        &format!("Start-Process -Verb RunAs -FilePath powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{safe}\"'"),
                    ])
                    .spawn()
                    .map_err(|e| e.to_string())?;
            } else {
                cmd("powershell")
                    .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File", &script])
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
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

/// Mouse-button summon (e.g. Ctrl+Mouse5). The global-shortcut plugin is
/// keyboard-only, so this runs a WH_MOUSE_LL hook on its own thread and toggles
/// the window when the configured button+modifiers are pressed. The matching
/// click is swallowed so the app under the cursor doesn't also act on it.
#[cfg(target_os = "windows")]
mod mouse_summon {
    use super::key_down;
    use std::sync::{Mutex, OnceLock};
    use tauri::Manager;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_MENU, VK_SHIFT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
        WM_XBUTTONDOWN,
    };

    #[derive(Clone, Copy)]
    pub struct Combo {
        pub ctrl: bool,
        pub alt: bool,
        pub shift: bool,
        /// XBUTTON1 (= "Mouse4", back) is 1, XBUTTON2 (= "Mouse5", forward) is 2.
        pub xbutton: u16,
    }

    static COMBO: Mutex<Option<Combo>> = Mutex::new(None);
    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

    /// Parse "Ctrl+Mouse5", "Alt+Shift+Mouse4", "Mouse5", …
    pub fn parse(spec: &str) -> Option<Combo> {
        let mut c = Combo { ctrl: false, alt: false, shift: false, xbutton: 0 };
        for part in spec.split('+').map(|p| p.trim().to_ascii_lowercase()) {
            match part.as_str() {
                "ctrl" | "control" => c.ctrl = true,
                "alt" => c.alt = true,
                "shift" => c.shift = true,
                "mouse4" | "xbutton1" | "back" => c.xbutton = 1,
                "mouse5" | "xbutton2" | "forward" => c.xbutton = 2,
                _ => return None,
            }
        }
        if c.xbutton == 0 {
            None
        } else {
            Some(c)
        }
    }

    pub fn set(combo: Option<Combo>) {
        if let Ok(mut g) = COMBO.lock() {
            *g = combo;
        }
    }

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && wparam as u32 == WM_XBUTTONDOWN {
            let info = &*(lparam as *const MSLLHOOKSTRUCT);
            let xb = ((info.mouseData >> 16) & 0xffff) as u16;
            let combo = COMBO.lock().ok().and_then(|g| *g);
            if let Some(c) = combo {
                if xb == c.xbutton
                    && key_down(VK_CONTROL) == c.ctrl
                    && key_down(VK_MENU) == c.alt
                    && key_down(VK_SHIFT) == c.shift
                {
                    if let Some(app) = APP.get() {
                        let app = app.clone();
                        let inner = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Some(w) = inner.get_webview_window("main") {
                                super::toggle_window(&w);
                            }
                        });
                    }
                    return 1;
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    /// Install the hook on a dedicated thread (low-level hooks need a message loop).
    pub fn install(app: tauri::AppHandle) {
        let _ = APP.set(app);
        std::thread::spawn(|| unsafe {
            let h = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), std::ptr::null_mut(), 0);
            if h.is_null() {
                eprintln!("mouse summon: hook failed");
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
        });
    }
}

/// Bind a mouse-button summon combo such as "Ctrl+Mouse5"; empty clears it.
#[tauri::command]
fn set_mouse_summon(spec: Option<String>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match spec.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => {
                mouse_summon::set(None);
                Ok(())
            }
            Some(s) => match mouse_summon::parse(s) {
                Some(c) => {
                    mouse_summon::set(Some(c));
                    Ok(())
                }
                None => Err(format!("bad mouse summon '{s}' (use e.g. Ctrl+Mouse5)")),
            },
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = spec;
        Err("mouse summon is only available on Windows".into())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(HotkeyState::default())
        .manage(StatsMutex::new(StatsState {
            sys: sysinfo::System::new(),
            nets: sysinfo::Networks::new_with_refreshed_list(),
            last: std::time::Instant::now(),
        }))
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
                        // Off the event loop: audio switch / paste block for a moment.
                        std::thread::spawn(move || {
                            let _ = perform(&b.action);
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
            get_hostname,
            set_mouse_summon,
            fire,
            set_clipboard,
            set_summon_monitor,
            list_commands,
            get_stats,
            notify,
            set_clipboard_watch,
            clipboard_history,
            clipboard_clear,
            clipboard_pick,
            search_files,
            reveal,
            list_bookmarks,
            voice_config,
            say,
            listen,
            set_voice_grammar,
            ai_config,
            ai_ask,
            capture_screen,
            pick_file,
            doctor
        ])
        .setup(|app| {
            let _ = APP.set(app.handle().clone());
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
            let commands_i = MenuItem::with_id(app, "commands", "Open commands folder", true, None::<&str>)?;
            let reload_i = MenuItem::with_id(app, "reload", "Reload config", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Fay", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &config_i, &commands_i, &reload_i, &quit_i])?;

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
                    "commands" => open_commands_folder(app),
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

            // Mouse-button summon hook (armed by the frontend from config).
            #[cfg(target_os = "windows")]
            mouse_summon::install(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Fay");
}

// ---- unit tests for the pure helpers (run with `cargo test`) -----------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_env_replaces_known_vars_and_keeps_unknown() {
        std::env::set_var("FAY_TEST_HOME", "C:\\Users\\me");
        assert_eq!(expand_env("%FAY_TEST_HOME%\\Desktop\\x.lnk"), "C:\\Users\\me\\Desktop\\x.lnk");
        assert_eq!(expand_env("%FAY_NOPE_UNSET%\\a"), "%FAY_NOPE_UNSET%\\a");
        assert_eq!(expand_env("100% sure"), "100% sure");
        assert_eq!(expand_env("no vars"), "no vars");
    }

    #[test]
    fn url_encode_is_ascii_safe_and_reversible_by_powershell_rules() {
        assert_eq!(url_encode("hey Fay"), "hey%20Fay");
        assert_eq!(url_encode("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(url_encode("ü\n"), "%C3%BC%0A");
    }

    #[test]
    fn base64_roundtrip() {
        for s in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0u8, 255, 16, 7]] {
            assert_eq!(b64_decode(&b64(s)).unwrap(), s.to_vec());
        }
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
        assert!(b64_decode("not base64!").is_err());
    }

    #[test]
    fn powershell_encoded_command_is_utf16le_base64() {
        // "A" as UTF-16LE is 0x41 0x00 → "QQA="
        let utf16: Vec<u8> = "A".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(b64(&utf16), "QQA=");
    }

    #[test]
    fn mozlz4_backup_is_parsed() {
        let json = br#"{"children":[{"title":"Fay","uri":"https://github.com/Ariestotele/Fay"},{"title":"sep","type":"text/x-moz-place-separator"},{"children":[{"title":"Tauri","uri":"https://tauri.app"}]}]}"#;
        let mut file = b"mozLz40\0".to_vec();
        file.extend_from_slice(&(json.len() as u32).to_le_bytes());
        file.extend_from_slice(&lz4_flex::block::compress(json));
        let dir = std::env::temp_dir().join(format!("fay-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("bookmarks-2026-09-18_3_abc.jsonlz4");
        std::fs::write(&p, &file).unwrap();
        let v = read_mozlz4(&p).expect("parses");
        let mut out = Vec::new();
        collect_moz(&v, &mut out, "Zen");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].title, "Fay");
        assert_eq!(out[1].url, "https://tauri.app");
        assert_eq!(out[1].browser, "Zen");
        assert!(read_mozlz4(std::path::Path::new("/definitely/missing.jsonlz4")).is_none());
    }

    #[test]
    fn chromium_bookmarks_are_walked() {
        let v: serde_json::Value = serde_json::from_str(r#"{"roots":{"bookmark_bar":{"children":[{"type":"url","name":"A","url":"https://a.test"},{"type":"folder","children":[{"type":"url","name":"B","url":"https://b.test"}]}]}}}"#).unwrap();
        let mut out = Vec::new();
        for (_, root) in v["roots"].as_object().unwrap() {
            collect_chromium(root, &mut out, "Chrome");
        }
        assert_eq!(out.iter().map(|b| b.title.as_str()).collect::<Vec<_>>(), ["A", "B"]);
    }

    #[test]
    fn tile_action_deserializes_camel_case_and_defaults() {
        let a: TileAction = serde_json::from_str(r#"{"kind":"multi","steps":[{"kind":"media","action":"playpause"},{"kind":"wait","wait":400},{"target":"x.exe","audioOut":"Speakers"}]}"#).unwrap();
        assert_eq!(a.kind, "multi");
        assert_eq!(a.steps.len(), 3);
        assert_eq!(a.steps[1].wait, Some(400));
        assert_eq!(a.steps[2].audio_out.as_deref(), Some("Speakers"));
        assert!(a.steps[2].paste.is_none());
        let b: HotkeyBinding = serde_json::from_str(r#"{"accelerator":"Ctrl+Alt+F","kind":"focus","minutes":25}"#).unwrap();
        assert_eq!(b.accelerator, "Ctrl+Alt+F");
        assert_eq!(b.action.minutes, Some(25));
    }

    #[test]
    fn perform_multi_stops_at_first_failing_step() {
        // A launch step with no target fails; the error names the step number.
        let a = TileAction { kind: "multi".into(), steps: vec![TileAction { kind: "wait".into(), wait: Some(1), ..Default::default() }, TileAction::default()], ..Default::default() };
        let err = perform(&a).unwrap_err();
        assert!(err.starts_with("step 2:"), "{err}");
    }

    #[test]
    fn perform_wait_caps_at_sixty_seconds_and_returns_ok() {
        let a = TileAction { kind: "wait".into(), wait: Some(1), ..Default::default() };
        assert_eq!(perform(&a).unwrap(), None);
    }

    #[test]
    fn doctor_classifies_targets() {
        assert_eq!(check_target("steam://open/main").0, None);
        assert_eq!(check_target("ms-settings:sound").0, None);
        assert_eq!(check_target("https://tauri.app/x?y=1").0, None);
        assert_eq!(check_target("").0, Some(false));
        let here = std::env::current_dir().unwrap();
        let (ok, detail) = check_target(&here.display().to_string());
        assert_eq!(ok, Some(true), "{detail}");
        let missing = here.join("definitely-not-here.exe").display().to_string();
        let (ok, detail) = check_target(&missing);
        assert_eq!(ok, Some(false));
        assert!(detail.starts_with("not found:"), "{detail}");
        assert_eq!(check_target("%FAY_UNSET_VAR_X%\\a.exe").0, Some(false));
    }

    #[test]
    fn app_name_matching_prefers_exact_and_skips_uninstallers() {
        assert_eq!(match_score("Zen", "zen"), Some(0));
        assert_eq!(match_score("Zen Browser", "Zen"), Some(1));
        assert_eq!(match_score("My Zen Thing", "zen"), Some(2));
        assert_eq!(match_score("Uninstall Zen", "zen"), None);
        assert_eq!(match_score("Firefox", "zen"), None);
        assert_eq!(match_score("Task Manager", "task manager"), Some(0));
        assert_eq!(match_score("anything", ""), None);
    }

    #[test]
    fn walk_for_finds_the_best_match_within_budget() {
        let dir = std::env::temp_dir().join(format!("fay-walk-{}", std::process::id()));
        let nested = dir.join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("Zen Browser.lnk"), b"x").unwrap();
        std::fs::write(nested.join("Uninstall Zen.lnk"), b"x").unwrap();
        std::fs::write(nested.join("notes.txt"), b"x").unwrap();
        let mut budget = 1000u32;
        let mut best = None;
        walk_for(&dir, "zen", &["lnk"], 4, &mut budget, &mut best);
        assert!(best.as_ref().unwrap().1.ends_with("Zen Browser.lnk"), "{best:?}");
        // The exact file wins over the prefix match, whatever the walk order.
        std::fs::write(nested.join("Zen.lnk"), b"x").unwrap();
        let (mut budget, mut best) = (1000u32, None);
        walk_for(&dir, "zen", &["lnk"], 4, &mut budget, &mut best);
        assert_eq!(best.as_ref().unwrap().0, 0);
        // Depth 1 never descends into the subfolder.
        let (mut budget, mut best) = (1000u32, None);
        walk_for(&dir, "zen", &["lnk"], 1, &mut budget, &mut best);
        assert!(best.is_none());
        // A spent budget stops the walk.
        let (mut budget, mut best) = (0u32, None);
        walk_for(&dir, "zen", &["lnk"], 4, &mut budget, &mut best);
        assert!(best.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn mouse_summon_parse() {
        let c = mouse_summon::parse("Ctrl+Mouse5").unwrap();
        assert!(c.ctrl && !c.alt && c.xbutton == 2);
        assert!(mouse_summon::parse("Ctrl+F").is_none());
        assert!(mouse_summon::parse("Mouse4").unwrap().xbutton == 1);
    }
}

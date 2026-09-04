use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use tauri::{
    AppHandle, Manager, RunEvent, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

/// The running `dsh` server child, if any. Killing must go through
/// [`kill_server`] so the whole process tree dies with it.
struct ServerState {
    child: Mutex<Option<Child>>,
    url: Mutex<Option<String>>,
}

/// User-editable launch configuration (`%APPDATA%/dsh-desk/config.json`).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeskConfig {
    command: String,
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self {
            command: "dsh".into(),
            args: vec!["--profile".into(), "web".into(), "--no-open".into()],
            cwd: None,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|appdata| PathBuf::from(appdata).join("dsh-desk").join("config.json"))
}

fn log_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|appdata| PathBuf::from(appdata).join("dsh-desk").join("dsh-desk.log"))
}

/// Append one line to the log file. The release build is a GUI-subsystem
/// binary with no console, so the file is the only durable diagnostic sink;
/// when a console is attached (`tauri dev`), the line goes there too.
fn log_line(line: &str) {
    println!("{line}");
    eprintln!("{line}");
    if let Some(path) = log_path() {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn load_config() -> DeskConfig {
    let Some(path) = config_path() else {
        return DeskConfig::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<DeskConfig>(&text) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("dsh-desk: config.json is invalid ({error}); using defaults");
                DeskConfig::default()
            }
        },
        Err(_) => {
            let config = DeskConfig::default();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
                let _ = std::fs::write(
                    &path,
                    serde_json::to_string_pretty(&config).unwrap_or_default() + "\n",
                );
            }
            config
        }
    }
}

/// Kill the child process tree. `Child::kill` only reaches the immediate
/// process; on Windows `pnpm` is a shell shim whose node child survives, so
/// `taskkill /T /F` takes the tree. The fallback keeps other platforms sane.
fn kill_server(state: &ServerState) {
    let mut guard = state.child.lock().unwrap();
    if let Some(mut child) = guard.take() {
        let pid = child.id();
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status();
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    *state.url.lock().unwrap() = None;
}

/// Spawn the configured `dsh` command, watch its stdout for the readiness
/// line (`dsh web: <authenticated-url>`, optionally followed by an LAN URL),
/// then navigate the main window to it. Every stdout/stderr line is mirrored
/// to this process's console for diagnosis.
fn spawn_server(app: AppHandle) {
    let config = load_config();
    // Windows: a `.cmd`/`.bat` shim (installed dsh, pnpm) cannot be executed
    // by std::process directly; route through cmd.exe when the direct spawn
    // refuses. A real executable (node.exe) spawns without the detour.
    let mut build = |via_cmd: bool| {
        let mut command = if via_cmd && cfg!(windows) {
            let mut command = Command::new("cmd.exe");
            command.arg("/C").arg(&config.command);
            command
        } else {
            Command::new(&config.command)
        };
        command
            .args(&config.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &config.cwd {
            command.current_dir(cwd);
        }
        command
    };
    let spawned = build(false).spawn().or_else(|error| {
        if cfg!(windows)
            && matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput
            )
        {
            build(true).spawn()
        } else {
            Err(error)
        }
    });
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            let message = format!(
                "dsh-desk: cannot start `{} {}`: {error}",
                config.command,
                config.args.join(" ")
            );
            log_line(&message);
            show_message(&app, &message);
            return;
        }
    };
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let state = app.state::<ServerState>();
    *state.child.lock().unwrap() = Some(child);
    log_line(&format!(
        "dsh-desk: started `{} {}` (pid {pid})",
        config.command,
        config.args.join(" ")
    ));

    if let Some(stdout) = stdout {
        let app_out = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                log_line(&line);
                if let Some(url) = line.strip_prefix("dsh web: ") {
                    // The authenticated URL runs to the first space; an LAN
                    // variant may follow in parentheses.
                    let url = url.split(' ').next().unwrap_or("").to_string();
                    if url.starts_with("http") {
                        *app_out.state::<ServerState>().url.lock().unwrap() = Some(url.clone());
                        open_gui(&app_out, &url);
                    }
                }
            }
            // stdout closed: the server exited.
            server_exited(&app_out, pid);
        });
    }
    if let Some(stderr) = stderr {
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                log_line(&line);
            }
        });
    }
}

/// Report a dead server: log and tell the window.
fn server_exited(app: &AppHandle, pid: u32) {
    let state = app.state::<ServerState>();
    let had_url = state.url.lock().unwrap().take().is_some();
    state.child.lock().unwrap().take();
    let message = if had_url {
        format!("dsh-desk: the dsh server (pid {pid}) exited")
    } else {
        format!(
            "dsh-desk: the dsh server (pid {pid}) exited before printing its URL — \
             see %APPDATA%/dsh-desk/dsh-desk.log and the launch command in config.json"
        )
    };
    log_line(&message);
    show_message(app, &message);
}

/// Replace the window body with a plain diagnostic message.
fn show_message(app: &AppHandle, message: &str) {
    let script = format!(
        "document.body.innerHTML = ''; \
         document.body.style.cssText = 'font: 14px system-ui; padding: 24px; color: #333; white-space: pre-wrap;'; \
         document.body.textContent = {};",
        serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into())
    );
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.eval(&script);
    }
}

/// Navigate the main window to the authenticated GUI URL.
fn open_gui(app: &AppHandle, url: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let script = format!(
            "window.location.replace({});",
            serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into())
        );
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.eval(&script);
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .manage(ServerState {
            child: Mutex::new(None),
            url: Mutex::new(None),
        })
        .setup(|app| {
            // ── tray ─────────────────────────────────────────────────────────
            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let open_browser =
                MenuItem::with_id(app, "open-browser", "Open in browser", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "Restart server", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit (stop server)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &open_browser, &restart, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("DSH Desk")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app),
                    "open-browser" => {
                        if let Some(url) = app.state::<ServerState>().url.lock().unwrap().clone() {
                            let _ = app.opener().open_url(url, None::<&str>);
                        }
                    }
                    "restart" => {
                        let state = app.state::<ServerState>();
                        kill_server(&state);
                        let app = app.clone();
                        std::thread::spawn(move || spawn_server(app));
                    }
                    "quit" => {
                        let state = app.state::<ServerState>();
                        kill_server(&state);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;
            // ── global hotkey: Alt+Shift+D toggles the window ────────────────
            let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyD);
            app.global_shortcut().register(shortcut)?;
            // ── boot the server ──────────────────────────────────────────────
            let app = app.handle().clone();
            std::thread::spawn(move || spawn_server(app));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides to tray; the tray's Quit is the real exit.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                let state = app.state::<ServerState>();
                kill_server(&state);
            }
        });
}

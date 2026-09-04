use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Slow-start hint and give-up thresholds for the `dsh web:` readiness line.
/// The timeout must not kill the server: the watchers stay alive and a late
/// readiness line still navigates the window out of the degraded state.
const READY_HINT_AFTER: Duration = Duration::from_secs(15);
const READY_TIMEOUT_AFTER: Duration = Duration::from_secs(90);

/// Minimum WebView2 runtime (Chromium) major version the served GUI needs:
/// `Promise.withResolvers` ships in 119. Keep in sync with scripts/env-check.ps1.
const WEBVIEW2_MIN_MAJOR: u32 = 119;
/// Registry locations of the Evergreen runtime's `pv` version value, in the
/// WebView2 loader's resolution order: the per-user install wins over
/// per-machine, so the first key that answers speaks for the runtime that
/// would actually load. reg.exe is spawned rather than pulling a registry
/// crate for one read.
const WEBVIEW2_REGISTRY_KEYS: [&str; 2] = [
    r"HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
];
/// The dsh project home — the README's canonical link. The guide points here
/// instead of naming an install command, which drifts; the repo is the stable
/// surface (same bet as the readiness-line contract).
const DSH_PROJECT_URL: &str = "https://github.com/deepseek-ai/deepseek-harness";
/// The x64 standalone installer — deliberately NOT the bootstrapper
/// (fwlink 2124703): it refuses to install over an existing runtime
/// (0x80040828), which is precisely the machines this guide targets
/// (docs/postmortem-2026-09-04-webview2-114.md).
const WEBVIEW2_DOWNLOAD_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2124701";

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, RunEvent, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

/// The running `dsh` server child, if any. Killing must go through
/// [`kill_registered_child`] so the whole process tree dies with it.
struct ServerState {
    child: Mutex<Option<Child>>,
    url: Mutex<Option<String>>,
    /// Monotonic lifecycle generation. Every spawn attempt, every deliberate
    /// kill, and a reported exit (which retires that generation's timers)
    /// mints a new one; the stdout watchers and the
    /// hint/timeout timers capture the generation they were born under and
    /// may touch state only while it is still current. This is what keeps a
    /// superseded watcher (its server killed by Restart, pipe still draining)
    /// from stealing the new server's url/child when it reaches EOF — the G2
    /// race S2 exists to close.
    generation: AtomicU64,
    /// Serializes the whole kill → wait → spawn cycle. Held across `spawn()`
    /// on purpose — that is what makes "never spawn before the old child is
    /// dead" true — but only on lifecycle worker threads; the tray/main
    /// thread waits on it solely in the Quit path, where it may block for at
    /// most one in-flight cycle (taskkill tree + reap). While holding it,
    /// never make a call that blocks on the main thread: window.eval/show
    /// are fire-and-forget only as long as tauri's `tracing` feature stays
    /// off, which would turn eval into a synchronous wait on that thread.
    lifecycle: Mutex<()>,
    /// Set once the app is exiting. Lifecycle workers re-check it inside the
    /// critical section so a queued Restart cannot resurrect a server after
    /// the final Quit kill.
    exiting: AtomicBool,
}

impl ServerState {
    fn new() -> Self {
        Self {
            child: Mutex::new(None),
            url: Mutex::new(None),
            generation: AtomicU64::new(0),
            lifecycle: Mutex::new(()),
            exiting: AtomicBool::new(false),
        }
    }

    fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Mint the next generation for a lifecycle attempt and hand it to every
    /// watcher/timer created for that attempt.
    fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_exiting(&self) -> bool {
        self.exiting.load(Ordering::SeqCst)
    }
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

/// Whether the configured launch command plausibly resolves. A path (it has a
/// separator) is taken at face value; a bare name must be found by where/which
/// — the same PATH (and, via where, the same .cmd/.bat PATHEXT resolution)
/// the spawn's cmd.exe fallback would search. A probe that cannot run at all
/// fails open: the spawn attempt itself stays the source of truth.
fn command_locatable(command: &str) -> bool {
    if command.contains('/') || command.contains('\\') {
        return true;
    }
    #[cfg(windows)]
    let (probe, arg) = ("where.exe", command);
    #[cfg(not(windows))]
    let (probe, arg) = ("which", command);
    Command::new(probe)
        .arg(arg)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(true)
}

/// Pull the value out of `reg query <key> /v pv` output: the whitespace-
/// delimited token following the `REG_SZ` type marker on its line.
fn parse_reg_sz_value(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains("REG_SZ"))?;
    line.split_whitespace()
        .skip_while(|token| *token != "REG_SZ")
        .nth(1)
        .map(str::to_string)
}

/// Whether a registered runtime version (`pv`, e.g. `114.0.1823.43`) is too
/// old for the GUI. An unparseable version also blocks: a machine whose `pv`
/// is not even a version string cannot be trusted to run the GUI.
fn runtime_pv_blocks(pv: &str) -> bool {
    !matches!(
        pv.split('.').next().and_then(|m| m.parse::<u32>().ok()),
        Some(major) if major >= WEBVIEW2_MIN_MAJOR
    )
}

/// The runtime version that would actually load, when it cannot run the GUI:
/// older than the baseline, or registered but unparseable. `None` = the
/// runtime is fine — or none is registered at all, in which case the window
/// itself could not have been created and S12's installer bootstrapper owns
/// it. Only the first registry location that answers is evaluated (per-user
/// before per-machine): the machine-level runtime is irrelevant when a
/// per-user install shadows it.
fn webview2_too_old() -> Option<String> {
    for key in WEBVIEW2_REGISTRY_KEYS {
        let Ok(output) = Command::new("reg.exe")
            .args(["query", key, "/v", "pv"])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let Some(pv) = parse_reg_sz_value(&String::from_utf8_lossy(&output.stdout)) else {
            // The key answers but its value is not a readable string: a
            // runtime we cannot version is one we cannot trust.
            return Some("<unreadable>".into());
        };
        return if runtime_pv_blocks(&pv) {
            Some(pv)
        } else {
            None
        };
    }
    None
}

/// Take the child down with its whole tree. `Child::kill` only reaches the
/// immediate process; on Windows `pnpm` is a shell shim whose node child
/// survives, so `taskkill /T /F` takes the tree. The fallback keeps other
/// platforms sane. A child that already exited on its own is only reaped —
/// taskkill by pid on a possibly-recycled pid could hit an unrelated tree.
fn kill_child_tree(child: &mut Child) {
    if matches!(child.try_wait(), Ok(Some(_))) {
        let _ = child.wait();
        return;
    }
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

/// Kill the registered child process tree and retire its generation. Must
/// run with `lifecycle` held so no spawn is in flight. The generation bump
/// happens under the child lock, together with the take: a watcher
/// concurrently claiming an exit either sees the already-bumped generation
/// (and stays quiet) or holds the lock first and hands the Child over —
/// there is no ordering in which the Child vanishes unclaimed while a kill
/// believes it is still registered. Retiring the generation also silences
/// the dying child's watchers and timers, so the EOF their pipes deliver
/// next is not reported as an unexpected exit.
fn kill_registered_child(state: &ServerState) {
    let mut guard = state.child.lock().unwrap();
    state.generation.fetch_add(1, Ordering::SeqCst);
    if let Some(mut child) = guard.take() {
        drop(guard);
        kill_child_tree(&mut child);
    }
    *state.url.lock().unwrap() = None;
}

/// Extract the authenticated URL from a `dsh web:` stdout line: split on the
/// literal marker, then take the first whitespace-delimited token that starts
/// with `http` — tolerant to wording/space drift around the marker while
/// still anchored to the one stable surface dsh-desk depends on.
fn extract_ready_url(line: &str) -> Option<String> {
    let after = line.split_once("dsh web:")?.1;
    after
        .split_whitespace()
        .find(|token| token.starts_with("http"))
        .map(str::to_string)
}

/// A copy of `line` with every http(s) URL cut down to scheme://host:port.
/// dsh's readiness URL carries the auth token in its path/query and stdout is
/// mirrored to dsh-desk.log, which users are told to attach to issues — once
/// a token lands in that file it cannot be recalled, so anything past the
/// address is dropped before the line reaches any sink.
fn redact_urls(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(pos) = rest.find("http") {
        let (before, from) = rest.split_at(pos);
        out.push_str(before);
        let tail = from
            .strip_prefix("https://")
            .or_else(|| from.strip_prefix("http://"));
        if let Some(tail) = tail {
            out.push_str(&from[..from.len() - tail.len()]);
            // The authority (host[:port]) runs until the first character that
            // cannot belong to one; the rest of the whitespace-delimited
            // token is the token-bearing path/query.
            let authority_end = tail
                .find(|c: char| {
                    !matches!(
                        c,
                        'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | ':' | '[' | ']' | '_'
                    )
                })
                .unwrap_or(tail.len());
            out.push_str(&tail[..authority_end]);
            let remainder = &tail[authority_end..];
            let token_end = remainder
                .find(char::is_whitespace)
                .unwrap_or(remainder.len());
            if token_end > 0 {
                out.push('…');
            }
            rest = &remainder[token_end..];
        } else {
            out.push_str("http");
            rest = &from["http".len()..];
        }
    }
    out.push_str(rest);
    out
}

/// True while generation `gen` is still current and has not printed its
/// readiness line — the only condition under which the hint and timeout
/// timers may act. A Restart mints a newer generation and a reported exit
/// retires the current one; in both cases the superseded timer goes quiet
/// instead of overwriting the new attempt's state.
fn is_still_starting(state: &ServerState, gen: u64) -> bool {
    state.current_generation() == gen && state.url.lock().unwrap().is_none()
}

/// Spawn the configured `dsh` command, watch its stdout for the readiness
/// line (`dsh web: <authenticated-url>`, optionally followed by an LAN URL),
/// then navigate the main window to it. Every stdout/stderr line is mirrored
/// to this process's console for diagnosis.
///
/// Must run with `lifecycle` held: that is what makes spawn→register atomic
/// with respect to kills — no kill can land in the gap between `spawn()`
/// returning and the child landing in `state.child`. The child-field lock
/// itself is taken only for the register, never across `spawn()` (the
/// cmd.exe fallback can be slow, and kill is called synchronously from the
/// tray thread).
fn spawn_server(app: &AppHandle) {
    let config = load_config();
    // S4 runtime gate: an old-but-installed WebView2 leaves the window alive
    // but the GUI permanently broken, so this belongs on every spawn path —
    // boot, Retry, and the tray's Restart alike. It sits after load_config so
    // a first run still writes config.json for the tray's Edit config.
    if let Some(pv) = webview2_too_old() {
        log_line(&format!(
            "dsh-desk: WebView2 runtime {pv} is older than Chromium \
             {WEBVIEW2_MIN_MAJOR} — showing the runtime guide"
        ));
        show_runtime_old(app, &pv);
        return;
    }
    // S4 install guide: with an unlocatable launch command the spawn below can
    // only fail with a raw error — walk the user in instead. On a true first
    // run (no dsh installed) this is the onboarding spec'd in S4; later runs
    // with dsh still missing land in the same, more accurate place.
    if !command_locatable(&config.command) {
        let message = format!(
            "dsh-desk: launch command `{}` not found on PATH — showing the install guide",
            config.command
        );
        log_line(&message);
        show_onboarding(&app);
        return;
    }
    let state = app.state::<ServerState>();
    let gen = state.next_generation();
    // Windows: a `.cmd`/`.bat` shim (installed dsh, pnpm) cannot be executed
    // by std::process directly; route through cmd.exe when the direct spawn
    // refuses. A real executable (node.exe) spawns without the detour.
    let build = |via_cmd: bool| {
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
            // The page here is index.html (boot, or a Restart that just ran
            // deskReset) — possibly still loading at boot, so wait for it.
            show_degraded(&app, &message, true);
            return;
        }
    };
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *state.child.lock().unwrap() = Some(child);
    log_line(&format!(
        "dsh-desk: started `{} {}` (pid {pid})",
        config.command,
        config.args.join(" ")
    ));

    if let Some(stdout) = stdout {
        let app_out = app.clone();
        std::thread::spawn(move || {
            let state = app_out.state::<ServerState>();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                log_line(&redact_urls(&line));
                if let Some(url) = extract_ready_url(&line) {
                    // A readiness line from a superseded generation belongs
                    // to a killed server: mirroring it to the log is fine,
                    // navigating to it is not. The generation is re-read
                    // inside the url lock so a kill that retired this
                    // generation between the check and the write also wins
                    // the race for the slot (its clear takes the same lock)
                    // — the dead URL cannot be written back.
                    let mine = {
                        let mut slot = state.url.lock().unwrap();
                        let current = state.current_generation() == gen;
                        if current {
                            *slot = Some(url.clone());
                        }
                        current
                    };
                    if mine {
                        open_gui(&app_out, &url);
                    }
                }
            }
            // stdout closed: the server exited — but only this generation's
            // watcher may report it.
            server_exited(&app_out, pid, gen);
        });
    }
    if let Some(stderr) = stderr {
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                log_line(&redact_urls(&line));
            }
        });
    }

    // Slow-start hint, then the actionable degraded panel. Both act only
    // while this spawn's generation is still current and has not reported
    // ready; the server is never killed on timeout, so a late readiness line
    // can still bring the window back. Every spawn starts a fresh timer
    // pair; earlier pairs are invalidated by the generation check.
    let app_timers = app.clone();
    std::thread::spawn(move || {
        let state = app_timers.state::<ServerState>();
        std::thread::sleep(READY_HINT_AFTER);
        if is_still_starting(&state, gen) {
            log_line("dsh-desk: no readiness line yet, showing the slow-start hint");
            show_hint(&app_timers);
        }
        std::thread::sleep(READY_TIMEOUT_AFTER - READY_HINT_AFTER);
        // Re-check the generation right before acting: a Restart landing
        // between is_still_starting and the panel would otherwise paint the
        // degraded state over the new attempt's loading page (the restart's
        // deskReset clears it, but the flash is avoidable).
        if is_still_starting(&state, gen) && state.current_generation() == gen {
            let message = format!(
                "dsh-desk: no readiness line after {}s — the server is still running and was \
                 NOT killed. It may just be slow (a late start is picked up automatically), or \
                 `dsh web` changed its output wording. The log shows everything it printed; \
                 config.json controls how it is launched.",
                READY_TIMEOUT_AFTER.as_secs()
            );
            log_line(&message);
            // 90s in, index.html has long finished loading; no wait needed.
            show_degraded(&app_timers, &message, false);
        }
    });
}

/// Claim the server-exited transition for generation `gen`: when this
/// watcher is still current, take the url AND the child out of the state in
/// one critical section (together with the generation bump that retires the
/// timers) and hand both to the caller; `None` when a newer spawn has
/// superseded it — the stale watcher must then exit quietly, leaving the
/// new server's url/child alone. The child handoff is part of the claim: a
/// kill arriving concurrently must never find the child gone without an
/// owner responsible for its tree.
fn claim_exit(state: &ServerState, gen: u64) -> Option<(Option<Child>, bool)> {
    let mut child = state.child.lock().unwrap();
    if state.current_generation() != gen {
        return None;
    }
    state.generation.fetch_add(1, Ordering::SeqCst);
    let had_url = state.url.lock().unwrap().take().is_some();
    Some((child.take(), had_url))
}

/// Report a dead server: log and tell the window. EOF is only an assumption
/// that the whole tree died — if the process is somehow still alive (stdout
/// closed without exit, shell-shim semantics), the tree is killed here
/// while the handle is still owned; nothing else will.
fn server_exited(app: &AppHandle, pid: u32, gen: u64) {
    let state = app.state::<ServerState>();
    let Some((child, had_url)) = claim_exit(&state, gen) else {
        log_line(&format!(
            "dsh-desk: stdout of pid {pid} closed on a superseded generation \
             ({gen}); not the current server, ignoring"
        ));
        return;
    };
    if let Some(mut child) = child {
        kill_child_tree(&mut child);
    }
    let message = if had_url {
        format!("dsh-desk: the dsh server (pid {pid}) exited")
    } else {
        format!(
            "dsh-desk: the dsh server (pid {pid}) exited before printing its URL — \
             see %APPDATA%/dsh-desk/dsh-desk.log and the launch command in config.json"
        )
    };
    log_line(&message);
    // Died before the readiness line → the page is still index.html (maybe
    // still loading): wait for the helper. Died after → the page is the
    // remote dsh GUI where no helper exists: plain text at once.
    show_degraded(app, &message, !had_url);
}

/// Unhide the window with the slow-start hint on the loading page. No
/// `set_focus`: the hint is passive and must not yank focus while it appears.
fn show_hint(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.eval("if (typeof deskShowHint === 'function') deskShowHint();");
    }
}

/// Unhide the window and swap in a full-page panel by calling its helper in
/// src/index.html (the degraded, install-guide and runtime-too-old panels all
/// work this way). When the current page is the remote dsh GUI the helpers do
/// not exist there, so a plain-text fallback keeps the message visible.
///
/// `wait` is for panels raised during boot, which race the initial page load:
/// until index.html has run, the helper is undefined and the script would
/// fall back to buttonless plain text — so retry briefly for the helper to
/// appear first and only fall back after that.
fn show_panel(app: &AppHandle, helper: &str, message: &str, wait: bool) {
    let message = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());
    let fallback = format!(
        "document.body.innerHTML = '', \
         document.body.style.cssText = 'font: 14px system-ui; padding: 24px; color: #333; white-space: pre-wrap;', \
         document.body.textContent = {message}"
    );
    let script = if wait {
        format!(
            "(function () {{ var n = 0; \
              function go() {{ if (typeof {helper} === 'function') {helper}({message}); \
                else if (n++ < 25) setTimeout(go, 200); \
                else ({fallback}); }} \
              go(); }})();"
        )
    } else {
        format!(
            "if (typeof {helper} === 'function') {helper}({message}); \
             else ({fallback});"
        )
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.eval(&script);
    }
}

/// The degraded panel: the explanation plus the open-log / open-config /
/// retry actions defined in src/index.html. `wait` follows the same rule as
/// the other panels: sources that fire while the page is (still) index.html
/// wait for the helper — boot races the initial page load — while sources
/// that can fire on the remote dsh GUI page fall back to plain text at once.
fn show_degraded(app: &AppHandle, message: &str, wait: bool) {
    show_panel(app, "deskShowDegraded", message, wait);
}

/// The install guide (S4): nothing to launch — dsh is not on PATH and the
/// config does not point elsewhere yet. Shares the degraded panel's action
/// mechanism (same invoke/ACL path).
fn show_onboarding(app: &AppHandle) {
    let config_path = config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "%APPDATA%/dsh-desk/config.json".into());
    let message = format!(
        "Nothing to launch yet — dsh was not found on this machine.\n\
         \n\
         · Install dsh, then press Retry. The button below opens the project \
         page with its install instructions.\n\
         · Running dsh from a source checkout? Open the config file and point \
         `command` / `args` / `cwd` at your checkout, then press Retry.\n\
         \n\
         Config file: {config_path}"
    );
    show_panel(app, "deskShowOnboarding", &message, true);
}

/// The runtime-too-old guide (S4/G12): the GUI cannot work on this WebView2
/// runtime no matter how healthy the server is. Offer the standalone
/// installer (see WEBVIEW2_DOWNLOAD_URL) and a way back once it is updated.
fn show_runtime_old(app: &AppHandle, pv: &str) {
    let message = format!(
        "The WebView2 Runtime on this machine is {pv} (Chromium {pv_major}), but \
         the dsh web GUI needs Chromium {WEBVIEW2_MIN_MAJOR} or newer — the window \
         would open yet the GUI would stay broken.\n\
         \n\
         Download and run the x64 standalone installer from the button below, \
         then press Retry.",
        pv_major = pv.split('.').next().unwrap_or(pv),
    );
    show_panel(app, "deskShowRuntimeOld", &message, true);
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

/// One serialized lifecycle cycle: kill the process tree, wait for it to
/// die, reset the loading page, then spawn again. Runs on a worker thread so
/// the caller (tray menu, degraded-panel Retry) never blocks on
/// taskkill/spawn; rapid Restart clicks queue up behind the `lifecycle`
/// lock instead of interleaving kills and spawns — each queued cycle kills
/// the child the previous cycle registered, so nothing leaks.
fn run_lifecycle_cycle(app: &AppHandle, restart: bool) {
    let state = app.state::<ServerState>();
    if state.is_exiting() {
        return;
    }
    let _guard = state.lifecycle.lock().unwrap();
    // Re-check under the lock: Quit may have won the race while we waited.
    if state.is_exiting() {
        return;
    }
    if restart {
        kill_registered_child(&state);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.eval("if (typeof deskReset === 'function') deskReset();");
        }
    }
    spawn_server(app);
}

/// Restart is the same serial path everywhere: the tray menu and every
/// panel's Retry button land here. Never spawn while the previous child
/// might still be alive.
fn restart_server(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || run_lifecycle_cycle(&app, true));
}

/// Boot-time first spawn: nothing to kill, same serialization.
fn boot_server(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || run_lifecycle_cycle(&app, false));
}

/// Open a file with its system association; bare `.log`/`.json` files often
/// have none on Windows, so fall back to notepad.
fn open_in_viewer(app: &AppHandle, path: &Path) {
    match app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        Ok(()) => log_line(&format!(
            "dsh-desk: opened {} in system viewer",
            path.display()
        )),
        Err(error) => {
            log_line(&format!(
                "dsh-desk: system viewer for {} failed ({error}); falling back to notepad",
                path.display()
            ));
            let _ = Command::new("notepad").arg(path).spawn();
        }
    }
}

#[tauri::command]
fn open_log_file(app: AppHandle) {
    log_line("dsh-desk: open_log_file invoked");
    if let Some(path) = log_path() {
        open_in_viewer(&app, &path);
    }
}

/// Open config.json for editing — shared by the tray's Edit config item and
/// the panels' Open config button.
fn edit_config(app: &AppHandle) {
    log_line("dsh-desk: opening config.json for editing");
    if let Some(path) = config_path() {
        open_in_viewer(app, &path);
    }
}

#[tauri::command]
fn open_config_file(app: AppHandle) {
    edit_config(&app);
}

/// Open a fixed external URL — the guide states link to specific pages, not
/// to arbitrary URLs the loaded page could choose.
fn open_external(app: &AppHandle, url: &str, what: &str) {
    match app.opener().open_url(url, None::<&str>) {
        Ok(()) => log_line(&format!("dsh-desk: opened the {what}")),
        Err(error) => log_line(&format!("dsh-desk: opening the {what} failed: {error}")),
    }
}

#[tauri::command]
fn open_dsh_page(app: AppHandle) {
    log_line("dsh-desk: open_dsh_page invoked");
    open_external(&app, DSH_PROJECT_URL, "dsh project page");
}

#[tauri::command]
fn open_webview2_download(app: AppHandle) {
    log_line("dsh-desk: open_webview2_download invoked");
    open_external(&app, WEBVIEW2_DOWNLOAD_URL, "WebView2 download page");
}

#[tauri::command]
fn retry_server(app: AppHandle) {
    log_line("dsh-desk: retry_server invoked");
    // Fast path for the runtime gate: it can lift without an app restart
    // once the user installs a newer WebView2, and re-showing the guide
    // here skips the pointless kill + deskReset + spawn dance. The gate
    // itself also re-runs inside spawn_server, so every path stays covered.
    if let Some(pv) = webview2_too_old() {
        log_line(&format!(
            "dsh-desk: WebView2 runtime still too old ({pv}) — staying in the guide"
        ));
        show_runtime_old(&app, &pv);
        return;
    }
    restart_server(&app);
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
        .manage(ServerState::new())
        .invoke_handler(tauri::generate_handler![
            open_log_file,
            open_config_file,
            retry_server,
            open_dsh_page,
            open_webview2_download
        ])
        .setup(|app| {
            // ── tray ─────────────────────────────────────────────────────────
            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let open_browser =
                MenuItem::with_id(app, "open-browser", "Open in browser", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "Restart server", true, None::<&str>)?;
            let edit_config_item =
                MenuItem::with_id(app, "edit-config", "Edit config", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit (stop server)", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&show, &open_browser, &restart, &edit_config_item, &quit],
            )?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("DSH Desk")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app),
                    "open-browser" => {
                        let url = app.state::<ServerState>().url.lock().unwrap().clone();
                        match url {
                            Some(url) => match app.opener().open_url(&url, None::<&str>) {
                                Ok(()) => log_line(
                                    "dsh-desk: opened the authenticated URL in the browser",
                                ),
                                Err(error) => {
                                    log_line(&format!("dsh-desk: open in browser failed: {error}"))
                                }
                            },
                            // No URL yet is normal early in a launch — say so
                            // instead of silently doing nothing (S1 boundary).
                            None => log_line(
                                "dsh-desk: open in browser skipped — the server has not \
                                 printed its URL yet",
                            ),
                        }
                    }
                    "restart" => restart_server(app),
                    "edit-config" => edit_config(app),
                    "quit" => {
                        let state = app.state::<ServerState>();
                        state.exiting.store(true, Ordering::SeqCst);
                        // Serialize with any in-flight lifecycle cycle so
                        // this kill cannot miss a child that is mid-spawn;
                        // queued workers see `exiting` and give up quietly.
                        let _guard = state.lifecycle.lock().unwrap();
                        kill_registered_child(&state);
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
            // The S4 gates (WebView2 runtime, install guide) live inside
            // spawn_server, so every path — boot, Retry, tray Restart —
            // passes them.
            boot_server(app.handle());
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
                state.exiting.store(true, Ordering::SeqCst);
                let _guard = state.lifecycle.lock().unwrap();
                kill_registered_child(&state);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_url_tolerates_marker_drift() {
        assert_eq!(
            extract_ready_url(
                "dsh web: http://127.0.0.1:3080/?token=secret (LAN: http://10.0.0.2:3080/?token=secret)"
            ),
            Some("http://127.0.0.1:3080/?token=secret".into())
        );
        // Wording and spacing around the literal marker, leading indentation.
        assert_eq!(
            extract_ready_url("  dsh web: ready → http://127.0.0.1:1234/x"),
            Some("http://127.0.0.1:1234/x".into())
        );
        // Marker present but no URL after it: ignore the line.
        assert_eq!(extract_ready_url("dsh web: not ready yet"), None);
        assert_eq!(extract_ready_url("some unrelated stdout line"), None);
    }

    #[test]
    fn redaction_keeps_only_scheme_host_port() {
        let line = "dsh web: http://127.0.0.1:3080/?token=S3CRET (LAN: http://10.0.0.2:3080/?token=S3CRET)";
        let redacted = redact_urls(line);
        assert!(!redacted.contains("S3CRET"));
        assert_eq!(
            redacted,
            "dsh web: http://127.0.0.1:3080… (LAN: http://10.0.0.2:3080…"
        );
    }

    #[test]
    fn reg_output_yields_pv_value() {
        // Real `reg query /v pv` shape: blank line, key line, value line with
        // the REG_SZ marker between name and value.
        let output = "\r\n\
            HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\r\n\
            \x20   pv    REG_SZ    152.0.1725.45\r\n\
            \r\n";
        assert_eq!(
            parse_reg_sz_value(output),
            Some("152.0.1725.45".into()),
            "the token after the type marker is the value"
        );
    }

    #[test]
    fn reg_output_without_sz_marker_is_rejected() {
        assert_eq!(parse_reg_sz_value("value not found."), None);
        assert_eq!(parse_reg_sz_value(""), None);
        // A REG_SZ line missing its value must not silently yield a token
        // from elsewhere.
        assert_eq!(parse_reg_sz_value("    pv    REG_SZ\r\n"), None);
    }

    #[test]
    fn runtime_gate_blocks_below_baseline_only() {
        assert!(
            runtime_pv_blocks("114.0.1823.43"),
            "the 2026-09-04 incident"
        );
        assert!(runtime_pv_blocks("118.999.999.999"));
        assert!(!runtime_pv_blocks("119.0.0.0"), "baseline itself passes");
        assert!(!runtime_pv_blocks("152.0.1725.45"));
        assert!(
            !runtime_pv_blocks("152"),
            "no dots: the major is the whole pv"
        );
        // Unparseable versions block — see runtime_pv_blocks.
        assert!(runtime_pv_blocks("garbage"));
        assert!(runtime_pv_blocks(""));
    }

    #[test]
    fn redaction_leaves_tokenless_lines_intact() {
        assert_eq!(redact_urls("no urls here"), "no urls here");
        assert_eq!(
            redact_urls("listening on http://127.0.0.1:3080 now"),
            "listening on http://127.0.0.1:3080 now"
        );
        // Token carried in the path, not the query.
        assert_eq!(
            redact_urls("auth: http://127.0.0.1:3080/token/S3CRET123,"),
            "auth: http://127.0.0.1:3080…"
        );
    }

    // ── S2: generation-gated lifecycle ──────────────────────────────────

    #[test]
    fn stale_watcher_eof_leaves_new_server_state_alone() {
        let state = ServerState::new();
        let gen = state.next_generation();
        *state.url.lock().unwrap() = Some("http://127.0.0.1:1/?token=old".into());
        // A Restart happened: the kill retired `gen`, a newer spawn minted
        // its own generation and captured its own URL.
        let _killed = state.next_generation();
        let new_gen = state.next_generation();
        *state.url.lock().unwrap() = Some("http://127.0.0.1:2/?token=new".into());
        // The old watcher drains its pipe and hits EOF after the restart —
        // the exact sequence that used to steal the new server's url/child.
        assert!(claim_exit(&state, gen).is_none());
        assert_eq!(
            state.url.lock().unwrap().as_deref(),
            Some("http://127.0.0.1:2/?token=new")
        );
        // The current watcher's EOF still transitions and retires its timers.
        let (taken, had_url) = claim_exit(&state, new_gen).expect("current generation claims");
        assert!(taken.is_none());
        assert!(had_url);
        assert!(state.url.lock().unwrap().is_none());
        assert!(!is_still_starting(&state, new_gen));
    }

    #[test]
    #[cfg(windows)]
    fn claim_exit_hands_the_child_over_atomically() {
        let state = ServerState::new();
        let gen = state.next_generation();
        // A real (already-exited) process stands in for the dsh child: the
        // claim must take it out of the state together with the url, so a
        // concurrent kill can never find the child gone without an owner.
        let mut child = Command::new("cmd.exe")
            .args(["/C", "exit 0"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("cmd.exe spawns in tests");
        let _ = child.wait();
        *state.child.lock().unwrap() = Some(child);
        let (taken, _) = claim_exit(&state, gen).expect("current generation claims");
        assert!(taken.is_some(), "the child leaves the state with the claim");
        assert!(state.child.lock().unwrap().is_none());
        // A kill arriving after the claim is a no-op, by design: the claimer
        // owns the handle and is responsible for the tree.
        kill_registered_child(&state);
        assert!(state.child.lock().unwrap().is_none());
    }

    #[test]
    fn deliberate_kill_retires_generation_and_clears_url() {
        let state = ServerState::new();
        let gen = state.next_generation();
        *state.url.lock().unwrap() = Some("http://127.0.0.1:9/".into());
        // No child registered: exercises the state effects alone.
        kill_registered_child(&state);
        assert!(state.url.lock().unwrap().is_none());
        assert_ne!(state.current_generation(), gen);
        // The killed generation's watcher/timers no longer own the lifecycle.
        assert!(claim_exit(&state, gen).is_none());
        assert!(!is_still_starting(&state, gen));
    }

    #[test]
    fn timers_act_only_while_current_and_not_ready() {
        let state = ServerState::new();
        let gen = state.next_generation();
        assert!(is_still_starting(&state, gen));
        *state.url.lock().unwrap() = Some("http://127.0.0.1:5/".into());
        assert!(!is_still_starting(&state, gen), "ready silences the timers");
        *state.url.lock().unwrap() = None;
        state.next_generation(); // a restart superseded this attempt
        assert!(!is_still_starting(&state, gen));
    }
}

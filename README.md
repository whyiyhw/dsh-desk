# dsh-desk

Desktop shell for the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) web GUI — a Tauri 2 app that owns the `dsh web` server lifecycle and hosts the GUI in a native window.

## What it does

- **Server lifecycle**: spawns `dsh --profile web` as a child process, watches stdout for the `dsh web: <url>` readiness line (which carries the auth token), then navigates the window to the authenticated GUI. No copy-pasting tokens.
- **Tray-resident**: closing the window hides it to the tray; the server keeps running. Tray menu: Show window / Open in browser / Restart server / Edit config / Quit (stops the server).
- **Global hotkey**: `Alt+Shift+D` shows/hides the window from anywhere.
- **Single instance**: a second launch focuses the existing window.
- **Guided failures**: every dead end turns into an actionable panel — a slow/failed server shows Open log / Open config / Retry; a machine without dsh shows an install guide; a WebView2 runtime older than Chromium 119 shows the runtime-download guide.

## Install (from a release)

Windows 10 or newer.

1. Install [dsh](https://github.com/deepseek-ai/deepseek-harness) first — the app launches `dsh web` for you. (Not on PATH yet? The app's first-run guide will point you here too.)
2. Download the latest `dsh-desk_x.y.z_x64-setup.exe` (NSIS, recommended) or `.msi` from [Releases](https://github.com/whyiyhw/dsh-desk/releases) and run it.
3. Most Windows 10/11 machines already carry the WebView2 Evergreen runtime (it ships with Edge), so the installer has nothing extra to fetch. An in-app gate also checks the runtime on every launch and walks you through an upgrade if it is older than Chromium 119.

**SmartScreen / antivirus warning**: the installers are not code-signed, so Windows may show "Windows protected your PC". Click *More info → Run anyway* — and only ever download from this repository's Releases page, the one official distribution channel.

**Machines with no WebView2 runtime and no internet during install** (rare): the installer embeds the small bootstrapper (+~2 MB), but installing the runtime itself still needs internet. On such a machine, first install Microsoft's standalone x64 WebView2 runtime ([go.microsoft.com/fwlink/?linkid=2124701](https://go.microsoft.com/fwlink/?linkid=2124701)) from a connected machine, then run the dsh-desk installer.

**Updates**: the tray's *Check for updates* compares your version against the newest GitHub release and opens the Releases page when a newer one exists. dsh-desk never updates itself in place.

## Install (from source)

Requires Node ≥ 22, pnpm, and the Rust toolchain.

```sh
pnpm install
pnpm tauri build        # produces the installer under src-tauri/target/release/bundle/
# or for development:
pnpm tauri dev
```

## Configuration

On first run a default config is written to `%APPDATA%\dsh-desk\config.json` (Windows) — edit it to match your dsh installation.

With an installed `dsh` on PATH:

```json
{
  "command": "dsh",
  "args": ["--profile", "web", "--no-open"]
}
```

From a source checkout (launch node directly — pnpm's script layer mangles forwarded flags when spawned without an interactive shell):

```json
{
  "command": "node",
  "args": ["--import", "tsx/esm", "apps/cli/src/bin.ts", "--profile", "web", "--no-open", "--port", "0"],
  "cwd": "D:\\www\\github\\deepseek-harness"
}
```

- `command` / `args`: how to launch dsh. `--no-open` keeps the default browser from popping up (the window is the browser). `--port 0` lets the OS pick a free port — the shell navigates to whatever URL dsh prints, so the port drifting between launches is fine; drop it only if you know the default port is free.
- `cwd`: optional working directory (a source checkout root).
- The dsh stdout/stderr mirror to `%APPDATA%\dsh-desk\dsh-desk.log`.

## Support

- Bugs and questions: [GitHub Issues](https://github.com/whyiyhw/dsh-desk/issues).
- Please attach `%APPDATA%\dsh-desk\dsh-desk.log` — every URL in it is redacted down to scheme://host:port, so the auth token never reaches the file.

## FAQ

- **Opening `http://127.0.0.1:<port>/` in a browser shows "dsh web authentication required".**
  That is by design: the dsh server requires a token on first visit (`/?token=…` exchanges it for a long-lived cookie). Use the app's window (it performs the exchange automatically), or the tray's *Open in browser* (which opens the full authenticated URL). The log deliberately never contains the token URL — a token in a log file is a leak — so "copy the URL from the log" is intentionally not a thing.
- **The window shows "still starting…", then a panel with buttons.**
  The server did not print its readiness line within 90 s. It may just be slow (a late start is picked up automatically), or your dsh build changed its output wording — *Open log* shows everything the server printed, *Open config* the launch command, and *Retry* relaunches the server.
- **Why does the app depend on one stdout line?**
  It is the only stable surface dsh-desk allows itself: no DSH internals, no version-pinned APIs. Upstream gives no stability guarantee (the harness self-describes as a developer preview), so the matcher stays tolerant to wording drift and a failed match degrades into a visible, actionable panel within 90 s instead of hanging silently.

## Notes

- The shell depends only on the stable web surface (the printed authenticated URL), not on DSH internals — safe across harness upgrades; see the FAQ for what happens if the wording drifts.
- Sessions are durable on the dsh side: Quit stops the server, but your sessions resume on the next launch.
- **Versioning**: `src-tauri/tauri.conf.json` is the single source of truth; releases are tagged `vX.Y.Z` from a tree where all three version fields (`tauri.conf.json`, `Cargo.toml`, `package.json`) agree — CI enforces this before any release build.
- **WebView2 Runtime requirement**: the served GUI uses modern JS APIs (`AbortSignal.any`, `Promise.withResolvers`), so the WebView2 Runtime must be **Chromium 119+** (recommend ≥ 120). If the window renders but shows a persistent "connection lost" badge while a regular browser works, check the runtime version first — see `AGENTS.md` for the one-liner and the force-update procedure.

## License

MIT

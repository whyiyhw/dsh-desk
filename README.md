# dsh-desk

Desktop shell for the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) web GUI — a Tauri 2 app that owns the `dsh web` server lifecycle and hosts the GUI in a native window.

## What it does

- **Server lifecycle**: spawns `dsh --profile web` as a child process, watches stdout for the `dsh web: <url>` readiness line (which carries the auth token), then navigates the window to the authenticated GUI. No copy-pasting tokens.
- **Tray-resident**: closing the window hides it to the tray; the server keeps running. Tray menu: Show window / Open in browser / Restart server / Edit config / Quit (stops the server).
- **Global hotkey**: `Alt+Shift+D` shows/hides the window from anywhere.
- **Single instance**: a second launch focuses the existing window.
- **Guided failures**: every dead end turns into an actionable panel — a slow/failed server shows Open log / Open config / Retry; a machine without dsh shows an install guide; a WebView2 runtime older than Chromium 119 shows the runtime-download guide.

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

## Notes

- The shell depends only on the stable web surface (the printed authenticated URL), not on DSH internals — safe across harness upgrades.
- Sessions are durable on the dsh side: Quit stops the server, but your sessions resume on the next launch.
- **WebView2 Runtime requirement**: the served GUI uses modern JS APIs (`AbortSignal.any`, `Promise.withResolvers`), so the WebView2 Runtime must be **Chromium 119+** (recommend ≥ 120). If the window renders but shows a persistent "connection lost" badge while a regular browser works, check the runtime version first — see `AGENTS.md` for the one-liner and the force-update procedure.

## License

MIT

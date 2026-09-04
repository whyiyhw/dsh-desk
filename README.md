# dsh-desk

Desktop shell for the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) web GUI — a Tauri 2 app that owns the `dsh web` server lifecycle and hosts the GUI in a native window.

## What it does

- **Server lifecycle**: spawns `dsh --profile web` as a child process, watches stdout for the `dsh web: <url>` readiness line (which carries the auth token), then navigates the window to the authenticated GUI. No copy-pasting tokens.
- **Tray-resident**: closing the window hides it to the tray; the server keeps running. Tray menu: Show window / Open in browser / Restart server / Quit (stops the server).
- **Global hotkey**: `Alt+Shift+D` shows/hides the window from anywhere.
- **Single instance**: a second launch focuses the existing window.
- **Diagnostics**: all dsh stdout/stderr is mirrored to the console dsh-desk was started from; a server that dies before printing its URL replaces the window content with the failure.

## Install (from source)

Requires Node ≥ 22, pnpm, and the Rust toolchain.

```sh
pnpm install
pnpm tauri build        # produces the installer under src-tauri/target/release/bundle/
# or for development:
pnpm tauri dev
```

## Configuration

On first run a default config is written to `%APPDATA%\dsh-desk\config.json` (Windows) — edit it to match your dsh installation:

```json
{
  "command": "pnpm",
  "args": ["dsh", "--profile", "web", "--no-open"],
  "cwd": "D:\\www\\github\\deepseek-harness"
}
```

- `command` / `args`: how to launch dsh. `--no-open` keeps the default browser from popping up (the window is the browser).
- `cwd`: optional working directory (a source checkout root, for `pnpm dsh` launches).

## Notes

- The shell depends only on the stable web surface (the printed authenticated URL), not on DSH internals — safe across harness upgrades.
- Sessions are durable on the dsh side: Quit stops the server, but your sessions resume on the next launch.

## License

MIT

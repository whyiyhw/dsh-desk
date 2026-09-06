# S22 验证记录 — 子进程控制台窗口抑制（CREATE_NO_WINDOW）

日期：2026-09-06 · 分支：`s22-s23`（独立 worktree，见文末并行会话说明） · 提交：见 git log

## 问题

安装版（release 为 GUI 子系统，无控制台）spawn 控制台程序时，Windows 为每个子进程新建可见控制台窗口：

1. **持续窗口**：dsh 服务端子进程（node.exe，或 cmd.exe /C 兜底路径）——用户实测截图（2026-09-06 上午，v0.2.1 安装版）。这个窗口还是 **token URL 唯一的上屏出口**：dsh 打印的 `dsh web: http://…?token=…` 明文显示其中，S1 的日志脱敏管不到它。
2. **闪窗**：每次 `where.exe` PATH 探测（spawn 前置）、`reg.exe`（WebView2 门禁 + 暗色探测）、`taskkill`（杀树）都会闪一个黑窗。

机制：`std::process::Command` 默认不带 `CREATE_NO_WINDOW`（0x08000000）；debug 构建（`tauri dev`）父进程有控制台、子进程继承之，窗口不出现——所以只在安装版暴露。

## 修复

`src-tauri/src/lib.rs`：新增 `hide_console()` helper（`CommandExt::creation_flags(CREATE_NO_WINDOW)`，纯 std，无新依赖），应用到全部 5 处 spawn 点：

| 站点 | 用途 |
|---|---|
| `spawn_server` 的 `build()`（直接 spawn + cmd.exe /C 兜底共用出口） | dsh 服务端子进程——主 bug 位 |
| `command_locatable` | where.exe 探测 |
| `webview2_too_old` | reg.exe 版本查询 |
| `app_mode_dark` | reg.exe 暗色探测 |
| `kill_child_tree` | taskkill 杀树 |

不动的两处：`open_in_viewer` 的 notepad（故意可见）、测试用 cmd.exe（cargo test 本身在控制台里跑）。

## 验证方法与证据

仪器：`D:\tmp\s22-verify\run-watch.ps1` —— 200ms 节拍 EnumWindows 快照 `ConsoleWindowClass` 窗口，**按进程树归属过滤**（pid 沿 ParentProcessId 链上溯到被测 exe 才计入 HIT；机器上其他控制台窗口记 OTHER 排除）；就绪判据 = 全文件 `'dsh web: http'` 计数超基线；结束后 `taskkill /T /F` 杀被测树 + 前后进程清点。

**探针构建披露**：验证时用户安装版（pid 16792，11:47 启动）正在运行。原版构建会因 single-instance 插件被弹回、且热键注册失败会中止启动，无法共存。故对照/修复两组均打了**探针补丁**：identifier 改 `com.whyiyhw.dshdesk.s22probe`（独立 single-instance 锁 + 独立 WebView2 数据目录，顺带保证不碰用户实例的 cookie 库）+ 热键注册改非致命。探针补丁与控制台窗口行为正交（两组唯一差异 = S22 修复本身），不进仓库。

**对照组（HEAD + 探针补丁）** `prefix-probe`：

```
ready=13.9s consoleHits=43
  HIT: tick=4.6s pid=19644 where.exe visible=1     ← PATH 探测闪窗，被 200ms 节拍抓到
  HIT: tick=4.9s→13.9s pid=20000 node visible=1    ← 持续可见的 node 控制台窗（全程）
  postDeskAlive=0 postMyNodes=0                     ← 杀树后零残留
```

**修复组（S22 + 探针补丁）** `postfix-probe`：

```
ready=10.1s consoleHits=0                           ← node 窗、where/reg 闪窗全部消失
  postDeskAlive=0 postMyNodes=0
```

两次运行同机同会话、用户安装版全程在场（其自身 node 控制台窗口被记 OTHER 正确排除，pid 2476/15600）。`cargo test` 17 passed，`cargo fmt --check` 净。

## 已知边界（如实记录）

- **taskkill 闪窗未单独实机捕捉**：本验证的杀树由脚本发起（观察器已先行停止，避免把脚本自己的 taskkill 计入）；应用自身 Quit 路径的 taskkill 走同一 `hide_console()` helper（代码锚定），S23 批次真机走查托盘 Quit 时会带观察器复核。
- **§2.4 六契约走查不在本记录**：本验证只锚控制台窗口断言与契约 6（零残留）；完整走查在 S23 验证战役中对包含 S22 的最终构建执行。
- **审查修订（P3-1）**：flag 改为仅 release 生效（`if !cfg!(debug_assertions)`）——dev（`tauri dev`）下父进程持有控制台、子进程原本附着其上并随 Ctrl+C 卸载而死；无条件加 flag 会让子进程脱离 dev 控制台变成孤儿。release 行为与本验证所测一致（探针即 release 构建），dev 语义完整保留。

## 并行会话说明

主工作树当前有另一会话未提交的 S21（窗口 caption 断缝）改动（lib.rs 317 行、webview2-com 依赖）。按仓库"一个任务一个 worktree"铁律，S22/S23 在独立 worktree `D:\www\github\dsh-desk-s22s23`（分支 `s22-s23`，基线 cf8f582）进行；对面 hunk 与 S22 的 spawn 站点不相交，合并期唯一相遇点是 S23 的窗口创建区。

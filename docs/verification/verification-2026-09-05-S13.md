# S13 验证记录 — 崩溃可诊断与日志轮转(2026-09-05)

> 交付物:panic hook(→ log_line)、启动横幅(版本 + exe + **dsh 命令行**)、启动时日志轮转(>512 KiB → `.old` 一代)。提交 `9118fcd`(v0.2.0)。

## 验收对照(spec §2.3 S13:"std::panic::set_hook → log_line;日志启动时写版本横幅与 dsh 命令行;启动时截断/轮转日志")

**1. panic hook — 通过(cargo test 锚定)**

`install_panic_hook()`(run() 与测试共用同一闭包,格式不会漂移);测试 `panic_hook_lands_in_the_log`:装钩 → `catch_unwind` → 断言日志含 `dsh-desk panic: panicked at` + `[cargo-test probe]` 探针文案(标注来源,稀释"panic 行=真信号"的风险已述)。探针会向真实日志追加一行测试标记——已用文案区分,属可接受噪声。

**2. 启动横幅 — 通过(真机)**

```
dsh-desk v0.2.0 starting (D:\...\dsh-desk.exe); launch: `node --import tsx/esm apps/cli/src/bin.ts --profile web --no-open --port 0`
```

横幅含 dsh 命令行(审查 P3 修正:原实现只写版本+exe,而 WebView2 过旧门禁路径在"started"行之前就 return,config 的 command/args 会全程不进日志——并入横幅后所有路径都覆盖)。每次启动必写(两次真机重启均见)。

**3. 日志轮转 — 通过(真机 ×2)**

- 预置 601,391 字节(含垃圾行)→ 启动 → `dsh-desk.log.old`(601,391 B)保留一代、新日志以横幅开头;
- 阈值 512 KiB 以下不动(`oversized_log_rotates_to_old` 锚定:超限转、欠限留、`.old` 顶替)。

**轮转位置 = setup() 而非 run() 顶部(审查 P2 修正)**:single-instance 插件在 builder 期 init,第二实例在那里 `exit(0)`、到不了 setup——若轮转在 run() 顶部,二次启动会**把运行中实例的活日志腰斩进 `.old`**、自身又不写横幅解释,用户按支持指引附的日志会静默缺失近期历史。挪进 setup 后只有幸存实例轮转。

## §2.4 六条(v0.2.0 最终二进制,2026-09-05)

#1 就绪→隐藏启动→可见(确定性判据:首次可见瞬间就绪行已在盘)→ WS ESTABLISHED ✓;#2 托盘六项菜单 ✓;#3 热键 ✓;#4 二次启动 ✓;#5 关窗藏托盘 ✓;#6 Quit 零残留 ✓(清点仅基线进程)。

## 途中踩坑(已按台账处置,细节回写 AGENTS.md)

- **cookie 库再脏**(WS 无建立、窗口标题停在 "DSH Desk"):本轮全是干净 Quit 周期也复现——触发面比台账原记录(强杀风暴)更宽。处置同台账:Quit → 清 dshdesk 名下 webview 的 Cookies/Cookies-journal → 重启 → WS 恢复。
- **探测基线 vs 轮转**:启动前取的日志计数基线会被轮转作废(新日志从零计),就绪检测必须在轮转后重取基线——launch 脚本族要配套。

## 环境还原

- 最终 Quit 后清点:无 dsh-desk、无本应用 node 子进程、无 dshdesk webview;基线 node(18932/20800/22956 + 会话工具链)如常。
- 测试脚本与证据:`%TEMP%\p3-smoke\`(launch-probe/maximize/rect/vmcheck + junk 日志验证);walkthrough 等复用 `%TEMP%\p2-smoke\`。
- 真实日志中留有 `[cargo-test probe]` 探针行与测试期间正常运行日志,属预期。

## 一句反思

"启动时轮转"看似只能放在 run() 第一行,但 single-instance 的第二实例恰恰死在 builder 期——跨进程的副作用(rotate)必须放在"确认自己是幸存者"之后,这是所有启动期文件操作的通用规则。

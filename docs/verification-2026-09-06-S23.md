# S23 验证记录 — 多实例支持（--instance，多进程一实例一服务）

日期：2026-09-06 · 分支：`s22-s23`（worktree，S21 由并行会话在主树进行） · 构建物：`D:\tmp\s23-verify\dsh-desk.exe`（release，GUI 子系统）

**状态（终版）：Phase A + Phase B 全过，合并门禁清空**。Phase B 于 2026-09-06 晚（用户退出安装版、机器独占后）在 **v0.3.0 终版 release**（e54c53b 同源重建，横幅 v0.3.0）上完成。

## Phase A — 命名实例（与用户在跑实例零干扰地完成）

仪器：`D:\tmp\s23-verify\s23-launch.ps1`（按实例日志等就绪行 + ConsoleWindowClass 进程树归属观察 + 窗口定位截图）、`capture.ps1`（SetProcessDPIAware + 标题定位 + CopyFromScreen；**前景化注意**：SetForegroundWindow 从后台进程偷焦点会被拒，覆盖窗需先最小化）、Umi-OCR（窗口内容 = 在线金标准，netstat 判据已废）。

| # | 断言 | 结果 |
|---|---|---|
| 1 | 命名实例就绪（vtest 11.4s / vtest2 12.6s），与安装版 + 彼此三进程并存 | ✅（pid 16792/20296/21632，各自窗口标题 `DSH Desk — vtest(-2)`） |
| 2 | 首跑默认 config 含 `--port 0`，落在 `instances\<name>\config.json` | ✅（文件内容核对） |
| 3 | 实例独立落盘：`instances\vtest\{config.json, dsh-desk.log, tray-hint.shown, webview\}` | ✅（tray-hint 在关窗动作后出现；webview 剖面 40MB） |
| 4 | GUI 认证（OCR 金标准）：会话列表/侧栏/设置可见，非 401 非 starting 面板 | ✅ |
| 5 | **cookie 隔离**：vtest2 完成 token 交换后 vtest 仍认证 | ✅（vtest 二次 OCR 仍为真实会话列表——共享 cookie 库场景此步必 401，cf8f582 同族） |
| 6 | **同实例弹回**：`--instance vtest` 二次启动 exit 0、不产生第二窗口 | ✅（实测两次：陈旧实例在场时新启动即退） |
| 7 | **弹回唤醒**：藏窗（产品路径 WM_CLOSE→hide→托盘）后二次启动，幸存窗口复活 | ✅（exited=True code=0 windowVisibleAgain=True；WM_COPYDATA→wnd_proc→show+focus 全链路） |
| 8 | 热键降级：命名实例不注册 + 日志 `hotkey not registered (non-default instance)` | ✅（不中止启动——旧代码此处 `?` 会直接杀掉启动） |
| 9 | S4 门禁按实例工作：首跑默认 config 的 `dsh` 裸名不在 PATH → 安装引导面板（不挂死） | ✅（换真实 config 后正常起服） |
| 10 | S22 回归：全程 ConsoleWindowClass 树内命中 = 0 | ✅（三次启动） |
| 11 | 按实例清点：taskkill 树后无残留（desk/node 全退，18 进程） | ✅（kill 路径；托盘 Quit 路径归 Phase B） |

日志证据（`instances\vtest\dsh-desk.log`）：banner、`instance "vtest"`、`hotkey not registered`、就绪行。

## 独立审查与修复复验（2026-09-06 第二轮）

无上下文审查子代理（审 D:\tmp\s23-verify\s23.diff + 硬约束 + 五问，可跑只读 cargo）：**1×P1、2×P2、6×P3**；架构主张（代码建窗后 window-state 仍恢复、默认剖面未动、`.window-state.json` 文件名不变、按实例文件互斥、弹回零副作用、S2 生命周期无干涉——均对照 vendored 源码核实）全部成立。修复：

- **P1** `--instance default` 与默认实例同锁不同目录（最恶劣配对）→ **`default` 保留字**（校验 + 测试锚）。
- **P2** `SendMessageW` 无超时（幸存者忙/挂起时二次启动隐形卡死）→ `SendMessageTimeoutW(SMTO_ABORTIFHUNG, 3s)`，超时落日志提示走托盘。
- **P2** `CreateMutexW` 空句柄未检查（锁未持有即当主实例 → 双主）→ 空句柄 = 明确降级"无同实例保护继续跑" + 日志；拒启日志措辞修正（覆盖"守卫窗创建失败"场景）。
- **P3×5** `args_os` 替 `args()`（非 UTF-8 argv 不再 panic）；命名 webview 剖面迁 **LOCALAPPDATA**（缓存不进漫游剖面，与默认实例同约定）；wnd_proc 改 **进程级 static 指针**（窗口创建前发布，消除 userdata 竞态）；IDENTIFIER 常量注释（含与旧插件锁名不同的一次性升级边缘）；同义反复测试改真断言。

**修复后重建 release 全量复验 Phase A（本轮证据即最终构建）**：vtest 11.5s / vtest2 12.4s 就绪、三进程并存、双 hwnd、consoleHits=0、vtest 认证 OCR（本地构建/工作区/新会话）、vtest2 认证后 vtest 仍认证（5 处 GUI 标记）、WM_CLOSE 藏窗→同实例重启 exit 0→窗口复活（SendMessageTimeoutW 路径）；新目录布局核对（APPDATA: config/log；LOCALAPPDATA: webview 18M）；杀树 18 进程零残留。

## Phase B — 默认实例 §2.4 全量走查（2026-09-06 晚，v0.3.0 终版）

仪器：`pb1-launch.ps1`（默认日志就绪计数+控制台窗树归属观察）、`pb-flyout.ps1`（Win+B 开溢出浮层，配方 v4）、`p2-smoke/tray.ps1`（6002 直投开菜单 + 实测矩形按行点击 + 效果验证重试）、`pb-quit.ps1`（Quit 全程 60ms 紧凑控制台窗观察器）、Umi-OCR。

| # | 契约 | 结果 |
|---|---|---|
| 1 | 就绪行 → 窗口 → 认证 GUI | ✅ 11.7s 就绪；横幅 `v0.3.0`；OCR = 认证 GUI（会话输入页）；**零降级行**（默认实例持有热键）；consoleHits=0 |
| 2 | 托盘六项全部可用 | ✅ Show（6002 直投一次命中，窗口显）✓ Restart（新就绪行 22>21，**跨端口重启后 OCR 复认仍认证**——401 修复路径复验）✓ Edit config（日志确认打开默认实例路径 `…\dsh-desk\config.json`）✓ Check updates（`no newer release than 0.3.0; newest published is v0.2.1` 语义正确）✓ Open in browser（应用日志行 + 浏览器侧证据：Chrome 窗标题变 "DSH 本地构建 - Google Chrome"）✓ Quit（一次命中）✓ |
| 3 | 热键 Alt+Shift+D 切换 | ✅ True→False→True 双向两轮（p2-smoke hotkey 和弦） |
| 4 | 同实例二次启动唤醒 | ✅ exit 0 + 幸存者窗口成为前台 + **横幅/就绪行零增量**（弹回零副作用实证）+ 进程数仍 1 |
| 5 | 关窗藏托盘、服务继续 | ✅ WM_CLOSE（产品路径）→ 隐藏；server node 1→1 存活；首关一次性标记落盘（marker False→True；toast 送达级=标记+无失败日志，本机 FA 抑制同 S14-S19 口径） |
| 6 | Quit 零残留 | ✅ dsh-desk=0；应用 spawn 的 node（pid 17308/21376）全死；在场的 4 个 node 经甄别全为 ZCode MCP 服务器（`@z_ai/mcp-server`/`mcp-remote`，父链 cmd.exe、与应用 pid 无交集）——非残留 |

**S22 悬欠项就此关闭**：Quit 全程 60ms 紧凑观察器 consoleHits=0——应用自身 Quit 路径的 taskkill 亦无控制台窗闪现（此前仅代码锚定）。

**附带回归证据**：S9 窗口几何恢复照常（恢复用户保存几何，含已知 (-9,-9) 最大化会话瑕疵——同文件同插件同行为，非 S23 回归）；S13 横幅含完整启动命令；S19 首关提示按一次性语义工作。

Phase B 观察备忘：Edit config 行效果确认用了 3 次重试（系统查看器启动 >5s 超出首轮效果窗，菜单动作本身一次命中；查看器=Cursor，最小化处理未杀）；Open in browser 落在用户既有 Chrome 的新标签页（浏览器 cookie 共享面 = README 已注明的文档化例外）。

环境还原：window-state 恢复备份、tray-hint 标记删除（还原用户"未见过提示"状态）、Chrome 还原前台、Umi-OCR 关闭、instances 目录清除、dsh-desk 进程 0、config.json 未被改动。

## 测试锚（cargo test 22 passed）

实例名校验（含 `--flag` 防误读、33 长度、非 ASCII）、双形式 `--instance` 解析、默认/命名路径分叉、默认实例不重定向 WebView2 剖面 + 命名实例必须独立、默认 config `--port 0`。

## 已知边界

- **窗口外部 SW_HIDE 后弹回不复活**：外部藏窗绕过 tao 的 visible 状态，`window.show()` 同值短路 no-op（原 single-instance 插件回调同性质）；产品路径（hide()/close-to-tray）不受影响，已按产品路径复验。
- **Phase B 未跑完前默认实例行为**：代码面零改动（路径/标题/热键全部走 Default 分支原值），22 项测试锚定；但 §2.4 走查是门禁，未过不并 main。
- 非默认热键占用时默认实例降级继续（新行为，替代旧的中止启动）——Phase B 顺带实证。

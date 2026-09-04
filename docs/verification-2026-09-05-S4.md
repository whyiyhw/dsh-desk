# S4 交付自验记录（2026-09-05）

> 对象：S4 · 首跑引导与失败可操作化（规范见 [spec-and-plan.md](spec-and-plan.md) §2.3）
> 构建：`src-tauri/target/debug/dsh-desk.exe`（初验 00:25 构建；审查修复后 01:17 构建复验；含本会话 S4 改动 **与并行会话 S2 改动的合并态**，见"并行会话"节）
> `cargo test`：初验 **9 passed**，审查修复后 **10 passed**（S4 新增 3 条：reg 输出解析 ×2、版本门槛 ×1；其余为 S1/S2）
> 环境：本机 Windows 10 19045，WebView2 152.0.4191.62，`where dsh` 本机找不到（天然复现"干净机器无 dsh"）

## S4 验收三条（规范 §2.3）

| # | 验收条目 | 怎么测 | 结果 |
|---|---|---|---|
| 1 | 干净机器（无 dsh）首跑 ≤10s 进引导态且按钮可用 | 备份并删除 config.json（本机 `where dsh` 本来就失败，等价干净机器）→ 启动 exe → 计时观测日志门禁标记 | 门禁 **1.5s** 触发；AX DOM 级证据：引导文案逐字在场 + `Open config`/`Open dsh project page`/`Retry` 三按钮；页面停在 `tauri.localhost`（未误导航）。三按钮逐个 AXInvoke：Open config → 日志双行（opening + opened in system viewer）✓；Open dsh project page → 日志双行 ✓（默认浏览器打开项目页）；Retry → 门禁复触发（日志标记计数 1→2），正确滞留引导态 ✓ |
| 2 | 托盘 Edit config 能打开 config.json | 溢出浮层 → API 精确矩形右键 → 菜单实测矩形按行高点击第 4 行 | 菜单 5 行（高 126px，含新 Edit config 项）→ 点击行 4 → 日志 `opening config.json for editing` + `opened ... in system viewer` ✓（见"托盘驱动配方"节——图标在溢出区，S1 的 6002 直投配方失效，本记录沉淀了新配方） |
| 3 | pv 临时改低 → ≤10s 过旧态、下载按钮开官方页、恢复 pv 后 Retry 到 Ready | HKLM 写入被 UAC 拒绝（shell 未提权）→ 改走代码里的第二探测点 HKCU（同一套门禁代码，顺带验证 per-user 回退）：新建 `{F3017226-...}` 键写 `pv=118.0.9999.0` | 门禁 **1.0s** 触发，面板显示"runtime is 118.0.9999.0 (Chromium 118) ... needs Chromium 119 or newer"（AX 逐字核对）+ 两按钮；**无任何 server spawn**（日志无 started 行，门禁挡在 spawn 前）✓；下载按钮 → 日志双行 + 浏览器实际开始下载独立安装包（Downloads 出现 61MB+ `.crdownload`，未运行）✓；删除 HKCU 键恢复 → Retry → `retry_server invoked` → spawn（pid 4036）→ 就绪行（端口 4034）→ 窗口导航 ✓ |

补充验证（超出验收字面但属 S4 语义）：

- **非首跑也兜**：启动命令不可寻位时统一进引导态（不只字面"首跑"）——第二次启动若 dsh 仍缺，引导态比裸 spawn 错误更准确；Retry 拦截与启动门禁收敛为同一规则 `command_locatable`。
- **S1 转交项关闭**：open-browser 在 url=None 时不再静默 no-op（日志提示行已加，§2.4 走查中 Starting 态点击可观察——本次未单测该分支，代码路径极简）。
- **面板加载竞态防**：启动期面板（引导/过旧）eval 可能早于 index.html 脚本注册 helper——`show_panel(wait=true)` 以 200ms×25 轮询等 helper 出现，5s 后才落纯文本回退（真机上引导态 1.5s 出现且带按钮，证明轮询路径生效）。

## §2.4 回归契约走查（六条）

同一实例（pid 22628，真实 config：node 源码 checkout + `--port 0`）串行完成：

| # | 契约 | 结果 | 证据 |
|---|---|---|---|
| 1 | 就绪行 → 窗口显示并导航认证 GUI | ✓ | Retry 后就绪行（端口 4034）→ 窗口 AX 文档 = `http://127.0.0.1:4034/`（认证 URL 的 host:port）。注：页面一次显示 dsh 401 文本（token 未随导航生效），一次显示正常聊天 GUI——瞬态，见"已知边界" |
| 2 | 托盘四项全部可用 | ✓ | **Show**（行 1 点击 → 隐藏窗口变可见）/ **Open in browser**（行 2 → 日志 opened the authenticated URL，一次命中）/ **Restart**（行 3 → 旧进程杀净 + 新 spawn + 新就绪行 ×2 轮；S2 代数静默实战生效：`stdout of pid ... closed on a superseded generation; ignoring`）/ **Quit**（行 5 → 应用退出） |
| 3 | 热键 Alt+Shift+D 双向切换 | ✓ | 合成 chord：visible True→False→True |
| 4 | 二次启动唤起聚焦 | ✓ | 先藏窗口 → Start-Process 第二实例 → 进程数仍 1（礼让退出）+ 旧窗口被 show |
| 5 | 关窗藏托盘，server 继续运行 | ✓ | AX 按窗口 Close → 窗口 visible=False + app 存活 + node 子进程（pid 4036）存活 |
| 6 | Quit 后无 dsh/node 残留 | ✓ | 托盘 Quit 后清点：零 dsh-desk；node 全为用户自有进程（3080 开发实例 20800、editor、astro）与工具链（npx/mcp），无本次测试孤儿 |

## 托盘驱动配方（本次最大沉淀，S1 配方在本机已失效）

**S1 的 6002 直投配方为什么失效**：tray-icon 0.24.2 的 `WM_USER_TRAYICON` handler 里 `Shell_NotifyIconGetRect` 失败会**静默 return 0**（不发声事件、不开菜单）。DSH 图标现在常驻**溢出浮层**（折叠箭头后），图标不可见时该 API 失败 → 6002 投递看起来"没反应"。S1 当晚图标恰好可见所以配方能通。（读 crate 源码 `~/.cargo/registry/src/*/tray-icon-0.24.2/src/platform_impl/windows/mod.rs` 确认。）

**新配方（不依赖图标可见性）**，成品脚本 `%TEMP%\s4-test\tray-run.ps1`（带效果验证与重试）：

1. 真点击折叠箭头开浮层（本机物理坐标 ≈(2371,1421)；chevron 是开关，开着再点会关——先查 `NotifyIconOverflowWindow` 可见性）；
2. **浮层开着时** `Shell_NotifyIconGetRect(hwnd=应用 tray_icon_app 窗口, uID 扫描 1..8)` 会返回图标在浮层内的**真实屏幕矩形**（浮层关着时只返回 chevron 位，`T ≥ 任务栏顶`可甄别）；
3. 对该矩形中心 `mouse_event` 真右键 → 菜单（#32768，按 pid 过滤）；
4. 实测菜单矩形按 高度/5 算行中心点击；
5. **每步带效果验证重试**：菜单会被用户真实活动夺焦关闭（本机用户在使用中，实测一次点击落空），效果不过就整套重来，≤4 次。

坑位记录：视觉模型对 20px 托盘图标/浮层网格的**坐标断言不可靠**（同一张图两轮分析给出矛盾位置，按视觉坐标右键开出了 NVIDIA 的菜单）——`Shell_NotifyIconGetRect` 才是确定性的。

## 并行会话（重要：本记录的验证对象是合并树）

S4 验证中途发现另一 ZCode 会话在同一工作树实现 **S2（代数标记）**：其实例（00:22 启动）占住 single-instance 锁，把我的测试实例静默弹回（"启动即退出"假象的又一来源）。处置：按台账清残留口径杀其分离实例后改用"每阶段前清点"。**本记录所有真机证据采集自 S4+S2 合并构建（00:25）**——S4 特性归属清晰（引导/门禁/托盘项/命令），生命周期行为（Restart 日志、Quit 清理）为两者共同产物。S2 的验收由其会话自理。

## 已知边界（不阻塞 S4）

1. **窗口侧 401 瞬态**：Retry→Ready 后窗口一次显示 dsh 401 文本页（token 未随导航生效），另一次正常聊天 GUI；同 URL 经托盘 Open in browser 在浏览器正常进 GUI（state.url 完好）。**归因已有台账条目**：并行会话同日确诊"强杀风暴弄脏 EBWebView cookie 库 → 窗口 401"（AGENTS.md 真机 E2E 节）——本轮测试反复 `taskkill /T /F` 清场正中该诱因，非 S4 门禁/面板回归。后续验证前按该条目清理 cookie 库或减少强杀即可避免。
2. HKLM pv 无法从非提权 shell 改写——过旧态走 HKCU 路径验证（同一代码路径）。若要按验收字面测 HKLM，需提权 shell + UAC 确认。
3. Edit config 用系统查看器打开（本机非 notepad）——行为符合"打开即可编辑"，未断言具体编辑器。

## 回合末独立审查（已执行）与修复

无作者上下文的独立子代理审查全量未提交改动，结论"需修改后交付"：1×P2 + 6×P3，无 P1。修复与处置：

| 发现 | 级别 | 处置 |
|---|---|---|
| 启动期 degraded 面板（spawn 失败/早夭 EOF）竞态首屏脚本加载，落入无按钮纯文本回退 | P2 | **已修**：`show_degraded` 增加 `wait` 参数，spawn 失败与 `!had_url` 退出走 `wait=true`（页面必是 index.html），GUI 页上的退出与 90s 超时保持 `false` |
| 托盘 Restart 绕过运行时门禁（从过旧态一次 Restart 即 spawn 进破损 GUI） | P3 | **已修**：运行时门禁挪进 `spawn_server`（boot/Retry/托盘 Restart 全路径），setup 与 retry 的检查分别为简化与快路径。**真机复验**：过旧态下托盘 Restart → 门禁再触发、spawn 计数零增长 |
| 注册表探测 HKLM 先 + 任一过旧即挡，与 loader 的 per-user 优先相反（机器级旧+用户级新的机器误挡） | P3 | **已修**：探测键序按 loader 优先级（HKCU 先），**首个应答者定夺**；成功查询但 pv 不可解析也挡（`<unreadable>`） |
| 首跑即过旧的机器 config.json 永不创建 → 托盘 Edit config 打开空路径 | P3 | **已修**：门禁置于 `load_config` 之后（首跑仍写默认配置） |
| README 托盘清单缺 Edit config、失败态描述过时 | P3 | **已修**（托盘行 + Guided failures 行；S12 做发布物料时再全面重写） |
| spec §2.3 "首跑(无 config.json)" 与实现（每次 spawn 门禁）漂移 | P3 | **已记**：spec §5 S4 行补偏差说明 |
| `redact_urls` 只匹配小写 `http`（`HTTP://` 形式 token URL 会明文落盘） | P3 | **未修**（S1 既有代码，dsh 现行输出恒小写；留给 S1/S2 维护者，防御性增强一行可完成） |
| S2 侧定时器冗余代数复查（`is_still_starting(...) && current_generation()==gen`） | P3 | **未动**（S2 会话的代码，非 S4 范围） |

修复后：`cargo test` 10 passed（含并行会话新增）、重建并真机复验三链路——假 pv boot 1.2s 进过旧态 ✓、过旧态托盘 Restart 被拦（spawn 48→48）✓、删假 pv 后面板 Retry → spawn → 就绪行 ✓、托盘 Quit 零残留 ✓。

## 环境还原

- 注册表：HKLM pv 全程未动（152.0.4191.62 原值）；测试用 HKCU `{F3017226-...}` 键已删除（复验后再次清点确认）。
- config.json：已从备份还原（node 源码 checkout + `--port 0` 原配置）；`config.json.bak-s4`/`config.json.pre-s4` 已删除。
- 进程：初验与复验两次 Quit 退出，终态清点零残留（dsh-desk 0，node 仅 3080 开发实例与用户自有进程）。
- `%TEMP%\s4-test\`：全部测试脚本保留（tray-run.ps1 为带效果验证的托盘驱动成品）；`%TEMP%\s1-*` 为 S1 存档。
- Downloads：测试触发的 `MicrosoftEdgeWebView2RuntimeInstallerX64.exe` 下载已删除。

## 一句反思

并行会话共享真机时，"开测前清残留"必须从一次性动作升级为**每个测试阶段前的例行清点**——这次弹回浪费的十分钟，根源是我明知台账有"残留实例复活"条目却只在最开始查了一次进程表。

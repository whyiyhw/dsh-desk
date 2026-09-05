# AGENTS.md — 给后续 agent 的环境事实与教训

> 本文件记录这台机器上排查过的真实事故。接手任务前先读完这一页，可以少走几小时弯路。
> 详细复盘见 [docs/postmortem-2026-09-04-webview2-114.md](docs/postmortem-2026-09-04-webview2-114.md)。

## 项目一句话

Tauri 2 桌面壳：spawn `dsh web` 子进程 → 解析 stdout 就绪行拿带 token 的 URL → 导航 WebView2 窗口。核心逻辑在 `src-tauri/src/lib.rs`（S1-S5a 后约 1200 行，含测试）。

## 规划与规范入口

- **唯一规范源**：`docs/spec-and-plan.md` —— 阶段门禁、S1-S13 规范与验收标准、决策记录。
- **执行手册 skill**：`.agents/skills/dsh-desk-plan/` —— 硬约束、工作协议、快速索引。做任何开发/评审/排障工作前先读它。
- 两者冲突时，以 `docs/spec-and-plan.md` 为准。

## 本机环境事实（都是踩过坑验证过的）

### WebView2 运行时曾经冻结在 114（已修复，但要警惕复发）

- 2026-09-04 排查"连接异常"时发现：WebView2 运行时是 **114.0.1823.43**（2023-06），而 Evergreen 自动更新早已失效。同机 Edge 浏览器是 151。
- dsh 前端用了 `AbortSignal.any`（Chromium 116+）和 `Promise.withResolvers`（Chromium 119+），在 114 里页面 JS 直接崩 → 症状是 GUI 能渲染但一直"连接异常"。
- **已强制升级到 152**。但如果将来 WebGUI 又出现"浏览器正常、桌面壳异常"，第一件事查运行时版本：

```powershell
(Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}').pv
```

- 强制升级方法（升级其他机器时照抄，每一步都有坑，见复盘文档）：
  1. 官方 bootstrapper（fwlink 2124703）对已安装实例直接报 `0x80040828 already installed` 拒绝重装，**没用**。
  2. `MicrosoftEdgeUpdate.exe /ua /installapp appguid={F3017226-...}` 实测**没动静**。
  3. 有效的方法：fwlink **2124701**（x64 独立包）→ **下载下来是 EXE 不是 MSI**（文件头 `4D 5A`，别拿 msiexec 去开，报误导性的 2203/1620）→ `Start-Process <exe> '/silent','/install' -Verb RunAs -Wait`，需要用户点一下 UAC。
  4. 升级后必须**杀干净所有 `msedgewebview2` 进程并重启应用**，旧进程不会热替换。

### 端口：config.json 里 `--port 0` 是必配的

- 这台机器常年跑着 dsh 开发实例占住 **3080**（就是 agent 自己的 WebGUI）。桌面板再起 `--profile web` 默认端口必然 `EADDRINUSE`。
- `%APPDATA%\dsh-desk\config.json` 的 args 已加 `"--port", "0"`（OS 挑空闲端口）。壳只认 stdout 里打印的实际 URL，端口漂移无影响。
- 看到 EADDRINUSE 不是 bug，是没加这个参数（或被改回去）。

### 沙箱与执行方式

- `pnpm tauri dev` 会启动真实 GUI（WebView2 要建 IPC 通道、要 spawn 子进程）。在 DSH 的 workspace-write 沙箱下 WebView2 初始化直接崩（`platform_channel.cc: Access is denied`），**必须以完整权限运行**。
- 编译产物在 `src-tauri/target/`；增量编译很快（<1s），但冷构建约 2 分钟，耐心等。

### 真机 E2E 与托盘程序化驱动（S1 验收沉淀，2026-09-04/05 深夜；S4 修订；Phase 0 再修订＝配方 v3）

- **托盘菜单可以程序化驱动**，但 **6002 直投配方有前提：`Shell_NotifyIconGetRect` 能解析出图标矩形**——图标在可见托盘区，**或溢出浮层开着时**都满足（S4 先证"图标必须可见"，Phase 0 补证"浮层开着即可"）：
  1. tray-icon crate 只认 `WM_USER_TRAYICON=6002` 回调消息（lParam 装鼠标事件，如 WM_RBUTTONUP=0x0205）——直发鼠标消息无效，读 crate 源码定消息号，别猜；
  2. **handler 里 `Shell_NotifyIconGetRect` 失败会静默 return 0**（不发声事件、不开菜单）——图标在**溢出浮层**（折叠箭头后）时必现，6002 投递"没反应"先查图标可见性；
  3. **图标在溢出区时的稳定配方**（S4 沉淀，成品 `%TEMP%\s4-test\tray-run.ps1`，带效果验证重试）：真点击折叠箭头开浮层（chevron 是**开关**，开着再点会关，先查 `NotifyIconOverflowWindow` 可见性；本机物理坐标 ≈(2371,1421)）→ **浮层开着时** `Shell_NotifyIconGetRect(tray_icon_app 窗口, uID 扫 1..8)` 返回图标浮层内真实矩形（浮层关着只给 chevron 位，按 `T < 任务栏顶` 甄别）→ 矩形中心真右键 → 菜单（#32768 按 pid 过滤）→ 实测矩形按 高度/行数 点击；
  4. 菜单仅在前台稳住时才留住——**用户正在用机器时会秒关**，点击落空不是坐标错；每个动作都要**效果验证 + 整套重试**（≤4 次）。**Open in browser 测试留下的浏览器窗口是头号焦点杀手**（S2 实测：Chrome 在前台时浮层右键菜单连开 4 次全败，关掉后一次成功）——托盘驱动前先关浏览器；
  5. **PS 脚本必须先 `SetProcessDPIAware()`**：本机 125% 缩放下 unaware 进程的 SetCursorPos 坐标被 ×1.25 虚拟化，点击静默落空；菜单/浮层弹出后用 GetWindowRect 实测矩形算行中心，不要用旧标定值；
  6. 就绪检测用**全文件 `'dsh web: http'` 计数**，别匹配尾行——`log_line` 多线程 O_APPEND 追加，文件行序 ≠ 逻辑时序；
  7. **视觉模型对 20px 托盘图标/浮层网格的坐标断言不可靠**（S4 同图两轮分析给出矛盾位置，按视觉坐标右键开出了 NVIDIA 的菜单）——`Shell_NotifyIconGetRect` 才是确定性坐标源，视觉只作线索。
  8. **图标在溢出浮层内的格子会随邻居图标增减漂移**（S2 实测：一轮风暴前后从 (2363,1312) 挪到 (2221,1350)）——每次点击前现取 GetRect，别复用上一次的矩形。
  9. **配方 v3（Phase 0 冒烟沉淀，成品 `%TEMP%\p0-smoke\p0-tray2.ps1`）：浮层开着时 6002 直投是首选开菜单方式**——`PostMessage(tray_icon_app, 6002, uID, WM_RBUTTONUP)`，不动物理鼠标，Phase 0 四次菜单动作（Show/Restart/Open-browser/Quit）全部一次命中；**物理右键在用户在场时不稳**（同机同位置四轮 no-menu，即第 4 条"菜单秒关"模式），降级为兜底。**uID 实测为 2**，别假设是 1，仍按 1..8 扫描取首个 `hr=0 且 T<任务栏顶` 的。
  10. **`FindWindowW('Tauri Window')` 本机返回 0，但 EnumWindows+GetClassName 能看到该窗口**（跨 DPI 感知上下文的 Win10 怪癖）——窗口查找/可见性判据一律走 EnumWindows，别用 FindWindowW。
  11. **`MainWindowHandle` 不能当可见性代理**：主窗口隐藏后会回退到 single-instance 插件的可见辅助窗口（类名 `com.whyiyhw.dshdesk-sic`），句柄非 0——判可见性用第 10 条的 EnumWindows + `IsWindowVisible`。
  12. **S5a 后托盘菜单为 6 项**（Show / Open in browser / Restart / Edit config / Check for updates / Quit），行数写死 5 的旧脚本会点错行——Phase 2 会话成品 `%TEMP%\p2-smoke\tray.ps1` 已参数化 `-Rows`（默认 6），同目录还有 launch/census/hotkey/second/close/winvis/walkthrough 全套（launch/second 的 exe 路径已参数化，可指向安装版）。
  13. **配方 v4（S7 验收沉淀，2026-09-05 宿主重启后）：开溢出浮层的首选 = `Win+B`（焦点到托盘）+ `Enter`（点 chevron），脚本内用 `keybd_event(0x5B/0x42/0x0D)` 序列**——宿主重启后物理点击 chevron（SetCursorPos+mouse_event 与 CUA 点击两种姿势、多组坐标含任务栏 Button 子窗口实测量得的 (2369,1421)）全部无效，浮层就是不开；键盘序列一次成功。成品 `D:\tmp\s7-smoke\s7-step3.ps1`（含图标饱和度采样：彩色≈17.5 vs 灰≈1.1）。chevron 的矩形可由 Shell_TrayWnd 的 TrayNotifyWnd 下首个空文本 `Button` 子窗口 GetWindowRect 得到（find-chevron.ps1）。
  14. **GetRect 在浮层内格子重排后可能返回滞后的旧格**（S7 实测：restart 后格子漂了，GetRect 仍回旧位、拍到纯白空格 bright=248）——浮层整体截图 + 人工/视觉确认图标在场，比单格采样更稳。
- **并行 ZCode 会话共用真机是"启动即退出"的另一来源**（S4 实测）：对面会话分离式启动的实例占住 single-instance 锁，我的测试实例被静默弹回（无任何日志）。**每个测试阶段前都清点 `Get-Process dsh-desk`**，不能只在开工时查一次；对工作树的共享编辑（lib.rs/docs/AGENTS.md）每次写前重读。
- **并行会话在场时,环境谜题排障第一步是"读对面刚落盘的东西"**（S2 教训，浪费约 40 分钟）：托盘图标"消失"、6002 失效等怪象排查前，先 `ls docs/` 看有没有并行会话的新验证记录、`tail dsh-desk.log` 找外国实例轨迹指纹（例：日志里 `WebView2 runtime 118.0.9999.0` = S4 的注册表测试正在进行；低代数号的 `superseded generation` = 别的会话的新实例）。共享真机上，docs/ 与共享日志是排障第一现场。
- **残留的 `pnpm tauri dev` 监视链会复活实例**：改源码触发 watcher 重编译重启，吃掉新构建并干扰测试。开测前 `tasklist` 查 `pnpm→node tauri.js→cargo→dsh-desk` 链整树 `taskkill /T /F`；杀 pnpm 前先认命令行，别误杀用户自己的 `pnpm dsh --profile web`。
- **single-instance 会让测试 exe 立即 `exit 0`**（礼让给旧实例并 show+focus）——看到"启动即退出"先查旧实例（含并行会话的），不是崩溃。
- **进程清理一律 `taskkill /PID x /T /F`**：PowerShell `Stop-Process -Force` 不杀子树，会留 dsh 孤儿污染 Quit 清点（曾两次误判成产品泄漏）。但注意下一条——强杀风暴有副作用。
- **强杀实例后 WebView2 首导航偶发 `tauri.localhost`→"127.0.0.1 拒绝连接"错误页**（脏 profile）：杀净 `msedgewebview2` 再启动即愈；错误页上 eval 仍可执行（`location.replace` 能把窗口导航走）。
- **强杀风暴还会弄脏 EBWebView 的 cookie 库 → 窗口 401**（"dsh web authentication required"，2026-09-05 用户实测复现；同页 SameSite=Strict 认证 cookie 存不进/发不出）。服务端无头链路（token→303+Set-Cookie→带 cookie `/`→200）全通，唯独 webview 401 = 本症。**处置**：退出应用 → 只杀 `dshdesk` 名下的 msedgewebview2（按命令行过滤，别 `/IM` 全杀）→ 删 `%LOCALAPPDATA%\com.whyiyhw.dshdesk\EBWebView\Default\Network\Cookies` 与 `Cookies-journal` → 重启。**判定 GUI 真在线的金标准**：`netstat` 看到 `msedgewebview2 ↔ node:<port>` 有 ESTABLISHED（GUI 的 mux WS，握手需有效 cookie）——"AX 树无 401 文本"不可靠（窗口最小化时正文不暴露，曾两度误判）。**2026-09-05 补充：干净 Quit 周期也会复现**（Phase 3 会话多轮干净重启后 WS 无建立、窗口标题停在 "DSH Desk"）——触发面不止强杀风暴，长测试序列后若 WS 不建立直接按上法处置即可，一律以 netstat 为准。

### S9/S13 与发布链路补充（Phase 3 交付沉淀，2026-09-05）

- **tauri-plugin-window-state 的默认 flags 是陷阱**：`all()` 含 VISIBLE 与 MAXIMIZED，两个都会在启动期把 `visible:false` 的窗口拉出来（VISIBLE→`show()+set_focus()`；MAXIMIZED→`maximize()`→Win32 `SW_MAXIMIZE`＝激活并显示）。dsh-desk 只保 SIZE|POSITION。另：插件的 `Moved` 处理器不设最大化守卫，最大化会话会把缓存位置写成显示器原点，恢复时窗口落在 (-9,-9)——无害已知瑕疵。
- **启动期文件副作用（轮转/截断）必须放在 single-instance 判定之后**（setup 内）：第二实例在 builder 期 single-instance init 里 `exit(0)`，到不了 setup；放 run() 顶部会让二次启动把运行中实例的活日志腰斩进 `.old`。
- **就绪计数基线会被 S13 日志轮转作废**：启动前取的 `'dsh web: http'` 计数基线在轮转后（新日志从零计）永远追不上，launch 类脚本必须在启动后（等轮转落定）重取基线；就绪与可见的先后判据用"首次可见瞬间日志里是否已有就绪行"，别用两个轮询时间戳相减（同周期内先盖 visible 戳是测量伪影，曾 1ms 假违约）。成品 `%TEMP%\p3-smoke\launch-probe.ps1`。


### 本机 Hyper-V 组件库损坏与 VM 排障仪器（Phase 2 虚机门禁取证沉淀，2026-09-05）

> 完整复盘见 [docs/postmortem-2026-09-05-host-hyperv-broken.md](docs/postmortem-2026-09-05-host-hyperv-broken.md)。一句话：**本机 Hyper-V 载荷停留在 2020 版**（vmms/vmcompute 19041.320、vmbus.sys RTM .1、vmbusroot.sys 缺失，内核 .6456）——任何 guest 活不过 90 秒（Gen1 冻结、Gen2 Worker 18508 自关机）、心跳永不连，Docker Desktop 的 VM 自 09-01 起报同款 33101；DISM/SFC 报健康、功能禁用重启用重展开同版旧货、重启无效。**Phase 2 虚机门禁已按用户决策放弃执行**，想跑 VM 相关验证先修宿主（就地修复升级）或换机。

- **「所有 guest 都活不过一分钟」= 先做宿主二进制版本审计**（`Get-Item vmms.exe/vmcompute.exe/vmbus.sys/ntoskrnl.exe` 的 VersionInfo 一组对比），这轮它本可以是第一条命令。
- **guest 屏幕 ground truth = WMI `GetVirtualSystemThumbnailImage`**（root\virtualization\v2；成品 `D:\tmp\vm-gate\vm-gate-diag7c.ps1`）：vmconnect 窗口会缩放/掉线/自退出，全不可靠；帧对比用逐像素 diff，相同字节=内容未变（CDN 上传也按字节去重，喂视觉前先重编码换名）。
- **Worker 事件 18508/18514 = "谁杀了 VM"的第一手证据**；WinPE 冻结定位用「startnet 插桩 + 日志落 VHDX 数据分区 + 宿主挂盘回读」（RAM 盘 X: 的日志断电即失）。
- **免交互装 VM 的正道 = WinPE(boot.wim idx1) apply 进 VHDX + startnet.cmd 探盘符拉 `setup.exe /unattend:`**，配 IMAPI2 自制应答数据 ISO；El Torito "press any key" 抢键（AppActivate/SendKeys/物理点击）三种姿势全不稳，别浪费时间。
- 离线部署 BCD 的 `{default}` device/osdevice 必须改 `partition=C:`（guest 上下文盘符），否则 0xc000000f 黑屏。
- 宿主 Startup 文件夹受 Defender 受控文件夹访问保护（提权 Copy-Item 也拒）；沙箱对用户配置目录只读——自启用 HKCU Run 键，文件一律先落 D:\tmp。
- `D:\tmp\vm-gate\` 保留全套（脚本/ISO/安装包/全程日志），换机跑同一门禁直接复用；runbook 见其中 RESUME.md。


### S2 生命周期代数（S2 交付沉淀，2026-09-05）

- lib.rs 生命周期语义自此由代数标记统治：spawn/主动 kill/已上报退出各 mint 一代，watcher/timer 只认本代；Restart 与 Retry 同走 `run_lifecycle_cycle` 串行路径（杀净→等退→再起）。**日志里 `superseded generation (N)` 的 N 单调递增，可反推生命周期轨迹**——排障时它是"谁杀了谁"的第一手证据。
- 串行化语义（"未杀先起"禁止）耦合 AppHandle 无法单测，防线 = 真机 Restart 风暴 + Quit 零残留清点（S2 验收判定的做法，见验证记录）；纯状态代数部分有 4 条 cargo test 锚定。
- 回合末审查曾抓到 **P1**：child 交接若不在 claim_exit 同一临界区完成，微秒窗口内旧 watcher 可把 Child take 走直接 drop（无 taskkill）——"EOF ≈ 进程退出"只是假设（cmd shim 场景不成立），交接原子性是 Quit 零残留的隐性前提。改动生命周期代码时此不变量必须保持：**bump、take、清 url 三件事要么同临界区，要么有明确的所有者交接**。
- S2 验收记录见 [docs/verification-2026-09-05-S2.md](docs/verification-2026-09-05-S2.md)。

### S6 图标管线与 cursor-agent CLI（S6 交付沉淀，2026-09-05 深夜）

- **替换 `src-tauri/icons/*` 后必须验 exe 内嵌图标**：embed-resource 按 `.rc` 文本缓存 `.res`，文本不变就复用**旧图标**资源——`pnpm tauri build` 照样 exit 0 但 exe 嵌的是旧的。`cargo clean --release -p dsh-desk` 后重建即愈（依赖缓存不受影响）。判定手段：`[System.Drawing.Icon]::ExtractAssociatedIcon($exe)` 抽出来目视，别只看构建 exit 0。
- `pnpm tauri icon <svg>` 直接吃 SVG（内置 resvg），默认**额外产出 `ios/`、`android/` 子目录**，Windows 优先项目每次生成后记得删。
- **视觉类交付（图标/UI/截图）必须由"能看见的复核者"逐轮目检**：cursor-agent 盲画字母贝塞尔两轮自报成功（v1 读作 "D21"、R1 读作 "bsP"），全靠看图推翻；改用 opentype.js 提取系统字体轮廓（`C:\Windows\Fonts\segoeuib.ttf`）一次收敛。agent 自报成功 ≠ 正确。
- cursor-agent CLI：官方二进制在 `%LOCALAPPDATA%\cursor-agent`（已登录）；npm 的 `cursor-agent` 包是同名第三方库（`bin` 为空），勿装。无头用法 `-p "…" --force --trust --output-format stream-json`；它会加载 `~/.cursor/mcp.json`，其中 `ones`（mcp-remote OAuth）会**卡死无头会话**——已在 CLI 侧 `mcp disable` 两个服务器（IDE 不受影响，恢复用 `cursor-agent mcp enable <名>`）。判断它是否真在干活：看盘上文件时间戳 + stream-json 事件，**别看主进程 CPU**（常年接近 0，worker 才干活）。
- S6 验收记录见 [docs/verification-2026-09-05-S6.md](docs/verification-2026-09-05-S6.md)；真机托盘实时目视与开始菜单目视已于 2026-09-05 在 S3 安装包上补齐（exe 内嵌图标抽取 + 溢出浮层截图，见 [verification-2026-09-05-S3.md](docs/verification-2026-09-05-S3.md)）。

### S3/S12/S5a 发布链路（Phase 2 交付沉淀，2026-09-05）

- **发版流程**：三处版本（tauri.conf.json 真源 / Cargo.toml / package.json）bump 一致 → commit main → `git tag vX.Y.Z && git push origin vX.Y.Z` → release.yml 自动全量构建并把 NSIS/MSI 挂上 Release（build≈9 分钟冷缓存）。**CI 守卫会拒绝 tag 与树内版本不符的构建**——这是特性不是障碍。
- **prerelease 验收 fixture 的正确姿势**（不能随手打 tag）：开分支 bump 版本 → push → `gh release create vX.Y.N --target <分支> --prerelease` → tag 推送自动触发 cascade 构建，softprops 对预建 release **保留 prerelease 标记、只追加资产**（实测）→ 测完按 **release → 远端 tag → 分支** 顺序删。
- **PowerShell 5.1 的 `Set-Content -Encoding UTF8` 写 BOM**，会把 JSON 弄坏（`JSON.parse` 遇 `﻿` 直接崩）——改写文件用 `[System.IO.File]::WriteAllText($f, $t, (New-Object System.Text.UTF8Encoding $false))`。
- **Git Bash 里 `$TEMP` 是 MSYS 路径（/tmp/...）**，直接传给 `powershell -File` 会报"文件不存在"——先 `cygpath -w` 转 Windows 路径；同理复杂的 PowerShell 内联命令别在 bash 双引号里写（`$_`、反引号转义连环坑），写 .ps1 文件再 `-File` 执行。
- **NSIS 静默卸载**：`uninstall.exe /S "_?=C:\...\dsh-desk"`（`_?=` 必须绝对路径且为最后一个参数，使卸载器原地同步运行，`-Wait` 才有意义）；卸载器删不掉自身所在目录（NSIS 常态），`Remove-Item` 手动清。Tauri NSIS 默认 currentUser 安装到 `%LOCALAPPDATA%\dsh-desk`。
- **embedBootstrapper 语义**（官方 schema）：装机装运行时**仍需联网**，只是内嵌引导器（+~1.8MB）；真离线只有 offlineInstaller（+~127MB，未选）。README 已如实描述。
- **本会话视觉复核链路再次确认**：Read 本地 PNG → 返回 CDN URL → `analyze_image`（icon 与浮层截图两张都走通）；托盘 20px 图标的定性描述（"蓝块+dsh 字样"）够用，坐标断言仍然不可信（第 7 条不变）。

## 调试方法论教训（本次事故浓缩）

1. **先二分，再动手**。服务端嫌疑先用无头手段排除：用 curl/node 完整复现浏览器链路（`/?token=` 换签名 Cookie → 带 Cookie 打 `/api` → 开 `ws://127.0.0.1:<port>/api/remote.mux` 握手）。服务端全通 → 问题必然在客户端运行时。本次排除服务端只花了 10 分钟，避免了瞎改 Rust 代码。
2. **"浏览器正常、嵌入壳异常" = 先查壳的内核版本**，候选顺序：WebView2/CEF 版本 → 代理/网络 → 代码时序。别一上来怀疑自己的代码——本次代码从头到尾没 bug。
3. **尽早向用户要 Console 报错**。一条 `AbortSignal.any is not a function` 直接钉死根因；在此之前我推演过"启动时序竞态""代理拦截 WS"两个错误假设，全是浪费。
4. **别轻信文件扩展名和官方工具的行为**：微软 fwlink 给的"standalone installer"是 EXE；bootstrapper 会因"已安装"拒绝干活。下载后看文件头（`4D 5A`=EXE，`D0 CF 11 E0`=MSI）再选工具。

## 收尾反思机制（做完 ≠ 交付，每个任务收尾必过）

> 起因：S1 交付时代码与真机验证都完成了，但自验记录和踩坑台账只在对话里"口头存在"，靠用户两问（"文档更新了吗？""踩坑记录更新了吗？"）才补落盘。以下清单固化为例行步骤，报告"完成"之前先自查。

1. **验收对照落盘**：逐条对照 spec §2.3 该项验收标准，把"怎么测的、证据在哪、哪些没测到"写进 `docs/verification-<日期>-<S项>.md`，**并在 `docs/README.md` 索引表加行**（S2 交付时漏过一次，靠用户追问才补）；对话里的报告不算数。
2. **台账回写**：本轮新踩的坑 / 新环境事实写回本文档对应小节；**被本轮推翻的旧结论当场修正**（例：曾记录"托盘菜单无法程序化触发"，后经 6002 回调方案推翻），不留错误记录误导后人。
3. **规范同步**：范围/验收有变 → 更新 `docs/spec-and-plan.md` 并在 §5 加行；没变也在 §5 加一行交付事实（指向验证记录）。
4. **环境还原清点**：测试用 config 已还原、无孤儿 dsh/node 进程（§2.4 第 6 条口径）、`%TEMP%` 下的注入脚本/假程序注明位置与用途。
5. **一句反思**：哪个环节因错误假设/弱证据浪费了时间，浓缩成一条可执行的教训（进本文档或验证记录的"方法备忘"）。

## 异常的下落（约定执行四级梯度）

> 总纲：**每个异常，要么被拦住，要么变成规矩**——拦住是反馈（当场捕获），变成规矩是前馈（下次同类异常无从发生）。两个结局都没有的，不算处理完。本仓库的四级落位：

1. **注入**（开工自动加载）：本文档 + dsh-desk-plan skill 硬约束。只放红线与事实，细则归 docs，别往这里堆。
2. **审查**（回合末）：S 项代码交付前，开一个**无作者上下文的新会话/子代理**独立审查——喂本次 diff + skill 硬约束 + 本文红线 + 三问（违反约定没？文档过时没？生命周期/并发风险没——S2 类 bug 主战场），只输出带位置的问题清单，改不改作者定。同一 AI 自查几乎查不出问题，它带着答案查自己。
3. **测试固化**（违规直接红）：约定一旦"写了代码才暴露"就固化成 `cargo test`（例：S1 的就绪行解析/URL 脱敏各有测试锚定）。
4. **hook 硬拦**（错一行就污染仓库的才用）：`.githooks/pre-commit` 只跑 `cargo fmt --check` 秒级快检，**绝不跑构建/测试**（慢检查会被绕过，等于没有）；失效处置 fail-open（工具缺失/报错警告放行，CI 兜底），只有 fmt 明确不过才拦。激活方式：`git config core.hooksPath .githooks`（本机已配）。

**环境自检**：`scripts/env-check.ps1` 把本文档"本机环境事实"里可机检的条目（WebView2 ≥119、config `--port 0`、无残留实例、门禁激活、cargo 可用）变成命令——真机验证前先跑一遍，"记住"变"拦住"。脚本含中文必须带 UTF-8 BOM（PowerShell 5.1 无 BOM 按 ANSI 解码，语法直接崩）。

## 并行开发约定（worktree）

- **多任务/多 agent 同时开工时，一个任务一个 worktree**，禁止多个 agent 共用同一工作树（工作区 diff 会互相污染）。主工作树（仓库根）负责 main 集成与串行主线开发（S1→S2）。
- **前置纪律**：worktree 只包含**已提交**的内容，未跟踪文件不会跟着走——开新 worktree 前先确认 `docs/`、`.agents/` 等已提交（即 Phase 0 完成）。计划文档与 skill 只在 main 上修改，各 worktree 勤 rebase 主线获取最新规范。
- **可并行**（文件不相交，可随时提前开独立分支）：S3 CI（纯新增 `.github/workflows/`）、S6 图标（替换 `src-tauri/icons/`）、S12 发布物料（README + 版本字段策略）。
- **须串行**（都动 `src-tauri/src/lib.rs` 的生命周期区域）：S1 → S2 → S4 / S5a。S2 依赖 S1 的定时器语义，顺序不可换，一律在主工作树按序做。
- **编译可并行，真机运行必须串行**：本应用是单实例 + 托盘常驻 + 全局热键，全局状态在 `%APPDATA%\dsh-desk\`（config.json、日志）。同时跑两个构建，第二个会被 single-instance 弹回第一个，且互相踩配置/日志/热键注册。dsh 服务端端口冲突已由 config.json 的 `--port 0` 解决（见上文），但应用层冲突仍在；且真机验证本身需要完整权限环境（见上文沙箱事实）。同一时间只验证一个实例，验证完彻底退出（确认无 dsh/node 残留进程，spec §2.4 契约第 6 条）再测下一个。
- **构建成本**：每个 worktree 独立 `target/` 与 `node_modules`，冷构建约 2 分钟/处。只在当前要做真机验证的 worktree 跑完整 `pnpm tauri build`，其余 worktree 停在 `cargo check`。
- **合并**：短命分支，完成即 rebase main → 合并 → 删除分支并 `git worktree remove <path>` 清理。

## 其他备忘

- `ws-probe.cjs` 是本次的服务端探针脚本（token→cookie→WS 握手一条龙），排查连接类问题可直接复用；`ws` 包在 harness 仓库 `packages/api/gateway/node_modules/ws`，不在本仓库。
- 用户级 EdgeUpdate 日志：`%TEMP%\MicrosoftEdgeUpdate.log`；MSI/EXE 详细日志加 `/l*v`。
- **「浏览器裸 URL → 401」是设计行为，不是 bug**（用户实测提出，社区大概率复问）：dsh 服务端要求首访带 token（`/?token=…` 换 30 天 cookie），裸 `/` 必 401 "dsh web authentication required"。桌面壳窗口内部自动完成交换（免手工）；外部浏览器走**托盘 Open in browser**；S1 脱敏后日志/终端拿不到 token URL（故意，token 入日志=泄露）。做支持/排障先分清用户看的是壳窗口还是外部浏览器。FAQ 已落地 README（2026-09-05，S12）。

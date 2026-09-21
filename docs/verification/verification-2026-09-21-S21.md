# S21 真机验证记录（窗口 caption 断缝 / 无边框双轨）— 2026-09-21

- 验证人：agent（ZCode 会话）
- 被验构建：本地 `pnpm tauri build` NSIS 安装版（基线 = main `9bc70fc` + 本轮 S21 修复，树内版本 0.3.0→0.3.1 过渡期）
- 机器：本机（Windows 10 19045，125% DPI，WebView2 运行时 152）
- 结论：**修复后全项通过；修复前抓到 P0（首启必死锁）——已发布的 v0.3.0 对全新安装不可用，由 v0.3.1 取代**

## 0. 验证过程抓到的缺陷（先于通过项，按发现序）

### P0：setup 内 `with_webview` 同步自死锁（v0.3.0 首启必挂）

- 症状：全新安装 v0.3.0 启动后，日志停在启动 banner，无 "started"、无就绪行；主线程 `Responding=False`；窗口/托盘/热键辅助窗均已创建，WebView2 子进程已起，但 `boot_server` 永不执行。
- 根因（vendored 源码级实证，`tauri-runtime-wry-2.11.4/src/lib.rs` `send_user_message`）：**setup 回调运行在主线程上，而 `with_webview` 从主线程调用时闭包被同步就地执行**；闭包内对 WebView2 COM 面（`controller().CoreWebView2()` / `Settings9::SetIsNonClientRegionSupportEnabled`）的调用要等事件循环泵消息才能完成，而泵正被 setup 占住 → 自死锁。S21 原实现把 `init_borderless_chrome` 直接放在 setup 里，正好踩中。
- 修复：探测挪到 `std::thread::spawn` 旁路线程——同一条 `with_webview` 从非主线程调用走 proxy 投递，在泵着的循环上执行；探测结果（`set_decorations`）仍远早于就绪行显窗（规格硬约束"无边框切换只发生在窗口可见前"保持成立，实测探测日志先于 "started" 落盘）。
- 判定：修复后连续 4 次冷启动全部正常到达就绪行（38→39 等计数增量），零挂起。

### P1：关闭按钮 CSS 缺 `right: 0`

- `#tb-min`/`#tb-max` 有 `right` 偏移而 `#tb-close` 没有——absolute 定位 + 无水平偏移 = 静态位置，按钮会飘到条带左端。纸面推理发现，修复（补 `#tb-close { right: 0; }`）后经启动面板截图实测三键位置正确（min 距右 88px、max 44px、close 0px）。

### 记录性偏差：tao 0.35.3 的 `set_decorations(false)` 不摘 WS_CAPTION 样式位

- spec §2.3 S21 验收原文写"GetWindowLongW 无 WS_CAPTION"。实测：`set_decorations` 返回 `Ok(())` 但 GWL_STYLE 前后均含 WS_CAPTION（0x04CF0000）；tao `window_state.rs` `to_window_styles()` **无条件** `style |= WS_CAPTION`（仅 CHILD/全屏分支例外）。手工剥位实验证明摘位与否不影响行为。
- 但行为层面无边框完全成立：原生标题条不可见（见下），app-region 拖拽/双击最大化/右键系统菜单全部工作，wry 的 `TAURI_DRAG_RESIZE_BORDERS` 透明子窗提供边缘 resize。**验收判据以行为证据取代样式位判据**（更强：样式位只是手段，本条记录偏差防后人复查时误判）。

## 1. 通过项（全部真机实测）

| # | 验收项（spec §2.3 S21） | 方法 | 结果 |
|---|---|---|---|
| 1 | 探测成功：无边框生效、GUI 侧栏自 y=0 起 | CopyFromScreen 窗口区截图 + 像素采样（x=100/200，y=2/26/40） | **PASS**：y=26/40 全 `#F9FAFB`（GUI 侧栏灰），y=2 为 GUI 自有元素色，无 `#FFFFFF` 原生 caption 带、无接缝；视觉模型复核整图确认无标题栏 |
| 2 | drag 拖拽 | DPI 感知脚本 SetCursorPos+mouse_event 在条带 y+20/y+30 按拖 | **PASS**：窗口精确移动 (120,70)×2 组；y+5 处为 resize 带（wry 边框辅助窗顶边），属正常边缘行为 |
| 3 | 双击最大化 | 条带 y+20 双击 | **PASS**：窗口变 (-9,-9) 2578×1420（最大化） |
| 4 | 右键系统菜单 | 条带 y+20 右击 + EnumWindows 找 #32768 | **PASS**（非客户语义完整） |
| 5 | 自家页三键 | config 临时指向缺失命令使面板常驻 → 三键逐一点击 | **PASS**：min（iconic）/ max（最大化）/ close（隐藏到托盘）；三键位置经截图确认（含 P1 修复） |
| 6 | 关闭=藏托盘（与原生 X 同契约） | 三键 close 测试 + 此前 S23 已验 CloseRequested 路径共用 `hide_to_tray` | **PASS** |
| 7 | A 轨 caption 调色 | DWM 调用逐 HRESULT 日志 | 无失败日志（B 轨生效后 A 轨不可见，符合设计） |
| 8 | §2.4 抽查：热键双向 | keybd_event Alt+Shift+D ×2 | **PASS**（隐藏→显示） |
| 9 | §2.4 抽查：二次启动弹回 | 再起 exe | **PASS**：banner 计数零增量（零文件副作用）、单进程、原窗口被唤醒 |
| 10 | §2.4 契约 1：就绪→认证 GUI | 窗口内容截图（金标准） | **PASS**（会话列表可见，非 401） |

- §2.4 契约 2（托盘六项）与契约 6（Quit 零残留）**未在本轮重跑**：该路径代码与 S23 Phase B 全过的构建完全一致（S21 不触碰进程生命周期），引用 [verification-2026-09-06-S23.md](verification-2026-09-06-S23.md)；本轮多次 `taskkill /T /F` 树杀后清点均无孤儿。
- 探测失败分支（旧运行时回退原生 caption + A 轨）：模拟困难，未实机触发（spec 已预见此边界）；同构论证=探测三重 `let Ok ... else return false` 均走"保持原生 caption"路径，A 轨 DWM 调用与启动时同源。

## 2. 仪器与脚本

全套在 `D:\tmp\s21-verify\`：launch-style（启动+样式枚举）、pixel-probe（截图+像素）、drag-offsets / dblclick-menu（拖拽/双击/菜单）、catch-starting（启动面板抓帧）、button-test2（三键，配合 config 临时劫持）、hotkey-bounce、hijack-config（+备份 config-backup.json）。教训沉淀见 AGENTS。

## 3. 环境还原

- config.json 已从备份恢复（node + `--port 0` 原样）；`.window-state.json` 已删（测试期坐标污染，重置为默认几何）
- `%LOCALAPPDATA%\dsh-desk\instances\`（S23 work 测试剖面）已清；最终安装 = 修复版 0.3.1 候选（本地构建）
- 进程清点：dsh-desk 0 残留；webview 子进程随树清空
- v0.3.0 的 Release 页已补首启死锁告示，v0.3.1 取代之

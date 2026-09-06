# 复盘：桌面壳"连接异常"——WebView2 运行时冻结在 114

- **日期**：2026-09-04
- **症状**：dsh-desk 桌面窗口里 GUI 能渲染，但侧边栏一直显示黄色"连接异常"；同机浏览器打开同样的地址完全正常。
- **根因**：本机 WebView2 运行时冻结在 **114.0.1823.43**（2023-06，Evergreen 自动更新失效），而 dsh 前端使用了 Chromium 116+/119+ 才有的 JS API，页面脚本在 WebView2 里启动即崩。
- **状态**：已修复（运行时强制升级到 152.0.4191.62，应用重启后恢复）。

## 时间线与证据链

1. **背景**：`pnpm tauri dev` 启动 dsh-desk；`config.json` 未指定端口，dsh web 默认端口 3080 与常驻开发实例冲突（EADDRINUSE）。先修复为 `--port 0`（OS 挑空闲端口），应用能起来并导航到 GUI——但用户看到"连接异常"徽章。
2. **假设 1（错误）：启动时序竞态**。就绪行 `dsh web: <url>` 只代表 webserver 端口开始监听，100+ 插件树可能还没挂载完，首连失败 → 退避重连期被截图。**被推翻**：徽章持续不消失，且组件文档写明首次连接失败会在服务端就绪后自愈。
3. **假设 2（错误）：代理拦截 WebSocket**。查 WinHTTP/WinInet/环境变量：`ProxyEnable=0`，直连。排除。
4. **服务端全链路实测（关键步骤）**：写 `ws-probe.cjs` 模拟浏览器完整流程——
   - `GET /?token=…` → 303 + Set-Cookie（`browser-auth.ts` 的签名 Cookie 机制，按 authority 绑定）✅
   - 带 Cookie `GET /api` → 404（路由 404，非 401/403，认证通过）✅
   - 带 Cookie + Origin 开 `ws://127.0.0.1:<port>/api/remote.mux`（Gateway 物理载体）→ **握手成功** ✅
   - 结论：服务器、认证、信任围栏（`api-request-trust.ts`）、WS 全部正常，问题必然在 WebView2 客户端侧。
5. **决定性证据（用户提供 Console）**：
   ```
   Uncaught TypeError: Promise.withResolvers is not a function      （Chromium 119+）
   TypeError: AbortSignal.any is not a function                      （Chromium 116+）
       at RemoteStream.read (remote-stream.ts:108)
       … [session-controller] control stream failed
   ```
6. **版本核对**：注册表 `HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` → `pv = 114.0.1823.43`；同机 Edge 浏览器 151。完全吻合。

## 修复过程（含每一步的坑）

| 尝试 | 结果 | 教训 |
|---|---|---|
| 官方 bootstrapper（fwlink 2124703）`/silent /install` | 失败 `0x80040828`：*"The Microsoft Edge Webview2 Runtime is already installed"*——检测到已装就拒绝干活，不做升级 | bootstrapper 只管"从无到有" |
| `MicrosoftEdgeUpdate.exe /ua /installapp appguid={F3017226-…}` | 跑了 2 分钟无任何变化，用户级日志无痕 | 不可靠，别浪费 时间 |
| fwlink **2124701**（x64 独立包）下载后用 `msiexec /i` | 失败 1620，日志 `2203 / 0x80030050` | **文件头是 `4D 5A`——是 EXE 不是 MSI**。按扩展名/直觉选工具会浪费一轮 |
| 该 EXE 直接 `/silent /install` + `-Verb RunAs`（用户点 UAC） | **成功**，exit 0，`pv → 152.0.4191.62` | ✅ 唯一可靠路径 |
| 杀光 `msedgewebview2` + `dsh-desk` 进程后重启应用 | 新进程树用新运行时，GUI 恢复正常 | 旧 WebView2 进程不会热替换，必须全杀 |

> 有价值的弯路记录：`msiexec` 报的 `2203/0x80030050`（共享冲突）与真实原因（拿 msiexec 开 PE 文件）毫不相干，纯误导。**下载安装包后先看文件头再选工具**：`4D 5A`=EXE，`D0 CF 11 E0`=MSI。

## 症状-根因为何相距这么远

"连接异常"徽章只报告**结果**（浏览器侧与后端连接不健康），不报告**原因**。页面 JS 在 `Promise.withResolvers` 处第一行就崩，会话控制流根本没建立——但从 UI 看与"网络不通"完全同貌。GUI 能渲染是因为外壳 bundle 先于崩溃点执行。这解释了为什么服务端五项检查全绿却依然异常。

## 防复发建议（未实施）

1. **dsh-desk 启动时检查 WebView2 版本**：`lib.rs` 读上述注册表 `pv`，低于基线（建议 ≥ 120）就在窗口里直接渲染"运行时过旧 + 下载链接"，而不是让用户面对玄学徽章。成本约 20 行。
2. **前端把 "使用了不支持的浏览器 API" 做成显式检测**：入口处 feature-detect `AbortSignal.any` / `Promise.withResolvers`，缺失时渲染一整页明确提示，代替半渲染 + 徽章。
3. Evergreen 失效的机器（更新服务被关/冻结镜像）不少见，桌面壳类产品值得把"内核版本门槛"当成一等公民处理。

## 顺带修复的另一个问题（同日）

`config.json` 未指定端口导致 `EADDRINUSE`（3080 被常驻实例占用）。修复：args 加 `--port 0`。壳只解析 stdout 的实际 URL，端口漂移无影响。**这是必配项，别删。**

## 复用资产

- `ws-probe.cjs`：服务端连接探针（token→Cookie→`/api`→Gateway WS 握手一条龙）。`ws` 依赖在 harness 仓库 `packages/api/gateway/node_modules/ws`，本仓库没有。
- EdgeUpdate 日志位置：`%TEMP%\MicrosoftEdgeUpdate.log`（用户级）/ 详细安装日志加 `/l*v <path>`。

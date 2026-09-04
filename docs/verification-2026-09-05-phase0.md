# Phase 0 交付自验记录（2026-09-05）

> 对象：Phase 0 · 立整基线（规范见 [spec-and-plan.md](spec-and-plan.md) §3 Phase 0 行）
> 构建：`pnpm tauri build` → release exe + MSI + NSIS 三产物（`src-tauri/target/release/bundle/`）
> 冒烟对象：`src-tauri/target/release/dsh-desk.exe` —— S1+S4+S6+S2 合并态的**首次 release 构建冒烟**（此前各 S 项验证均在 debug 构建）
> `cargo test`：10 passed（0.02s）
> 环境：Windows 10 19045，WebView2 152.0.4191.62，真实 config（node 源码 checkout + `--port 0`），`scripts/env-check.ps1` 五项全绿；开测前进程清点无 dsh-desk/webview 残留、无浏览器
> 备注：冒烟期间用户在场使用机器——对托盘驱动稳定性的影响与对策见"托盘驱动配方 v3"

## Phase 0 交付清单

| 交付项 | 怎么做 | 结果 |
|---|---|---|
| 处置 `ws-probe.cjs` | 原文件硬编码认证 token + 本机绝对路径（skill 硬约束 #2：token 不得进发布物，本仓库公开）→ 参数化（token/base 经 argv，`ws` 经 NODE_PATH 借自 dsh checkout），`node --check` 过 | ✅ 已提交脱敏版；原 token 未进 git 历史 |
| 提交未跟踪内容 | 分五笔逻辑提交（文档与工具 / S1+S4+S2 代码 / S6 图标 / CI / 验证记录），工作树含此前并行会话交付但未提交的 S1/S2/S4/S6 全部产物 | ✅ git status 干净 |
| 真机构建 + 冒烟 | release exe 按 §2.4 六条契约逐条走查（下表） | ✅ 全过 |
| push CI（bare cargo check） | `.github/workflows/check.yml`：windows-latest，push 触发，Swatinem 缓存 | 见"CI 结果"节 |

## §2.4 六条契约（release 构建，逐条可判定证据）

| # | 契约 | 证据 |
|---|---|---|
| 1 | 就绪行打印后窗口显示并导航认证 GUI | 启动后 **10.3s** 出就绪行（port 11932，日志行已脱敏仅 scheme+host+port）；窗口 vis=True；AX 树文档 = `127.0.0.1:11932 - Web content` + 聚焦 textarea（聊天输入框）；netstat：`msedgewebview2 ↔ 127.0.0.1:11932` **ESTABLISHED ×2**（AGENTS.md 金标准） |
| 2 | 托盘四项菜单全部可用 | **Show**（菜单行 1，via6002 一次命中，窗口 False→True）/ **Open in browser**（行 2，日志 `opened the authenticated URL in the browser` + Chrome 实际打开认证页，取证后即关闭）/ **Restart**（行 3：旧 pid 23956 的 watcher `superseded generation (1)` 静默退出 → 新 pid 21084 起动 → 新就绪行 port 13412 → webview 双连接迁至 13412；全程 dsh-desk 恒 1 个进程）/ **Quit**（行 5，一次命中，进程退出） |
| 3 | 热键 Alt+Shift+D 从任意应用切换窗口 | 合成 chord（keybd_event Alt+Shift+D）：窗口 vis **True→False→True**（EnumWindows 判据） |
| 4 | 二次启动唤起并聚焦已有窗口 | 先藏窗口 → 第二实例 **0s 退出 code 0**（礼让）→ 原实例存活（count=1）且窗口被唤起 |
| 5 | 关窗 → 隐藏到托盘，server 继续运行 | PostMessage WM_CLOSE → 窗口 vis=False、app 存活（pid 21916）、node 子进程（23956）存活 |
| 6 | Quit 后无 dsh/node 残留进程 | 终局 census：dsh-desk **0**；`msedgewebview2`(dshdesk) **0**；node 仅用户自有 3 个（3080 开发实例 20800 / editor 18932 / astro 22956）+ pnpm 4 个全程未动 |

## 托盘驱动配方 v3（本轮最大沉淀，修订 S4 配方 v2）

1. **uID 实测为 2**（扫 1..8：uid 1 返回 E_FAIL，uid 2 给出浮层内真实矩形 `(2221,1350)-(2295,1400)`）——别假设是 1，一律扫 1..8 取首个 `hr=0 且 T<1400` 的；
2. **浮层开着时 6002 直投是首选开菜单方式**：`PostMessage(tray_icon_app_hwnd, 6002, uID, WM_RBUTTONUP)`——**不动物理鼠标**，本轮 Show/Restart/Open-browser/Quit **四次全部 via6002 一次命中**；
3. 物理右键在用户在场时不稳（本轮四轮全 no-menu，即台账"用户在场菜单秒关"模式）——保留为兜底，不作主路径；
4. 6002 的前提仍是图标可解析矩形：**浮层开着时 `Shell_NotifyIconGetRect` 成功**（hr=0），浮层关着只给 chevron 位——先点开浮层再直投（与 S4 结论自洽：v2 的"6002 失效"实为浮层未开时 GetRect 失败静默 return）；
5. **`FindWindowW('Tauri Window')` 本机返回 0，但 EnumWindows+GetClassName 能看到该窗口**（跨 DPI 感知上下文的已知 Win10 怪癖）——窗口查找/可见性判据一律走 EnumWindows；
6. **`MainWindowHandle` 不能当可见性代理**：主窗口隐藏后会回退到 single-instance 插件的可见辅助窗口（类名 `com.whyiyhw.dshdesk-sic`），句柄非 0；
7. 菜单行点击仍按实测矩形 高/5 算行中心物理点击（菜单开着时稳定）；
8. 成品脚本：`%TEMP%\p0-smoke\p0-tray2.ps1`（6002 优先 + 物理右键兜底 + 效果验证重试 ≤4）。

## 已知边界 / 未测项

- MSI/NSIS 安装包未做安装验证——S3/Phase 2 的干净虚机"仅按 README 装包"验收范围。
- 托盘 Edit config（第 4 行）未重复走查——S4 验收项且其已验，不在 §2.4 四项内。
- 冒烟期间用户在场：浮层/菜单类操作被前台活动打断属台账预期行为，配方 v3 的效果验证重试把影响降为零（四次菜单动作全部一次命中）。

## 环境还原

- config.json 全程未动（真实配置直测）；无备份文件残留。
- 进程终态即契约 #6 的 census；测试触发的 Chrome 已清零。
- `%TEMP%\p0-smoke\`：本轮全部驱动脚本（p0-*.ps1）保留供复用；`%TEMP%\s4-test\` 仍在。

## CI 结果

**绿**。Run [33905299622](https://github.com/whyiyhw/dsh-desk/actions/runs/33905299622)（`acf78fe` push 触发，windows-latest，cargo check + Swatinem 缓存）：completed / success。唯一 annotation 为 actions/checkout@v4 的 Node 20 弃用提示（informational，非失败）。

附注：push 凭证走 gh 双账号（本机 keyring 同时存 oxpxo 与 whyiyhw，仓库属 whyiyhw）——`gh auth switch` 临时切活跃账号 + `git -c credential.helper='!gh auth git-credential'` 单命令内生效，推完即切回，全局配置零改动。

## 一句反思

可见性/坐标类判据要先在"已知态"上校准判据本身再进正式流程——本轮 `FindWindowW` 返回 0、`MainWindowHandle` 回退辅助窗口两个坑，各浪费一轮测试，全因拿未校准的判据当了 ground truth。

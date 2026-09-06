# S3 验证记录 — CI 与预编译安装包(2026-09-05)

> 交付物:`.github/workflows/release.yml`(tag/PR 全量构建)、`check.yml` 增 tags-ignore、`scripts/check-versions.mjs`(S12 版本守卫,构建前置)、`package.json` packageManager 锁定。
> 提交:`a9cbda5`(S3 工作流)+ `62e430b`/`8c34503`(同批 S12/S5a,守卫在其构建链上生效)。独立审查:见文末。

## 验收对照(spec §2.3 S3)

**验收 1:打 tag 后 Release 页出现可安装的 `.exe`/`.msi` — 通过**

- tag `v0.1.0` → run [33909476394](https://github.com/whyiyhw/dsh-desk/actions/runs/33909476394)(build 9m18s + release 8s,全绿);
- [Release v0.1.0](https://github.com/whyiyhw/dsh-desk/releases/tag/v0.1.0)(Latest,非 draft/prerelease)挂载:
  - `dsh-desk_0.1.0_x64-setup.exe`(NSIS,4,351,763 B)
  - `dsh-desk_0.1.0_x64_en-US.msi`(5,586,944 B)
  - release notes 由 `generate_release_notes` 生成。

**验收 2:新机器(无 Rust/Node、已装 dsh)装完可用默认配置跑通 — 本机部分验证,虚机门禁保持开放**

- 本机(有 Rust/Node,非干净机)安装冒烟全过:下载 release NSIS → `/S` 静默安装 exit 0 → 安装于 `%LOCALAPPDATA%\dsh-desk`(currentUser)+ 注册表卸载项 + 开始菜单快捷方式 → 启动后 **10.8s READY**(readiness line 新增,端口 5146)+ `msedgewebview2 ↔ node:5146` **WS ESTABLISHED**(金标准在线判据)。
- 未覆盖:干净虚机(无 Rust/Node)与"默认配置"(本机沿用既有 dev config,指向源码 checkout)。**该门禁留待虚机,spec Phase 2 门禁相应未关**。
- 卸载链路亦验证:`uninstall.exe /S _?=C:\Users\...\dsh-desk` exit 0,exe/快捷方式/进程全清(卸载器删不掉自身目录,属 NSIS 常态,手动清)。

## 工作流设计要点(与审查联动)

- **权限最小化**(审查 P2-4):build job 仅 `contents: read`(产出 upload-artifact);release job `needs: build`、仅 tag 触发、持 `contents: write`(download-artifact → softprops 挂载)。fork PR 永远拿不到写 token。
- **版本守卫前置**(S12):`check-versions.mjs` 在 install/build 之前跑——三处版本(tauri.conf.json 真源 / Cargo.toml / package.json)必须一致,tag 构建时 tag 必须等于 `v<版本>`。反向验证:本地 `v0.2.0` 拒绝(exit 1);fixture 分支(树=0.1.1,tag=v0.1.1)通过。
- **并发与超时**(审查 P3-12):`concurrency` per-ref `cancel-in-progress` + build 30min / release 10min 超时。
- **check.yml 增 `tags-ignore: ['v*']`**(审查 P3-11):打 tag 不再重复跑 cargo check。
- embedBootstrapper 构建行为:构建期下载 bootstrapper 内嵌(NSIS/MSI 双侧,tauri-bundler 支持),装机装运行时仍需联网——语义勘误见 S12 记录。

## §2.4 六条契约走查(在 v0.1.0 安装版上,2026-09-05)

前置:无 dsh-desk 实例、浏览器关闭、dev 实例(3080)与本会话工具链 node 进程记为基线。

| # | 契约 | 结果 | 证据 |
|---|------|------|------|
| 1 | 就绪行 → 窗口显示并导航认证 GUI | ✅ | 10.8s READY;WS ESTABLISHED ×2(webview→node:5146) |
| 2 | 托盘菜单全部可用(6 项) | ✅ | Show(隐藏→可见)/ Open in browser(Chrome 打开"DSH 本地构建"=认证后 GUI)/ Restart(就绪行 38→39 + 新端口 5314 WS ESTABLISHED)/ Edit config(日志"opened …config.json",编辑器弹出并关闭)/ Check for updates(见 S5a 记录)/ Quit(见 #6) |
| 3 | 热键 Alt+Shift+D | ✅ | vis True→False→True 两轮切换 |
| 4 | 二次启动唤起聚焦 | ✅ | 第二实例 0.1s exit 0,原实例存活且窗口变为可见 |
| 5 | 关窗 → 隐藏到托盘,server 继续运行 | ✅ | WM_CLOSE 后窗口隐藏、app 存活、node 子进程(3060)仍在 |
| 6 | Quit 后无 dsh/node 残留 | ✅ | 托盘 Quit 一次命中;dsh-desk 0 进程、应用子进程 3060 消失、无 dshdesk 名下 msedgewebview2;清点仅剩基线进程 |

托盘驱动:配方 v3(`%TEMP%\p2-smoke\tray.ps1`,6002 直投为主),**菜单行数参数化为 6**(Check for updates 第 5 行,Quit 第 6)。全部菜单动作一次命中(仅 Check for updates 首次效果检查早于 HTTP 返回,重试第 2 次确认,属预期节奏)。

## S6 遗留目视(依赖本安装包,一并完成)

- **exe 内嵌图标**:CI 冷构建产物抽取 → 蓝色圆角方块 + 白色小写 "dsh"(Segoe UI Bold 字形)——与 `design/S6-icon-brief.md` 定稿一致,非 Tauri 模板,资源缓存坑未复发。
- **托盘图标实时目视**:溢出浮层截图(左上角蓝块 dsh 图标,与 NVIDIA/微信图标并列)。
- 开始菜单快捷方式指向 exe 图标(同源,注册表 DisplayIcon 确认)。

## 独立审查(回合末第四道门)

无作者上下文子代理审查全 diff: **P1×0;P2×4 全部修复**(prerelease 后缀语义、spec 端点回写、embedBootstrapper 语义勘误、workflow 权限拆分);**P3×8 修复 7**(托盘气泡 UI 反馈defer 至 S7,记录为已知缺口)。审查原文结论:硬约束 1/2/3/4 未发现违反;`check_for_updates` 线程零锁、不触代数状态、无法阻塞托盘主线程;版本守卫对"tag 指向的树"防护完整。

## 方法备忘(可复用)

- **prerelease fixture 流程**(S5a 类验收的标准做法):开分支 bump 三处版本 → push → `gh release create vN.N.N --target <分支> --prerelease` → tag 推送自动触发 cascade 构建(守卫要求 tag==树内版本,故必须走 bump 分支,不能随手打 tag)→ softprops 对预建 release **保留 prerelease 标记、只追加资产**(本次实测)→ 测完按 **release → 远端 tag → 分支** 顺序删除。
- 构建期时间戳:CI 全量构建约 9 分钟(冷缓存);本地增量 release 构建 3 分钟。

## 环境还原

- fixture(v0.1.1 release + tag + 本地/远端分支)已全部删除,仓库 release 仅剩 v0.1.0。
- 安装版已卸载(`%LOCALAPPDATA%\dsh-desk` 目录清空删除);`%APPDATA%\dsh-desk\`(config/log,先于测试存在)未动。
- 无 dsh-desk / 本应用 node 子进程残留;浏览器窗口已关闭。
- 测试脚本与截图在 `%TEMP%\p2-smoke\`(tray/launch/census/hotkey/second/close/winvis/walkthrough/uninstall/patch + icon-exe-256.png + tray-flyout.png),供复核;安装包副本 `dsh-desk_0.1.0_x64-setup.exe` 同目录。

## 一句反思

验收 fixture 不能"随手打 tag"——版本守卫会正确地拒绝 tag 与树不符的构建;先在分支上 bump 再从分支建 release,守卫、构建、prerelease 标记三者才都自洽。守卫挡住自己时,说明流程设计对了。

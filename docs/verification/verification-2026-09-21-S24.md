# S24 应用内签名自动更新器（Q2 兑现 / S5b 落地）— 2026-09-21

- 验证人：agent（ZCode 会话）；用户拍板 Q2（"拍板 Q2"）
- 被验构建：本地 `pnpm tauri build` NSIS（树 = 0.3.2 终版含审查修复；验证用 A3/B2 为同代码 + 测试端点补丁）
- 机器：本机（WebView2 152）
- 结论：**全链路通过**（本地端点实测：检测→对话框→下载→验签→杀 server→NSIS passive 安装→重启为新版本）；过程抓到并解决一次"0 字节 exe"事故（根因是**被打断的构建**，非更新器缺陷，教训已入 AGENTS）

## 0. 密钥仪式（Q2）

- 密钥对：`pnpm tauri signer generate`，minisign 格式。**主拷贝在 `C:\Users\Administrator\.tauri\dsh-desk\`**（`dsh-desk.key` 私钥 / `dsh-desk.key.pub` 公钥 / `password.txt` 口令，均不入仓库）。**私钥或口令丢失 = 所有已发布客户端被锁死在内置公钥上、永远无法再自动更新**——请对这三件做离线备份。
- CI secrets（whyiyhw/dsh-desk）：`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（2026-09-21 设置；首对密钥因口令经 MSYS openssl 输出带 `\r` 无法解密，已换新并重写 secrets——**Windows 下生成口令避开 openssl base64 的 CRLF**）。
- 公钥嵌入 `tauri.conf.json` `plugins.updater.pubkey`；端点 `releases/latest/download/latest.json`（GitHub `latest` 不含 prerelease → **预发布永不自动装**，只经 Releases 页手动，这是设计）。

## 1. 实现要点（全部真机验证或源码级确认）

| 项 | 设计 | 验证 |
|---|---|---|
| 分层 | HTTP 检查（GitHub list API，见 prerelease）= 发现层；updater（latest.json，签名链）= 安装层；安装层不可用/失败回退开 Releases 页 | 本地端点全链 + 真实 GitHub"无更新"路径双验 |
| 确认对话框 | `tauri-plugin-dialog` 原生确认（不受本机 Focus Assist 抑制——toast 被抑制是 S14 已知环境事实，对话框是可见的）；blocking_show 在检查线程（非主线程，符合插件约束） | 对话框出现+截图留证（`confirm-dialog.png`）+ Enter 确认 |
| 签名校验 | minisign 公钥内嵌；`.sig` 由 CI/本地构建签名；manifest 携带签名；插件在 install 前验签 | 日志 "downloaded and **verified**"；构建期 `tauri signer sign` 自测 |
| server 生命周期 | `on_before_exit`：exiting 置位→lifecycle 锁内杀树（与 Quit 路径同纪律）+ 保存窗口几何；`install()` 的 `std::process::exit(0)` 不走 RunEvent，故必须在此杀 | 日志 "server stopped for the update install" + 代数语义行 + 重启后单进程零孤儿 |
| 实例参数 | NSIS `/P /UPDATE /R` + 当前 exe 参数 → `--instance` 跨更新存活（vendored updater.rs 的 current_exe_args 链确认） | 默认实例实测；命名实例同构（参数透传源码级确认） |
| 失败恢复（审查 P1 修复） | 安装器启动失败（on_before_exit 已跑后）：清 exiting + `restart_server` 复活 server，单 toast 单开页 | 代码路径审查确认（触发条件=ShellExecuteW 失败，罕见）；逻辑由审查子代理复核 |
| 超时（审查 P2 修复） | `updater_builder().timeout(60s)`：清单+下载共享客户端，挂死连接不再钉死检查线程/IN_FLIGHT 标志 | 代码级（reqwest 客户端超时） |
| fork PR（审查 P1 修复） | PR 构建（无 secrets）走 `--config '{"bundle":{"createUpdaterArtifacts":false}}'`，tag 构建才签名 | workflow 语法级（待 CI 首跑实证，见"未覆盖"） |

## 2. 全链路真机时间线（本地端点，0.3.2→0.3.3）

```
update check triggered（验证钩子，终版已撤）
checking for updates...
release v0.3.3 is newer than this build (0.3.2) — trying the signed in-app update first
downloading the signed update to 0.3.3...
update download at 10%...（10% 步进）
update downloaded and verified — installing (the app restarts)
stdout of pid N closed on a superseded generation (1); not the current server, ignoring
server stopped for the update install
dsh-desk v0.3.3 starting ...          ← NSIS /R 自动重启
chrome probe ok — borderless mode active   ← S21 无边框在更新后照常
started ... (pid M)
dsh web: http://127.0.0.1:8373…       ← server 复活并就绪
```

重启后清点：单 dsh-desk 进程 + 单 server 子进程（parent 对得上），零孤儿。**NSIS 重启连环境变量都继承**——顺带实测了新版对真实 GitHub 的"无更新"路径（`no newer release than 0.3.3; newest published is v0.3.1`）。

## 3. 验证过程事故：0 字节 exe（根因不在更新器）

- 现象：更新安装后 `dsh-desk.exe` 变 0 字节、进程消失；手动 `/S` 重装也"装"出 0 字节（带旧 mtime）。
- 排查：全新目录探针 → 安装器写入的本来就是 0 字节 + 源 mtime 19:27 → **安装包本身坏**（1.8MB，正常 5.1MB）→ 打包的源 `target/release/dsh-desk.exe` 是 0 字节。
- 根因：Build B 首次构建**被工具中断**，链接器把产物截断为 0 字节；重跑时 cargo 指纹判定"输出比依赖新"→ 跳过重链 → bundler 忠实打包 0 字节、signer 忠实签名、updater 忠实验签安装。**每一环都"正确"，坏的是输入**。
- 修复：`rm target/release/dsh-desk.exe deps/dsh_desk*.exe` 强制重链 → 5.14MB 健康包 → 全链路通过。
- 附带发现：坏安装器留下的注册表 `InstallLocation` 会劫持后续安装的默认路径（新装去不了 %LOCALAPPDATA%\dsh-desk）——显式 `/D=` + 清注册键解决；本机已清理复原。

## 4. 验证仪器

`D:\tmp\s24-server\`：本地 HTTP 端点（python -m http.server 8123：latest.json/releases.json/安装包）、full-update-flow / update-flow2（启动→对话框轮询→截图→Enter→观察重启）、tray-6002-direct / tray-diag（托盘驱动尝试，被浮层拒开击败后改用验证钩子——**托盘菜单自动化在用户在场时不可靠，验证钩子是正解**）、confirm-dialog.png（对话框留证）。A3/B2 测试构建补丁：端点指本地 + `dangerousInsecureTransportProtocol`（仅测试构建）+ `DSH_DESK_UPDATE_CHECK` 启动钩子（终版已撤，见 git diff 无此串）。

## 5. 未覆盖（如实记录）

- **CI 首跑的 latest.json 产物**：manifest 步骤（node 内联脚本）语法级验证过本地等价物（本次 latest.json 即同构手写），CI 实跑待 v0.3.2 tag 后以资产为准核验。
- fork PR 构建分支（--config 关 updater artifacts）：本仓库无进行中的 fork PR，未实跑；同仓库 PR 分支也未触发。语法 + 文档级论证。
- 安装器启动失败的恢复路径（审查 P1 修复项）：触发条件（ShellExecuteW 失败）无法安全人工构造，未实机触发；同构论证 = exiting 标志读写与 restart_server 均为既有已验路径的组合。
- 预发布不自动安装：GitHub `latest` 语义文档级确认（fixture prerelease 的 latest.json 不在 latest 通道）。

## 6. 环境还原

- 终态安装：v0.3.2 官方 CI 产物（发版后覆盖安装，替换测试 0.3.3）
- 注册表 InstallLocation 已修回 `C:\Users\Administrator\AppData\Local\dsh-desk`；探针目录/0 字节残留已清
- 本地 HTTP 端点服务已停；密钥三件套位置见 §0（**待用户离线备份**）

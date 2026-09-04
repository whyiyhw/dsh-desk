# dsh-desk 规范与计划(社区分发版)

> 状态:**v1.1**(2026-09-04,经双 subagent 评审修订)· 基线 commit `5e0c2af`(已推送 origin/main)
> 本文档只做规范与计划,**不含任何代码改动**。所有"现状"小节均标注代码出处,与代码一致。
> 注意:行号锚点是 `5e0c2af` 时点的快照指向,Phase 1 改动落地后自然过期,勿当活指针。

---

## 1. 现状架构(与代码一致)

### 1.1 组件关系图

```mermaid
graph TD
    subgraph DESK["dsh-desk.exe — Tauri 2 shell (src-tauri/src/lib.rs, 354 行)"]
        WIN["主窗口 main<br/>1440×920, 启动时隐藏<br/>初始页: src/index.html 转圈"]
        TRAY["托盘<br/>菜单: Show / Open in browser / Restart /<br/>Edit config / Check for updates / Quit<br/>左键单击 = 切换窗口显隐"]
        HK["全局热键 Alt+Shift+D<br/>(硬编码, lib.rs:332)"]
        SI["single-instance 插件<br/>二次启动 → show + 聚焦已有窗口"]
        SS["ServerState<br/>child: Mutex&lt;Option&lt;Child&gt;&gt;<br/>url: Mutex&lt;Option&lt;String&gt;&gt;"]
        SPAWN["spawn_server 线程<br/>起进程 + stdout/stderr 监视线程"]
        KILL["kill_server<br/>Windows: taskkill /PID x /T /F"]
    end

    subgraph DSH["dsh 子进程 (用户可配)"]
        CLI["dsh --profile web --no-open<br/>或 node 直启源码 checkout"]
        GUI["dsh web GUI (localhost, 带 token)"]
    end

    CFG[("%APPDATA%/dsh-desk/config.json<br/>command / args / cwd")]
    LOG[("%APPDATA%/dsh-desk/dsh-desk.log<br/>dsh stdout+stderr 全量镜像<br/>+ dsh-desk 自身诊断行")]
    BROWSER["系统默认浏览器<br/>(Open in browser 菜单)"]

    SPAWN -- "读取配置" --> CFG
    SPAWN -- "spawn: Windows 下直启报 NotFound/InvalidInput 才回退 cmd.exe /C" --> CLI
    SPAWN -- "每行日志落盘" --> LOG
    CLI -- "stdout: dsh web: 后跟认证 URL" --> SPAWN
    SPAWN -- "location.replace(url)" --> WIN
    CLI -- "HTTP" --> GUI
    WIN -- "加载认证后的 GUI" --> GUI
    TRAY -- "open_url" --> BROWSER
```

### 1.2 启动时序图

```mermaid
sequenceDiagram
    participant T as Tauri setup (lib.rs:287)
    participant S as spawn_server 线程
    participant D as dsh 子进程
    participant W as 主窗口

    T->>T: 建托盘 / 注册热键
    T->>S: 启动 spawn_server (lib.rs:336)
    S->>S: load_config (config.json)
    S->>D: spawn, stdout/stderr piped<br/>Windows 直启报 NotFound/InvalidInput 才回退 cmd.exe /C<br/>其他错误直接进诊断
    S->>S: 起两个监视线程 (stdout/stderr)
    loop 每行 stdout
        D-->>S: 文本行
        S->>S: log_line 落盘镜像 (含就绪行整行, 见 G10)
        alt 行以 dsh web: 前缀开头
            S->>S: 取前缀后首个空白分隔 token<br/>仅当其以 http 开头才接受, 否则整行忽略
            S->>S: ServerState.url = Some(url)
            S->>W: show + focus + location.replace(url)
            W->>D: webview 加载认证 GUI
        end
    end
    D-->>S: stdout EOF (代码假设 = 进程退出)
    S->>W: server_exited → show_message<br/>窗口内容替换为纯文本诊断
```

### 1.3 服务生命周期状态机(现状)

```mermaid
stateDiagram-v2
    state "Idle (无子进程)" as Idle
    state "Starting (已 spawn, 等就绪行)" as Starting
    state "Ready (url 已捕获, GUI 已加载)" as Ready
    state "Dead (诊断文本占窗口)" as Dead

    [*] --> Idle: 应用启动
    Idle --> Starting: spawn_server (lib.rs:123)
    Starting --> Ready: stdout 行 dsh web: url (lib.rs:186)
    Starting --> Dead: stdout EOF 且从未拿到 url
    Ready --> Dead: stdout EOF
    Dead --> Starting: 托盘 Restart (lib.rs:307, 任意状态可用)
    Starting --> Starting: Restart
    Ready --> Ready: Restart
    Starting --> Idle: Quit 或 ExitRequested, kill_server<br/>竞态点: child 可能尚未入册被漏杀
    Ready --> Idle: Quit 或 ExitRequested, kill_server
```

图注:

- stdout EOF 在代码中被**假设**等价于进程退出(lib.rs:196);进程活着但关了 stdout 会被误报为死。
- Dead 的两条进入路径对应不同诊断文案(拿到过 URL 与否,lib.rs:210-224)。
- Restart 菜单分支无状态检查,任意状态可用;Quit/ExitRequested 从任意状态走 `kill_server`。
- **现状缺口**:`Starting` 的*自动*出边只有"就绪行"与"进程死"两条,无超时出边——dsh 改了就绪行文案,状态机永远停在 `Starting`,窗口无限转圈(G1)。Restart/Quit 是用户发起的出边,不构成兜底。

### 1.4 已知缺口清单(逐条锚定代码)

| # | 缺口 | 代码出处 | 对应规范 |
|---|------|----------|----------|
| G1 | 就绪行识别失败无超时降级,静默转圈 | lib.rs:181-199 + src/index.html | S1 |
| G2 | 生命周期竞态,两层:(a) spawn→child 入册(lib.rs:174)之间有窗口,此间 Quit/Restart 的 `kill_server` 扑空;(b) **旧 stdout 监视线程在 Restart 后读到 EOF,把新进程的 url/child `take()` 走**(`server_exited`,lib.rs:196-198, 210-217)→ 下次 Quit 扑空 → 孤儿 dsh/node 进程活过应用退出 | lib.rs:145-174, 196-217, 307-312 | S2 |
| G3 | 配置/日志路径只认 `APPDATA`,非 Windows 连 config.json 都不写 | lib.rs:41-51 | S11(defer) |
| G4 | 图标为 Tauri 默认模板,托盘不可辨识 | src-tauri/icons/ | S6 |
| G5 | 无 CI / 无预编译安装包,安装门槛 = Rust 工具链 | 仓库无 .github/ | S3 |
| G6 | 无更新感知,发出去的版本永远停在安装时点 | — | S5 |
| G7 | 首跑/失败信息只是窗口里的纯文本,无操作按钮 | lib.rs:227-238 show_message | S4 |
| G8 | 热键 Alt+Shift+D 硬编码 | lib.rs:332 | S8 |
| G9 | 窗口位置/大小不记忆 | tauri.conf.json | S9 |
| G10 | 就绪行**整行**(含认证 token)明文镜像进日志文件;而 S1/S4 的所有失败路径都在引导用户打开/粘贴该日志 | lib.rs:184-186, 56-69 | S1 |
| G11 | 发布物料缺失:README 只有源码安装章节;三个版本字段(Cargo.toml / package.json / tauri.conf.json)无唯一真源;WebView2 安装模式未决策(默认 downloadBootstrapper,装机需联网) | README.md, tauri.conf.json, Cargo.toml, package.json | S12 |
| G12 | WebView2 运行时"已安装但 Chromium < 119"无任何检测:页面 JS 静默崩溃(`AbortSignal.any`/`Promise.withResolvers` 不存在),用户只看到"连接异常"徽章,零升级指引。Evergreen 自动更新失效的机器真实发生过(pv 冻结 114,2026-09-04 事故,见 postmortem-2026-09-04-webview2-114.md) | 缺失检测,应在 lib.rs setup 窗口创建后、spawn 前新增 | S4 |

---

## 2. 产品规范(Spec)

### 2.1 定位

- **一句话**:把 `dsh web` GUI 变成一个"点图标即用"的桌面常驻应用,免终端、免复制 token。
- **用户**:dsh(DeepSeek Harness)社区用户,主力 Windows;会装软件,不一定会编译 Rust。
- **JTBD**:"我每次用 dsh 的 web 界面都要开终端、等 URL、复制带 token 的地址贴浏览器——我要像用普通桌面软件一样用它。"
- **成功标准(社区分发版)**:
  1. 安装 → 首次可用,零文档依赖(装完即用,不要求编辑 JSON 才能跑通主路径);
  2. 任何失败在 90 秒内可见且可操作(有下一步动作,不是死屏);
  3. **升级感知存在**:应用能提示新版本并指到 Releases 页(原地自动升级为 Phase 3 可选项,不作为成功门槛)。

### 2.2 Non-goals(明确不做)

- **不逆向/不依赖 DSH 内部实现**——只依赖"stdout 打印认证 URL"这一稳定 web 表面(README 已声明的架构赌注,保持)。
- **不做设置 GUI / 多 profile 管理**——单配置 JSON 足够,有真实诉求再议。
- **暂缓 macOS/Linux**(G3)——除非社区出现真实呼声;届时路径层换 `app_config_dir()` 即可,不提前铺路。

### 2.3 改进项规范

#### S1 · 就绪感知与超时降级(P0,含日志脱敏)

- **问题**:G1(静默转圈)+ G10(token 明文落盘)。
- **方案**:
  1. **15-20 秒被动提示**:Starting 超过 15-20s 时,窗口在转圈旁显示"仍在启动…查看日志"的被动提示(不抢焦点、不进诊断态),慢启动不再是黑盒;
  2. **90 秒超时降级**:超时后窗口进入 Degraded 态:错误说明 + **打开日志**、**打开配置**、**重试** 三个动作按钮(替代现 `show_message` 纯文本,lib.rs:227-238)。**超时不杀进程**——dsh 可能只是起得慢,监视线程保持活跃,迟到就绪行仍可让窗口恢复(见 §3.1);
  3. **前缀匹配放宽**:从 `strip_prefix("dsh web: ")` 精确匹配(lib.rs:186)放宽为"按字面量 `dsh web:` 切分,取其后首个以 http 开头的空白分隔 token"——抗空格/措辞漂移,零误报成本;
  4. **日志脱敏**:镜像就绪行时只保留 scheme + host + port,token 部分掩码(lib.rs:184-186)。此为发布前置项,一旦带 token 的日志落到用户机器上就无法召回。
- **验收**:
  1. 改掉 dsh 输出前缀模拟漂移,≤90s 内出现含三按钮的可操作诊断;
  2. 20s 时窗口出现被动提示;
  3. 日志文件中就绪行不含 token(仅 scheme+host+port);
  4. 正常路径与 §2.4 契约一致。
- **涉及**:lib.rs(spawn_server/定时器/log_line)、src/index.html(提示态与诊断态骨架)、src-tauri/capabilities/default.json(按钮经 `#[tauri::command]` + invoke 调 Rust;opener 需文件路径 scope——Tauri 2 ACL 拒绝是常见的半天级卡点)。

#### S2 · 生命周期竞态收敛(P0,代数标记)

- **问题**:G2 两层竞态。真正的危险行为不是锁窗口本身,而是**旧监视线程的反噬**:Restart 后旧 stdout 线程读到 EOF → `server_exited` 把新进程的 url/child `take()` 走 → 窗口被错误覆盖、"Open in browser" 失效、下次 Quit 扑空留下孤儿进程。
- **方案**:
  1. **代数(generation)标记**:`ServerState` 增加单调递增计数(如 `AtomicU64`);每次 spawn 递增,本代 stdout 监视线程与 S1 定时器持有各自的代数;`server_exited` 与超时动作**仅在代数仍与当前一致时生效**,旧代线程读到 EOF 只安静退出;
  2. **spawn→入册原子化**:经同一把**短持有**的串行锁完成(不得跨 `spawn()` 持 child 字段锁——cmd.exe 回退可能很慢,而 `kill_server` 在托盘/主线程被同步调用,lib.rs:309,长持锁会卡 UI);
  3. **定时器同规则**:每次 spawn 启动新的 90s 定时器并作废旧代定时器(旧代超时不得对新代报障);
  4. **重试 = Restart**:Degraded 态的重试按钮走与托盘 Restart 完全相同的串行流程(杀净 → 等退 → 再起),禁止"旧进程还活着就直接 spawn"。
- **验收(可判定)**:记录 dsh/node 进程数基线 → 连点 Restart×10 → 末次后等 30s:进程数 == 基线,且 120s 内窗口回 Ready 或正确的诊断态 → **然后 Quit:无任何 dsh/node 残留进程**(末项是抓孤儿 bug 的关键,原"进程数不增长"测不出来)。
- **涉及**:lib.rs(ServerState/spawn_server/server_exited/restart 分支)。

#### S3 · CI 与预编译安装包(P1,社区分发主线)

- **问题**:G5。
- **方案**:GitHub Actions,`windows-latest`:push → bare `cargo check`(便宜,提前到 Phase 1 落地也可);tag/PR → 完整 `pnpm tauri build`,产出 NSIS/MSI 挂 GitHub Releases(控制 Windows 冷构建成本,不做每 push 全量构建)。updater 产物仅在 S5b 启用时追加。
- **验收**:打 tag 后 Release 页出现可安装的 `.exe`/`.msi`,新机器(无 Rust/Node、已装 dsh)装完可用默认配置跑通。
- **涉及**:新增 .github/workflows/(纯新增,不动现有代码逻辑)。

#### S4 · 首跑引导与失败可操作化(P1)

- **问题**:G7 + 首跑体验。
- **方案**:首跑(无 config.json)时探测 `where dsh`;找到 → 写默认配置照常起;找不到 → 窗口进入"安装引导态":说明 + **打开配置文件**按钮(编辑 command/cwd 指向源码 checkout 或自装路径)+ **重试**。引导态与 S1 的 Degraded 态复用同一套"窗口内动作按钮"机制(同一条 invoke/ACL 通路)。托盘菜单加 **Edit config** 项。
- **WebView2 版本检测(G12,2026-09-05 增补)**:启动时在 spawn 前读运行时版本(注册表 `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` 的 `pv`),已装但解析失败或 **< Chromium 119** → 进入"运行时过旧"引导态:显示当前版本/基线 + **打开运行时下载页**按钮(官方 bootstrapper fwlink 2124703)+ **重试**。完全未装运行时时窗口本身无法创建,不归本态管,由 S12 的安装期 bootstrapper 兜底。基线 119 与 `scripts/env-check.ps1` 保持一致。
- **验收**:干净机器(无 dsh)首跑 ≤10s 进入引导态且按钮可用;托盘 Edit config 能打开 config.json;注册表 pv 临时改低(测后恢复)时启动 ≤10s 进入"运行时过旧"引导态,下载按钮打开官方下载页,恢复 pv 后重试正常到达 Ready。
- **涉及**:lib.rs(load_config/spawn_server/托盘菜单/启动版本检测)、src/index.html、src-tauri/capabilities/default.json。

#### S5 · 更新能力(P1,两增量拆分)

- **问题**:G6。
- **方案**:
  - **S5a(Phase 2,默认路径)**:托盘 **Check for updates** → `GET /repos/{owner}/{repo}/releases?per_page=1` 取最新一条(2026-09-05 修订:原稿 `/releases/latest` 不含 prerelease,与下述"用 prerelease 测验收"自相矛盾;列表端点含 prerelease,draft 对未认证读取不可见)→ 与内置版本按 semver 比对(含 `-rc1` 预发布后缀;同号下纯发布 > 预发布)→ 有新版则打开 Releases 页(opener)。约 60 行,无签名密钥、无密钥管理负担,满足成功标准 #3;
  - **S5b(Phase 3,可选,门禁 Q2)**:tauri-plugin-updater 原地升级,签名公钥内置、私钥妥善保管。**明确失败模式:私钥丢失 = 已发布客户端(内置公钥)永远无法原地升级**,用户被晾在旧版——单人维护者若不愿承担密钥管理,永久停在 S5a 是完全正当的选择。
- **验收**:S5a——在真实仓库发一个更高版本号的 **prerelease**(draft 对 API 不可见,不能用 draft 测),旧安装能检出并打开 Releases 页。S5b——假版本升级走通且签名校验生效。
- **涉及**:lib.rs(托盘项 + 网络请求)、tauri.conf.json(S5b)、CI(S5b 产物)。

#### S6 · 品牌图标(P1)

- **问题**:G4。
- **方案**:出一套 dsh-desk 专属图标(ico/icns/png 全套),托盘/任务栏/安装包统一。
- **验收**:托盘与安装后开始菜单出现新图标;`pnpm tauri build` 无图标报错。
- **涉及**:src-tauri/icons/、tauri.conf.json。

#### S12 · 发布物料(P1,Phase 2 与 S3 同批)

- **问题**:G11——没有这些,"分发闭环"只对会编译的人闭环。
- **方案**:
  1. **README 增加 Install from release 节**:最低 Windows 版本(建议 Win10+);SmartScreen/杀软误报说明("仅从官方 Releases 下载");前置条件 = 已装 dsh;**Support 节** = GitHub Issues + 附 `%APPDATA%/dsh-desk/dsh-desk.log`(S1 脱敏落地后此文件才可安全外发);
  2. **版本真源**:tauri.conf.json 的 version 为唯一真源,tag 时 bump;Cargo.toml / package.json 的版本字段随真源同步(或 README 声明它们不追踪);
    3. **WebView2 决策(2026-09-05 勘误)**:选 `embedBootstrapper`(安装包 +约 1.8MB,README 注明)——原稿称其"离线可用"有误,官方语义为装机装运行时**仍需联网**,仅内嵌引导器本身;真离线只有 `offlineInstaller`(+约 127MB)可做到,未选(体积代价大,离线机器罕见)。离线/无运行时机器由 README 指引先手工装独立运行时包(fwlink 2124701),"已装但过旧"由 S4/G12 应用内门禁兜底;若社区出现真实离线装机需求,一行配置可翻为 offlineInstaller;
  4. **FAQ 预期差条目(2026-09-05 增补,用户实测提出)**:「浏览器打开 `http://127.0.0.1:<port>/` 显示 *dsh web authentication required*」不是故障——dsh 服务端要求**首次访问必须带 token**(`/?token=…` 换 30 天 cookie),裸 `/` 无凭证必 401。正确入口:桌面壳窗口(内部自动完成 token 交换,免手工);外部浏览器走**托盘 Open in browser**(带完整 URL 打开默认浏览器)。S1 脱敏后日志/终端不再出现带 token 的 URL,属故意设计(token 入日志=泄露),README 需向社区说明"从日志抄 URL"这条路已关闭。
- **验收**:一名非技术 Windows 用户**仅按 README** 从 Releases 完成安装并到达 Ready。
- **涉及**:README.md、tauri.conf.json(构建脚本的版本同步策略)。

#### P2 备选池(择优,不承诺)

| # | 项 | 一句话方案 | 触发条件 |
|---|----|-----------|---------|
| S7 | 托盘状态可视化 | server 意外退出弹系统通知;托盘图标反映 running/stopped | Phase 3 有余量 |
| S8 | 热键可配置 | config.json 加 `hotkey` 字段,默认 Alt+Shift+D | 有用户报冲突 |
| S9 | 窗口状态记忆 | tauri-plugin-window-state | 几乎零成本,顺手 |
| S10 | 开机自启 | tauri-plugin-autostart,托盘开关 | 用户呼声 |
| S11 | 跨平台 | 路径层换 `app_config_dir()`,kill 走进程组 | 社区有 mac/linux 呼声 |
| S13 | 崩溃可诊断 | `std::panic::set_hook` → log_line;日志启动时写版本横幅与 dsh 命令行;启动时截断/轮转日志(现为无限增长) | Phase 3 |

### 2.4 正常路径回归契约(替代含糊的"零回归")

以下为每次交付前必须人工走查的**可观察契约**(也是 Phase 0 冒烟的记录模板):

1. 就绪行打印后,窗口显示并导航到认证 GUI(无 token 手工介入);
2. 托盘菜单全部可用:Show(切换显隐)/ Open in browser(打开认证页)/ Restart / Edit config(S4)/ Check for updates(S5a)/ Quit;
3. 热键 Alt+Shift+D 从任意应用切换窗口;
4. 二次启动唤起并聚焦已有窗口;
5. 关窗 → 隐藏到托盘,server 继续运行;
6. Quit 后无 dsh/node 残留进程(本条在 S2 落地后对 Restart 后的场景同样成立)。

---

## 3. 分阶段计划(Phase Plan)

```mermaid
graph LR
    P0["Phase 0 立整基线"] --> P1["Phase 1 可靠性 S1+S2"]
    P1 --> P2["Phase 2 分发闭环 S3+S4+S6+S12+S5a"]
    P2 --> P3["Phase 3 升级与打磨 S5b 可选 + P2 择优"]
```

| Phase | 目标 | 交付 | 验收门 | 依赖 |
|-------|------|------|--------|------|
| **0 · 立整基线**(半天) | 冻结基线,定义回归契约 | 处置未跟踪的 `ws-probe.cjs`(删或提交);提交 `docs/`(本文档);本机 `pnpm tauri build` + 真机启动冒烟一次,按 §2.4 记录基线;顺手落 push 触发的 bare `cargo check` CI | `git status` 干净;冒烟记录在案;CI 绿 | — |
| **1 · 可靠性**(1-2 天) | 失败全部可见、可操作、可脱敏 | S1(被动提示 + 超时降级 + 前缀放宽 + 日志脱敏)+ S2(代数标记 + 串行化重试);~~行动项:上游 issue 确认就绪行稳定性~~(原 Q3 已关闭——上游 issue 区禁用且明确不接受外部 PR,见 §5 2026-09-05 行) | S1/S2 验收标准逐条通过;§2.4 契约走查通过 | Phase 0 |
| **2 · 分发闭环**(2-3 天) | 社区用户装得上、首跑有人接、坏得起有日志 | S3 CI+安装包、S4 首跑引导、S6 图标、S12 发布物料、S5a 轻量更新检查 | 各项验收标准;干净 Windows 虚机**仅按 README** 装包到 Ready(含离线 WebView2 场景) | Phase 1(失败态/脱敏是引导态与外发日志的前提) |
| **3 · 升级与打磨**(按需) | 版本可演进(或明确不演进) | S5b 原地自动更新(门禁 Q2,可选)+ P2 择优(S9/S13 最可能) | S5b 验收(若启用);§2.4 契约 | Phase 2;Q2 决策 |

### 3.1 Phase 1 后的目标状态机(计划态,与 §1.3 对照)

```mermaid
stateDiagram-v2
    state "Idle" as Idle
    state "Starting (90s 定时器, 每次 spawn 重置并作废旧定时器)" as Starting
    state "Ready" as Ready
    state "Degraded (诊断 + 按钮: 日志/配置/重试)" as Degraded

    [*] --> Idle
    Idle --> Starting: spawn_server (递增代数)
    Starting --> Ready: 就绪行 (无论 15-20s 提示期是否已过)
    Starting --> Degraded: 90s 超时, 进程不杀, watcher 仍活跃
    Starting --> Degraded: 进程死 (仅当本代 watcher, S2)
    Ready --> Degraded: 进程死 (仅当本代 watcher, S2)
    Degraded --> Ready: 迟到就绪行 (进程未杀, watcher 仍活跃)
    Degraded --> Starting: 重试 或 Restart, 同一串行流程: 杀净 等退 再起
    Ready --> Idle: Quit
```

图注:三条 S2 语义在此固化——(1) 旧代 watcher 的 EOF 不触发任何转换;(2) 重试与 Restart 是同一条串行路径,不允许"未杀先起";(3) 超时与 EOF 转换都只认本代。

---

## 4. Canonical Context Blueprint (English, agent-handoff ready)

- **Objective**: Make dsh-desk distributable to the dsh community as a reliable, installer-based desktop shell for the dsh web GUI.
- **Product Intent**: One-click authenticated access to the dsh web GUI as a tray-resident desktop app; failures are always visible and actionable within 90s; logs are safe to share (token-redacted).
- **Scope**: S1 readiness hint + timeout degradation + prefix-tolerance + token redaction; S2 generation-tagged lifecycle (watchers/timers act only on their own generation; serialized kill→wait→spawn for Restart AND Retry); S3 CI (cargo check on push, full build on tag/PR) + prebuilt installers; S4 first-run guidance + actionable error states + tray "Edit config"; S6 branded icons; S12 release pack (README install/support, versioning single-source, WebView2 embedBootstrapper); S5a lightweight update check (Phase 2), S5b in-place updater optional (Phase 3, gated on key-management decision).
- **Non-goals**: No DSH-internal API integration (the `dsh web:` stdout line is the only contract); no settings GUI / multi-profile management; macOS/Linux deferred; S5b not required for success.
- **Constraints**: Tauri 2 + Rust only for shell logic; config stays a plain JSON file (`%APPDATA%/dsh-desk/config.json`); process-tree kill must keep Windows `taskkill /T /F` semantics; no breaking change to the §2.4 observable contract; do not hold the child lock across `spawn()` (UI jank via synchronous tray-thread kill_server).
- **Existing Repo Context** (inspected at `5e0c2af`, pushed): two commits (`101b492` initial shell; `5e0c2af` "Fix double-appended args; log to file; direct-exe spawn with cmd fallback"); tracked tree clean; untracked: `docs/` (this spec) and `ws-probe.cjs` (scratch probe, to resolve in Phase 0). `src-tauri/src/lib.rs` is 354 lines (server lifecycle, tray, hotkey, single instance). Also inspected: main.rs, tauri.conf.json (window 1440×920 hidden start, CSP null, targets all), capabilities/default.json (core/opener/global-shortcut defaults), src/index.html (spinner), package.json, README. Line anchors are snapshot pointers at this commit.
- **Required Deliverables**: Phase 0 — resolve ws-probe.cjs, commit docs/, build+launch smoke per §2.4, push-CI cargo check; Phase 1 — S1+S2 code changes + upstream stability issue filed; Phase 2 — .github/workflows, first tagged release, onboarding UI states, icon set, README release/support sections, versioning policy, WebView2 decision, S5a; Phase 3 — optional S5b, P2 picks.
- **Acceptance Criteria**: per-item in §2.3; global: §2.4 contract intact; every failure mode visible and actionable ≤90s; logs contain no auth tokens; rapid-Restart×10 then Quit leaves zero dsh/node processes; clean Windows VM reaches Ready following only README.
- **Risks**: dsh may change the readiness-line wording (mitigated: visible degradation + tolerant split match — both landed and verified in S1; upstream confirmation unobtainable: issue tracker disabled, external PRs declined, feedback routed to Discussions only, see decision log 2026-09-05); auth token in mirrored logs (mitigated by S1 redaction BEFORE any release artifact ships); unsigned installers → SmartScreen/AV friction (Q1); S5b signing-key loss strands old clients on their built-in pubkey (explicitly accepted if S5b skipped); WebView2 offline install; CI Windows cold-build time; docs line anchors rot after Phase 1 edits.
- **Open Questions**: Q1 code signing (pay for cert vs. accept SmartScreen warning for a community tool); Q2 who holds the S5b updater private key, with the stranding failure mode stated; Q3 upstream readiness-line stability (CLOSED 2026-09-05: upstream disables its issue tracker, explicitly declines external PRs, and self-describes as developer preview — no stability commitment is obtainable; risk accepted, mitigation = S1 tolerant match + visible degradation, S12 README to document the bet); Q4 distribution beyond GitHub Releases (winget?).
- **Verification Plan**: manual failure-injection (rename readiness prefix) for S1; rapid-restart-then-Quit process census for S2; clean-VM README-only install smoke for S3/S4/S12; prerelease-based update-check test for S5a (drafts are API-invisible); §2.4 walkthrough as the universal regression gate.

---

## 5. 决策记录(Assumption / Decision Log)

| 时间 | 决策 | 依据 |
|------|------|------|
| 2026-09-04 | 受众 = dsh 社区(非纯自用),分发升为主线 | 用户确认 |
| 2026-09-04 | 本轮只出规范与计划,不改代码 | 用户确认 |
| 2026-09-04 | 跨平台 defer,路径层重构不在本计划内 | 无 mac/linux 呼声,避免提前铺路 |
| 2026-09-04 | 保持"只依赖 stdout 认证 URL"契约,不逆向 DSH 内部 | README 声明的架构赌注;社区分发后升级频繁,解耦是生存条件 |
| 2026-09-04 v1.1 | S2 重定义为**代数标记**方案:根因是旧 stdout 监视线程反噬新状态(而非仅 spawn/kill 锁窗口),验收补"Restart 后 Quit 无残留进程" | subagent 评审 B1 |
| 2026-09-04 v1.1 | §3.1 补 Degraded→Ready(迟到就绪行)、重试=串行杀净再起、定时器随 spawn 重置 | subagent 评审 B2 |
| 2026-09-04 v1.1 | S1 增补:15-20s 被动提示、前缀匹配放宽、**就绪行日志脱敏**(发布前置) | subagent 评审 M2/m2 |
| 2026-09-04 v1.1 | 新增 S12 发布物料;CI 改为 push=check、tag/PR=全量;"零回归"替换为 §2.4 可观察契约 | subagent 评审 M3/m6/n4 |
| 2026-09-04 v1.1 | S5 拆两增量:默认走 S5a 轻量检查,S5b 原地升级可选(密钥丢失=老用户搁浅) | subagent 评审 M4 |
| 2026-09-04 v1.1 | 基线重锚 `5e0c2af`;Phase 0 改为处置 ws-probe.cjs + 提交 docs/ + 冒烟;Q3 转为 Phase 1 行动项 | subagent 评审 M1/n2/n3 |
| 2026-09-05 | **S1 已落地**(代码 + 真机自验 + §2.4 六条走查全过),自验记录见 [verification-2026-09-05-S1.md](verification-2026-09-05-S1.md);范围/验收未变更,S2 未动,本文行号锚点自此过期 | 交付事实记录 |
| 2026-09-05 | **S6 已落地**(cursor-agent 三轮设计定稿"字体轮廓"方案;ico/icns/png 全套重生成;`pnpm tauri build` 无图标报错 + exe 内嵌图标抽取目验通过),记录见 [verification-2026-09-05-S6.md](verification-2026-09-05-S6.md);真机托盘实时目视与开始菜单目视(依赖 S3 安装包)留待后续真机验证 | 交付事实记录 |
| 2026-09-05 | 吸收 IoT 后端经验(总纲"每个异常要么被拦住要么变成规矩"):新增回合末独立审查(skill 工作协议)、`.githooks/pre-commit` fmt 快检(fail-open,cargo check 归 S3 CI)、`scripts/env-check.ps1` 环境自检;受影响测试映射/天周级全量梯度**不采纳**(单 lib.rs 规模,规则成本>事故损失)。四级梯度落位见仓库 AGENTS.md"异常的下落"节 | 用户提供经验文章 |
| 2026-09-05 | S12 物料增补 FAQ 条目:「浏览器裸 URL → 401」预期差(壳内自动认证 vs 外部浏览器须带 token 首访;S1 后日志无 token 属故意) | 用户实测提出,社区大概率复问 |
| 2026-09-05 | **S4 范围增补(G12)**:启动时检测 WebView2 运行时版本,已装但 < Chromium 119 → "运行时过旧"引导态(当前/基线版本 + 官方下载页按钮 + 重试)。依据:2026-09-04 真实事故——本机 pv 冻结 114,页面 JS 静默崩溃仅显示"连接异常",用户零指引;S12 bootstrapper 只兜"完全未装",S1 降级只兜"服务器失败",此处为规范空白。详见 [postmortem-2026-09-04-webview2-114.md](postmortem-2026-09-04-webview2-114.md) | 事故复盘 + 用户确认补齐 |
| 2026-09-05 | **S4 已落地**(首跑引导态 / 托盘 Edit config / WebView2 版本门禁;`cargo test` 9 passed;验收三条 + §2.4 六条真机全过,记录见 [verification-2026-09-05-S4.md](verification-2026-09-05-S4.md))。验收口径与 §2.3 正文的落地偏差(独立审查后修正,均记验证记录):①两道门禁(运行时/安装引导)放 **spawn_server 内部对每次 spawn 生效**(boot/Retry/托盘 Restart 全路径),非 §2.3 字面的"首跑(无 config.json)时探测"——非首跑且 dsh 仍缺时引导态比裸 spawn 错误更准确,且防托盘 Restart 绕过运行时门禁;②过旧态下载链接用 **fwlink 2124701**(x64 独立包)而非本表原文的 2124703——bootstrapper 对已装实例报 already installed(0x80040828),恰是本态目标机器(postmortem 实锤);③pv 压低验收经 **HKCU** 路径执行(HKLM 写入需 UAC,非提权 shell 被拒;同一代码路径,顺带验证 per-user 回退——注册表探测按 loader 优先级 per-user 先、首个应答者定夺) | 交付事实记录 + 独立审查 |
| 2026-09-05 | S4 引导文案决策:**给 dsh 项目页链接**(github.com/deepseek-ai/deepseek-harness,README 已引用的稳定表面),不写死安装命令——安装方式会漂移,与"只依赖稳定表面"的架构赌注一致 | 用户留给 S4 定的开放点,实现时拍板 |
| 2026-09-05 | **S2 已落地**(代数标记 + 串行化生命周期:spawn/主动 kill/已上报退出各 mint 一代,watcher/timer 只认本代,Restart 与 Retry 同走杀净→等退→再起;`cargo test` 10 passed 含 S2 四条)。回合末独立审查抓到 1×P1(claim_exit 的 child 交接不原子——旧 watcher 微秒窗口可 take 走 Child 无 taskkill 直接 drop,kill 扑空,"EOF≈退出"假设下进程树活过 Quit)已修:child take 移进认领临界区、kill 的 bump 同入 child 锁、EOF 后 try_wait 仍活补杀、已死只 reap 防 pid 复用误杀;4×P2 同批处置。真机验收(判定式逐条):Restart 风暴 10 次效果确认(重试合计 16 周期)→ 清点恒 1 子进程、17 次旧代 EOF 全部 `superseded generation` 静默、`exited` 误报 0(S1 已知边界 1 根治)、终态 Ready;**托盘 Quit 后 dsh/node 零残留**;§2.4 六条全过(并与并行 S4 会话在同合并构建上的独立走查互为交叉证据)。记录见 [verification-2026-09-05-S2.md](verification-2026-09-05-S2.md)。注:S2 验收期间与 S4 会话共享真机/工作树交错(其验证记录有述),本轮所有证据采自 S1+S4+S2 合并构建 | 交付事实记录 + 独立审查 |
| 2026-09-05 | **Phase 0 完成**:`ws-probe.cjs` 脱敏参数化后提交(原硬编码 token 未进 git 历史);此前各会话交付但未提交的 S1/S2/S4/S6 全部产物分批入库,git status 干净;**release 构建首次冒烟** §2.4 六条契约全过(记录见 [verification-2026-09-05-phase0.md](verification-2026-09-05-phase0.md),含托盘驱动配方 v3:浮层开着时 6002 直投为首选开菜单方式);push CI(bare cargo check,windows-latest)落地 | 交付事实记录 |
| 2026-09-05 | **Q3 关闭(不可执行 → 风险接受)**:实测上游 deepseek-harness 仓库 `has_issues=false`(issue 区整体禁用,页示 "Issue creation is restricted")、CONTRIBUTING.md 明言 "cannot accept external pull requests at the moment"、反馈唯一入口 = GitHub Discussions(自述小团队未必逐条回复),且 README 自述 developer preview、预期破坏性变更——"提 issue 拿就绪行稳定承诺"既不可执行、也大概率无答案。缓解即 S1 已落地并验证的宽容匹配 + 90s 可见降级;S12 落地时在 README 记录该架构赌注与措辞漂移的故障形态(→降级态可操作)。可选后续:Discussions 发帖询问(外发动作,需用户拍板,不阻塞 Phase 2) | 用户提出 + gh API/CONTRIBUTING 实测核实 |
| 2026-09-05 | **S3 已落地**:release.yml(tag/PR 全量构建;build=contents:read 产 artifact,release=仅 tag、contents:write 挂 softprops;concurrency+timeout)+ check.yml tags-ignore + check-versions.mjs 守卫前置。验收 1 过——tag v0.1.0 → [Release 页](https://github.com/whyiyhw/dsh-desk/releases/tag/v0.1.0)挂 NSIS(4.35MB)/MSI(5.59MB);验收 2 本机部分过(静默安装→10.8s READY+WS ESTABLISHED,§2.4 六条在安装版全过,S6 开始菜单/托盘图标目视补齐),**干净虚机门禁保持开放**。记录见 [verification-2026-09-05-S3.md](verification-2026-09-05-S3.md) | 交付事实记录 |
| 2026-09-05 | **S12 已落地**:README 四节(Install from release/Support/FAQ 401+漂移/versioning)+ embedBootstrapper + 版本真源守卫。**勘误**:embedBootstrapper 官方语义为装机仍需联网(仅内嵌引导器,+~1.8MB),原稿"离线可用"有误,§2.3 已改;真离线=offlineInstaller(+~127MB)未选,离线机器走 README 直链 fwlink 2124701。虚机 README-only 门禁开放。记录见 [verification-2026-09-05-S12.md](verification-2026-09-05-S12.md) | 交付事实记录 + 审查勘误 |
| 2026-09-05 | **S5a 已落地**:托盘 Check for updates → `GET /releases?per_page=1`(偏离原稿 `/releases/latest`,因后者不含 prerelease 与验收条款矛盾,§2.3 已修订)→ semver 全序比对(含 `-rc` 后缀)→ 新版打开 Releases 页;独立线程零锁+在途去抖;失败仅日志。真机验收负向+正向全过(v0.1.1 prerelease fixture 检出并打开其 release 页,Chrome 标题实锤;fixture 测后删)。已知缺口:无更新时无用户可见 UI(S7 范畴)。`cargo test` 12 passed。记录见 [verification-2026-09-05-S5a.md](verification-2026-09-05-S5a.md) | 交付事实记录 |
| 待定 | Q1 代码签名 / Q2 更新密钥 / Q4 winget | 需用户决策,见 §4 |

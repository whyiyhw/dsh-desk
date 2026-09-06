# 验证记录:S14-S20 UX 债务批次(2026-09-06)

> 交付:更新检查可见反馈(S14)/面板与日志文案分离(S15)/可访性三件套(S16)/面板视觉身份与暗色(S17)/引导面板 Use detected dsh(S18,按用户指定实现原始版而非 spec 收窄版)/首次关窗进托盘一次性通知(S19)/界面语言决策显式化(S20,文档)。
> 构建:v0.2.1 版号未 bump 的 main 工作树,`pnpm tauri build` 后 NSIS 静默安装(`%LOCALAPPDATA%\dsh-desk\dsh-desk.exe`),全部真机验证在安装版上执行。
> `cargo test` **17 passed**(15 存量 + S15 分离锚定 + REG_DWORD 解析锚定),`cargo fmt --check` 过。

## 逐项验收

### S14 更新检查可见反馈(验收:三种结果各有可见反馈、≤5s、失败不静默)

| 结果 | 判定 | 证据 |
|---|---|---|
| 无新版(v0.2.1==最新 release) | **过** | 托盘菜单 6002 直投点 Check for updates(row 5/6)→ Action Center 入库时间戳更新,**点击后 2.7s**;日志 `no newer release than 0.2.1; newest published is v0.2.1` |
| 网络失败 | **未实机触发** | netsh 防火墙规则需提权,沙箱会话不可用("requires elevation"),规则从未生效。分支代码与已验路径同构(同一 `show_toast`,仅入参不同);HTTP 预算 10s→4s 保证失败路径落在 5s 内。重复的 no-newer 实测 2.4s |
| 有新版 | **沿用 S5a 前证** | semver 比较与 open_external 未改(S5a 当日 prerelease fixture 真机验证过);toast 是在 open_external 前新增的对称调用 |

**toast 显示层说明(全批适用)**:本机 Focus Assist 被 NVIDIA Overlay 全屏自动规则激活(`QuietHoursServiceState=2`),**所有应用的 toast 显示被系统抑制**(WinRT 原生控制 toast 也不显示)。判定降级为**送达级**:①Action Center 入库时间戳(HKCU `...\Notifications\Settings\com.whyiyhw.dshdesk!LastNotificationAddedTime`)在触发后更新=到达 OS;②无 `toast failed to show` 日志=API 成功;③同一 `show_toast` 通道(S7 意外退出 toast)当日早些时候在本机安装版上目验过显示。曾尝试恢复显示(杀 overlay、注册表置 0、竞速触发),WpnService 内存态不受注册表控制且无权重启服务,放弃。

### S15 面板/日志文案分离(验收:面板零前缀、零术语;日志保持前缀)

- **面板侧(真机 OCR 独立提取)**:EOF 早退面板截图 → Umi-OCR 逐字提取,全文为新文案(`The dsh server (pid N) exited before printing its web address. / The log shows... / Press Retry once the launch command works.`),**零 `dsh-desk:` 前缀、零 "readiness line"**。onboarding 面板 OCR 同验(`'no-such-dsh' was not found on this machine`——首行同时改进为点名配置的命令)。
- **日志侧**:三条 degraded 的 log 副本保持 `dsh-desk:` 前缀逐字不变(可 grep)。
- **测试固化**:`degraded_copies_split_log_prefix_from_panel_prose` 锚定四组消息(含 had_url 两变体)的"日志有前缀/面板无前缀无术语"。

### S16 可访性三件套(验收:对比度、边界 3:1、reduced-motion;焦点环保留)

- `.path` #5c5c5c×#fdf6e3=**6.20:1**(原 #777 为 4.15:1);按钮边框 #8a8a8a×#fff=**3.45:1**(原 #b9b9b9 约 2:1);spinner 动画包进 `@media (prefers-reduced-motion: no-preference)`,降级为静态环+文字。
- 全文件无 `outline:none`;`:focus-visible` 焦点环为增强(#4D6BFE 2px)。**键盘可用性真机实证**:phase2b 中面板按钮以 Tab+Enter 键盘序列成功触发(焦点落于 DOM 首个可见按钮=Use detected dsh)。
- 数值经独立审查子代理用 WCAG 相对亮度公式复算全部属实(含白字主按钮底色特意选 #3E5BF5=5.22:1 而非 #4D6BFE=4.33:1)。

### S17 面板视觉身份(验收:品牌标记+主色;暗色无白闪;旧运行时可渲染)

- **品牌标记+主色(像素级)**:EOF 面板截图含品牌蓝像素 8643 个(#3E5BF5×3720 + #4D6BFE×4923,主按钮+spinner+焦点色);OCR 从 40px 内嵌 SVG 标记上读出 "dsh" 字样(标记可读性意外的最强证据);亮色卡片近白像素 338953。
- **暗色无白闪——重要产品发现与修复**:实测本机(暗色 app mode,`AppsUseLightValue=0`)WebView2 把 `prefers-color-scheme` **钉死为 light**(注册表双值置暗+广播仍亮,连窗口标题栏都亮)。修复:Rust 读注册表真值(`reg_dword_is_zero`, OnceLock 缓存)经 eval 注入 `data-theme`,CSS 双通道(媒体查询保留+`[data-theme="dark"]` 兜底);注入只存在于"helper 存在"分支(不碰上游 GUI DOM,独立审查复核确认)。**暗色真机像素实证**:页面暗底 322430px、卡片暗底 293624px、残留白 35822px(titlebar);亮色分支置 1 复测通过(338909px 近白)后还原。
- **逃逸路径修补(审查 P2)**:starting 页可经托盘 Show/热键/二次启动在无 panel eval 时显示——三处 `window.show()` 后补 `stamp_page_theme`。
- **旧运行时自举**:所用最高特性 `:focus-visible`(86)/flex gap(84)/color-scheme meta(81)/prefers-*(76)/CSS 变量(49),全部 <114;JS 无新增现代 API。**pv=114 模拟渲染未测**(需 HKCU 压低 pv+降级运行时环境,本轮时间盒内未执行;CSS 特性清单+逐项版本判定经独立审查复核)。

### S18 Use detected dsh(按用户指定实现;验收:一键写默认 config+自动 Retry)

- **按钮显隐(OCR 双侧)**:无 dsh 于 PATH(本机天然如此)→ 按钮隐藏、无提示行;注入假 `D:\tmp\ux-fake\dsh.cmd`(PATH 前置)→ 按钮出现于首位 + 消息尾行 "`dsh` was found on PATH — "Use detected dsh" switches the config to it."
- **一键闭环(真机)**:键盘 Tab+Enter 点按钮 → config.json 被重写为默认 dsh 启动(文件内容实读)→ 日志 `replacing config \`no-such-dsh --profile web\` with the default dsh launch` → `started \`dsh --profile web --no-open\`` → 假 dsh 就绪行捕获且脱敏(`http://127.0.0.1:9…`)。
- **静默失败修补(审查 P2)**:guard 拒绝/写文件失败两路径经 `deskAppendNote` 向面板追加可见说明(与 invoke 失败兜底同通道)。
- **与 spec §2.3 原 S18 行的关系**:spec 原行是收窄后的"checkout 填表"方案;本次按用户指定的原始表述实现(检测到 PATH 有 dsh 时一键回默认)。**可达面比 spec 场景窄**(仅"裸名 command 失效+dsh 已装"人群;checkout 用户多落在 degraded 面板,无此按钮)——§5 行已记录此边界,checkout 填表留池中。

### S19 首次关窗进托盘一次性通知(验收:首次一次、后续零打扰)

- **exactly-once(真机,送达级)**:删标记文件模拟全新安装 → 首次 WM_CLOSE → Action Center 入库时间戳**更新** + 标记文件落盘;二次启动唤起窗口再关 → 时间戳**不再更新**。文案 "The window is closed — DSH Desk keeps running in the tray (Alt+Shift+D or the tray icon brings it back)."
- 标记为独立文件 `tray-hint.shown`(不污染 config.json);写失败落日志(最坏情况=下次关窗重复提示,已注释声明)。

### S20 语言决策(验收:决策行落 §5)

- §5 新增决策行:英文-only=正式决策(受众全球开发者、上游官方语言 en),真实 i18n 呼声出现再议;README Notes 补一句。
- 本轮由用户目标授权二选一,选择"记录决策"而非 i18n 钩子(改动量最小且与受众一致)。

## §2.4 六条契约走查(真实 config:node 直启 checkout、--port 0)

| # | 契约 | 判定 | 证据 |
|---|---|---|---|
| 1 | 就绪→窗口显示认证 GUI | 过 | 真实 config 启动,新就绪行 port=11321,**WS ESTABLISHED owner=msedgewebview2**(金标准),窗口可见 |
| 2 | 托盘菜单全部可用 | 过 | Show/Restart/Edit config 本轮 6002 直投实点(Restart 出新端口就绪;Edit config 打开系统查看器=ZCode,日志 `opened ... in system viewer` 实证);Check for updates=S14 本轮实证;Open in browser=当日 S7/S5a 记录前证(不在用户在场会话开浏览器) |
| 3 | 热键 Alt+Shift+D | 过 | keybd 序列:藏→显两态实测 |
| 4 | 二次启动聚焦 | 过 | 二启后窗口显示、单实例计数=1 |
| 5 | 关窗藏托盘 server 续活 | 过 | WM_CLOSE 后隐藏,node 计数稳定(2) |
| 6 | Quit 无残留 | 过 | 托盘 Quit 后 dsh-desk 进程消失;node census 甄别:唯一匹配 `tsx/esm` 的 node 创建于 10:00(14h 前)、父进程存活、不在本轮任何 spawn 的 pid 名单=**机器常驻 dsh 开发实例**,非泄漏;其余 node 为本会话 ZCode MCP 服务器 |

## 独立审查(回合末第四道门)

无作者上下文子代理审 diff+skill 硬约束+AGENTS 红线,三问核对。结论:**无 P0;S2 生命周期不变量、脱敏、稳定表面契约、§2.4 语义全部确认保住;对比度与 Chromium 版本底线复算属实。** 修复:P1(spec 与 S18 背离无记录→本记录+§5 行补齐)、P2×2(theme stamp 逃逸路径→三处补 stamp;use_detected_dsh 静默失败→panel_note)已全部落码并重建复验(DARK PASS+PHASE2B PASS);P2 README 过时、P2 验证记录落盘在本批文档中完成。P3 项判定不修(fallback 硬编码 #333 属既有行为、4s 超时为知情取舍、onboarding 双 primary 按钮为有意强调、Cargo.toml 伪变更提交时确认)。

## 方法备忘(本轮新沉淀,已回写 AGENTS.md)

1. **toast 验证的判定层级**(FA 抑制环境):Action Center 入库时间戳(HKCU per-app `LastNotificationAddedTime`)是确定性仪器——比截屏 OCR 和 UIA 观察都稳;"无 toast failed 日志+时间戳更新"=送达级证据。
2. **WebView2 的 prefers-color-scheme 可信度**:本机被钉死为 light(暗色系统下仍亮,双注册表+广播无效)——凡依赖媒体查询的暗色实现都要有 Rust 侧真值兜底。
3. **WebView2 的 UIA 树不可用**(Chromium 懒激活),web 内容断言走"窗口可见=面板上屏 → PrintWindow 截图 → Umi-OCR";web 按钮点击走键盘 Tab+Enter(隐藏元素跳过焦点)。
4. Umi-OCR 启动:`D:\soft\Umi-OCR_Paddle\Umi-OCR.exe`,模型预热约 25s,之后 `python ~/.agents/skills/ocr/scripts/ocr.py` 直用。
5. 中断的构建会留损坏 PDB(LNK1285)——删 `target/debug/deps/*.pdb` 重链即愈。

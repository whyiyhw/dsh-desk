# S1 交付自验记录(2026-09-04/25 深夜)

> 对象:S1 · 就绪感知与超时降级(规范见 [spec-and-plan.md](spec-and-plan.md) §2.3)
> 构建:`src-tauri/target/debug/dsh-desk.exe`(基线 `5e0c2af` + 工作树 S1 改动;`cargo test` 3 passed、`cargo build` 零警告、`cargo fmt` 已过)
> 环境:本机 Windows 10 19045,WebView2 152,真实 config(`node --import tsx/esm … --port 0`)

## S1 验收四条(规范 §2.3)

| # | 验收条目 | 怎么测 | 结果 |
|---|---|---|---|
| 1 | 改掉 dsh 输出前缀模拟漂移,≤90s 出现含三按钮的可操作诊断 | config 指向假 dsh(`dsh-fake*.cjs`,输出漂移措辞 + 假 token URL `S1TEST-TOKEN-xyz-DO-NOT-LEAK`,永不打印就绪行) | t+90s 降级面板出现;裁剪截图逐字核对:说明文案 + `Open log`/`Open config`/`Retry` 三按钮全部在场 ✓ |
| 2 | 20s 时窗口出现被动提示 | 同上,另一轮(t+20s 截图) | 转圈 + "Starting the dsh web server…" + 淡黄提示框(含 `%APPDATA%\dsh-desk\dsh-desk.log` 路径)逐字命中;15s 定时器触发(日志行)✓ |
| 3 | 日志文件中就绪行不含 token(仅 scheme+host+port) | `grep -c "S1TEST-TOKEN" dsh-desk.log`;再 grep 脱敏形态 | token 出现 **0 次**;`http://127.0.0.1:39999…`、`http://127.0.0.1:3080…` 等脱敏形态在场(stdout/stderr 镜像全量走 `redact_urls`)✓;单测 2 条覆盖 query/path 两种 token 位置 |
| 4 | 正常路径与 §2.4 契约一致 | 见下节 §2.4 六条 | 全部通过 ✓ |

**S1 附加语义验证**:
- 超时不杀进程:90s 超时消息后假 dsh 持续输出至 t+100s+(watcher 仍活)✓
- 迟到就绪行恢复(Degraded→Ready):假 dsh 于 t+100s 打印 `  dsh web: ready → http://127.0.0.1:3080/?token=…`(前导空格+箭头措辞漂移),放宽后的匹配器接住并导航,AX 树文档级证实窗口已到该 URL ✓(同时验证前缀放宽)
- 三按钮各自工作:`open_log_file invoked` → 记事本打开日志(.log→txtfile 关联);`open_config_file invoked` → 系统查看器打开 config.json;`retry_server invoked` → 旧进程被杀("exited before printing its URL")→ 新进程 spawn(杀净→再起,无未杀先起)✓
- 就绪行解析对真实 dsh 格式(`dsh web: <url> (LAN: <url>)`)与旧逻辑逐字节等价,token 为 base64url 不含空格,无截断风险(源码 `web-app/src/index.ts:280` 核对)

## §2.4 回归契约走查(六条)

| # | 契约 | 结果 | 证据 |
|---|---|---|---|
| 1 | 就绪行后窗口显示并导航认证 GUI(无手工 token) | ✓ | AX 文档 = 就绪 URL;页面非 401(401 纯文本页会在 AX 暴露正文,新实例无此文本);日志中就绪行已脱敏 |
| 2 | 托盘四项全部可用 | ✓ | 精确几何点击(实测菜单矩形 (2000,988,2210,1090),行高 25.5):**Show** item1→窗口 toggle 隐藏(反向由热键验证);**Open in browser** item2→`opened the authenticated URL in the browser` + Chrome 新标签页 "DSH 本地构建"(= dsh zh locale `brand.localBuild`);**Restart** item3→杀旧→"exited before printing its URL"→新 spawn→新就绪行(端口已变);**Quit** item4→纯托盘退出,进程消失 |
| 3 | 热键 Alt+Shift+D 切换窗口 | ✓ | 双向:合成 chord 可见→隐藏;keybd_event 合成隐藏→显示 |
| 4 | 二次启动唤起并聚焦 | ✓ | 第二进程 exit 0,隐藏窗口被 show+focus |
| 5 | 关窗隐藏到托盘,server 继续运行 | ✓ | AXPress Close → 窗口从枚举消失,app 与 dsh 子进程存活 |
| 6 | Quit 后无 dsh/node 残留 | ✓ | 托盘 Quit 实例:自身 dsh 子进程随退;终态清点仅存 3080 常驻实例(非本次测试所属);当晚多次收尾清点均为零孤儿(注:轮次间由测试脚本 `stop-process -force` 造成的两个孤儿与产品无关,已清理) |

## 测试方法备忘(后续 S 项可复用)

- 托盘程序化驱动:tray-icon 0.24 只认 `WM_USER_TRAYICON=6002` 回调(lParam=鼠标事件);PS 脚本须 `SetProcessDPIAware()`,菜单弹出后用 GetWindowRect 实测矩形按行高点击;投递须落在就绪行 set_focus 后的前台窗口期内,就绪检测用**全文件计数**而非尾行(尾行有 O_APPEND 多线程写序假象)
- 失败注入用假 dsh(`%TEMP%\s1-fake/`);`%TEMP%\s1-tray/` 是托盘驱动脚本(blitz2.ps1 为最终形态)

## 已知边界(不阻塞 S1)

1. 按 Retry 瞬间旧监视线程的 "exited" 诊断会短暂覆盖加载页——S2 代数标记将根治(§2.3 S2 方案即为该根因)
2. open-browser 在 Starting 态(url=None)为静默 no-op,建议 S4 补一条提示日志
3. 本机一次 WebView2 首导航落在 `tauri.localhost` 错误页(强杀实例残留脏 profile,清进程后自愈)。~~一次 401(用户 F5 丢 query)~~ **修正(2026-09-05 用户复现后重查)**:22:44 与 00:07 两次 401 同根因——强杀风暴弄脏 EBWebView cookie 库,SameSite=Strict 认证 cookie 存不进/发不出;服务端无头链路(token→303+Set-Cookie→带 cookie `/`→200)全通,唯独 webview 401。处置(杀应用专属 msedgewebview2→删 `…\EBWebView\Default\Network\Cookies*`→重启)与判定金标准(netstat 见 `msedgewebview2↔node` mux WS ESTABLISHED)见仓库 AGENTS.md「E2E」节。非 S1 回归,但本文"页面非 401"的 AX 判据不可靠(窗口最小化时正文不暴露,曾两度误判),以 WS 连接为准。

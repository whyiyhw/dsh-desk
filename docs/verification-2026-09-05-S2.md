# S2 交付自验记录(2026-09-05)

> 对象:S2 · 生命周期竞态收敛,代数标记(规范见 [spec-and-plan.md](spec-and-plan.md) §2.3 / §3.1)
> 构建:`src-tauri/target/debug/dsh-desk.exe`(**S1+S4+S2 合并树**,含并行 S4 会话 01:17 审查修复;`cargo test` 10 passed,`cargo fmt --check` 过)
> 环境:本机 Windows 10 19045,WebView2 152.0.4191.62,真实 config(`node --import tsx/esm … --port 0`,cwd deepseek-harness checkout)

## 实现摘要(lib.rs)

- `ServerState` 新增:`generation: AtomicU64`(每次 spawn、每次主动 kill、已上报的退出认领各 mint 一次)、`lifecycle: Mutex<()>`(串行化"杀净→等退→再起"全周期,只在 worker 线程持有;child 字段锁永不跨 `spawn()`)、`exiting: AtomicBool`(锁内复检,堵 Quit 后排队 worker 复活 server)。
- watcher/timer 持有出生代数,`claim_exit`(child+url 在同一临界区随代数 bump 一起交接)、`kill_registered_child`(bump 与 take 同在 child 锁内)、`is_still_starting`、url 写入在 url 锁内复读代数——旧代 EOF 只留一行 `superseded generation` 日志,不碰新状态。
- `run_lifecycle_cycle`(boot 与 restart 共用)在 worker 线程串行执行;托盘 Restart 与面板 Retry 同路。
- Quit 分支与 `ExitRequested`:置 exiting → 拿 lifecycle → kill(主线程至多阻塞一个在飞周期)。
- 单测 4 条锚定:`stale_watcher_eof_leaves_new_server_state_alone`(G2b 回归:旧 watcher EOF 不得偷走新 server 的 url/child)、`deliberate_kill_retires_generation_and_clears_url`、`timers_act_only_while_current_and_not_ready`、`claim_exit_hands_the_child_over_atomically`(P1 修复的真实进程交接)。

## S2 验收(spec §2.3,判定式逐条)

| # | 验收条目 | 怎么测 | 结果 |
|---|---|---|---|
| 1 | 记录基线 → 连点 Restart×10 → 末次后等 30s:进程数 == 基线 | 实例 pid 25348;基线 node 子进程 1;S4 配方(tray-run,溢出浮层+GetRect 真右键)连点 Restart,**10 次效果确认转换**(效果重试额外触发,合计 16 个完整周期);静默 32s 后清点 | 子进程恒 = **1** ✓;`superseded generation` 静默退出 **17** 次 ✓;`the dsh server (pid …) exited` 误报 **0** 次(S1 已知边界 1 根治:Restart/Retry 瞬间旧 watcher 诊断不再覆盖加载页)✓;90s 超时降级误触发 0 ✓ |
| 2 | 120s 内窗口回 Ready 或正确诊断态 | 风暴结束后日志末三行:superseded → started(pid 24624)→ `dsh web: http://127.0.0.1:10306…`;点击停止后零新增 spawn(无自循环) | ✓ |
| 3 | **然后 Quit:无任何 dsh/node 残留进程**(抓孤儿关键项) | 托盘 Quit(行 5)后清点:dsh-desk 0;本次测试全部 node 子进程(含风暴中 16 个被杀代的树)零存活;终态 node 仅 3080 开发实例等用户自有 3 进程 | ✓ **零残留** |

补充验证:

- **第一轮风暴(实例 pid 7240,00:44-00:52)**:S1 遗留 6002 直投配方仍有效的窗口期内 9 连击,同样 9 次 superseded 静默 + 0 误报 + 清点恒 1——与第二轮互为佐证。
- **风暴中观察到的代数语义**:日志 `superseded generation (N)` 的 N 单调递增且只取奇数(kill +1、spawn +1),与设计一致,可从日志反推生命周期轨迹。
- **串行化语义说明**:S2 第 4 条(串行杀→等→起)耦合 AppHandle 无法单测,以本真机风暴(16 个周期零交错、零泄漏)+ Quit 零残留作为防线;单测锚定的是纯状态代数部分。

## §2.4 回归契约走查(六条)

本会话在合并树上串行实测(实例 25348):①就绪行→窗口显示并导航(日志就绪行 + 窗口可见;S4 记录另有 AX 文档级证据)✓;②托盘项:Show(行 1→隐藏切换)✓ / Open in browser(行 2→`opened the authenticated URL` 日志 +1,浏览器实际打开)✓ / Restart(行 3×16)✓ / Quit(行 5→退出)✓(+S4 的 Edit config 行 4,S4 记录验证);③热键 Alt+Shift+D 双向 ✓;④二次启动礼让 + show ✓;⑤关窗藏托盘、app+server 存活 ✓;⑥Quit 后零残留 ✓。
另:并行 S4 会话的 [verification-2026-09-05-S4.md](verification-2026-09-05-S4.md) 在同一合并构建上独立通过 §2.4 六条(其 Restart 测试同样观察到 S2 的 superseded 静默语义),互为交叉证据。

## 回合末独立审查(已执行)与修复

无作者上下文的独立子代理审查(读全量 lib.rs + spec S2 语义 + 硬约束,核对 tauri-runtime-wry 2.11.4 源码):**1×P1 + 5×P2 + 1×P3**,无违反硬约束。处置:

| 发现 | 级别 | 处置 |
|---|---|---|
| `claim_exit` 对 child 的交接不原子(take 在第二个临界区,kill 的 bump 在 child 锁外;微秒窗口内 watcher 可抢先 take 走 Child 直接 drop,kill 扑空 → 若"EOF≠真死"进程树活过 Quit) | **P1** | **已修**:child 的 take 移进 claim_exit 临界区(返回 `Option<(Option<Child>, bool)>`);kill 的 bump 移到 child 锁内;`server_exited` 拿到 child 后 `try_wait` 仍活则补 `kill_child_tree`(顺带堵"EOF≈退出"假设);`kill_child_tree` 增加"已自然退出只 reap 不 taskkill"(防 pid 复用误杀);新增单测锚定交接 |
| watcher 写 url 的 check-then-act 窗口(死服务器 URL 可能写回并导航) | P2 | **已修**:url 锁内复读代数,不一致放弃写入与导航 |
| 定时器 is_still_starting 通过后到行动之间的代数漂移(旧代 90s 面板可能闪现在新尝试上,自愈但可避免) | P2 | **已修**:行动前复读代数(S4 审查标记为冗余、P3;保留——纵深防御,成本一行) |
| lifecycle 锁内 UI 调用依赖 tauri `tracing` feature 关闭(eval 一旦变同步等待即死锁) | P2 | **已核+已记**:当前构建确未启用 tracing(审查者查 `cargo tree -e features`);不变量写入 lifecycle 字段文档 |
| generation 字段文档漏列 claim_exit 也是 mint 方;Quit 阻塞时长"briefly"不实 | P2 | **已修**:两处文档更正(退出认领也 mint;主线程至多阻塞一个在飞周期) |
| 串行路径无单测 | P2 | **明示接受**:见上节说明,真机风暴为防线 |
| 其余三问(锁序反演/同线程重入/spawn 失败与 onboarding 早退路径/定时器旧代误报/bump 原子性) | — | 审查者逐项核查未见问题;S4 会话的独立审查对合并树另报 6×P3(其范围已修,S2 范围 1 条即上表定时器冗余项) |

## 测试方法与踩坑(本轮新沉淀,已回写 AGENTS.md)

1. **S1 的 6002 直投配方已死**:DSH 图标现常驻溢出浮层,tray-icon 0.24.2 在图标不可见时 `Shell_NotifyIconGetRect` 失败即静默 return 0(6002 投递"没反应"的根因,S4 会话读 crate 源码确诊)。托盘驱动一律用 S4 配方(`%TEMP%\s4-test\tray-run.ps1`):开浮层(BM_CLICK 前先查可见性,chevron 是 toggle)→ 浮层开着时 GetRect(uID 扫 1..8)→ 真右键 → 实测菜单矩形按行点 → 效果断言重试。
2. **浏览器是托盘菜单的头号焦点杀手**:Open in browser 留下的 Chrome 前台窗口让浮层右键菜单连开 4 次全失败;关掉 Chrome 后一次成功。托盘驱动前先清浏览器。
3. **并行会话共享真机的排障第一课**:托盘图标"消失"、PostMessage 失效等环境谜题排查 40 分钟,答案其实躺在并行 S4 会话 01:20 就写好的验证记录里(含新配方成品脚本)。**排障前先 `ls docs/` 看有没有并行会话的新记录、tail 一下 dsh-desk.log 有没有外国实例的轨迹**(本轮日志里 `WebView2 runtime 118.0.9999.0` 就是 S4 注册表测试的指纹,当时没认出来)。
4. 图标在溢出浮层内的格子会随邻居图标增删**漂移**(风暴前后从 (2363,1312) 挪到 (2221,1350)),视觉/旧坐标都不可靠,每次都要现取 GetRect。
5. effect 断言字符串经 `powershell -File` 二次传参时含反引号的 pattern 会踩引号坑,导致 tray-run 效果重试多点击(本轮风暴因此多出 6 个周期——反而强化了测试,但断言要避免反引号 pattern)。

## 环境还原

- config.json:已还原为真实启动配置(node + `--port 0`);`config.json.pre-s2` 为本轮备份留存。
- 进程:终态 `dsh-desk` 0、node 仅用户自有 3 进程(3080 开发实例/editor/astro)、notepad 已清;本轮测试触发的 Chrome 窗口已关。
- `%TEMP%\s2-test\`:全部测试脚本留存(final-storm.ps1 / final-contract.ps1 / finisher.ps1 为最终形态;diag*.ps1 / hover-*.ps1 / tray-enum*.ps1 为排障中间产物,可删);`%TEMP%\s4-test\`(S4 会话)含托盘驱动成品 tray-run.ps1。

## 一句反思

环境谜题(图标消失/消息失效)排查的最贵一步不是任何技术手段,而是**没有第一时间去读并行会话刚落盘的验证记录**——同一台机器上另一个 agent 一小时前踩过同一个坑并写下了答案;共享真机时,`docs/` 与共享日志的"外国轨迹"是排障的第一现场。

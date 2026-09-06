# S9 验证记录 — 窗口状态记忆(2026-09-05)

> 交付物:`tauri-plugin-window-state`(v2.4.1),`with_state_flags(SIZE | POSITION)`——**VISIBLE 与 MAXIMIZED 刻意排除**。提交 `9118fcd`(v0.2.0)。

## 验收口径与证据

spec §2.3 P2 池对 S9 的定义 = tauri-plugin-window-state("几乎零成本,顺手"),无独立验收条款;以 **§2.4 六条契约不破** + 几何持久化为验收。

**几何持久化 — 通过(真机,2026-09-05)**

1. 启动 → 就绪 → SetWindowPos 到 (150,150) 1000×700 → 托盘 Quit → 重启:恢复 **rect 150,150,1150,850 size 1000×700**——位置与尺寸精确持久化(保存于 `RunEvent::Exit` 自动 `save_window_state`,状态文件 `%APPDATA%\com.whyiyhw\dshdesk\.window-state.json`)。

**hidden-start 契约(含 P1 回归)— 通过(真机)**

- 独立审查抓到 **P1**:`StateFlags` 默认 `all()` 含 MAXIMIZED,而插件 restore 的 `maximize()` 经 tao 落到 Win32 `ShowWindow(SW_MAXIMIZE)`——**激活并显示**隐藏窗口。用户"最大化使用→Quit"后下次启动窗口会在就绪行前弹出抢焦点,破坏 §2.4 契约 1。修复 = 从 flags 去掉 MAXIMIZED(代价:最大化状态不记忆)。
- 回归验证(真机,确定性判据):`maximize → Quit → 重启`,在窗口**首次可见的瞬间**读日志——就绪行已在盘(watcher 线程先 `log_line` 后 `open_gui`,物理顺序由代码保证);窗口恢复为**未最大化** 1000×700。探测脚本 `%TEMP%\p3-smoke\launch-probe.ps1`。
- 轮询时间戳的亚毫秒"倒序"是测量伪影(同一轮询周期内先盖 visible 戳后盖 ready 戳)——判据必须用"首次可见时刻日志内容",不是两个时间戳相减。

**§2.4 六条在 v0.2.0 最终二进制全过**(菜单/热键/二次启动/关窗/Quit 零残留,与 S13 同批走查,见 [S13 记录](verification-2026-09-05-S13.md)链路)。

## 已知小瑕疵(记录在案,不修)

- **最大化会话后恢复位置 = 显示器原点**(实测 -9,-9)而非最大化前位置:插件的 `Moved` 事件处理器**不设最大化守卫**,最大化把缓存 x/y 写成满屏原点;Exit 时 `update_state` 因 is_maximized 跳过位置写,缓存保留的就是原点。窗口尺寸仍正确恢复、不破坏契约——若社区在意,升级插件或改为"显示后手动 restore"再议。

## 审查修正

P1(MAXIMIZED 显示隐藏窗口,审查发现,上述修复)+ 确认项:Quit→`app.exit(0)`→`RunEvent::Exit`→插件保存**可靠**(run 闭包未 prevent_exit);close-to-tray 隐藏时保存几何**有效**(hide 不改 rect,hidden≠minimized)。

## 环境还原

见 [S13 记录](verification-2026-09-05-S13.md)。

## 一句反思

"零成本顺手插件"的成本全在默认 flags 里——`all()` 把 VISIBLE/MAXIMIZED 一起带上,两个都会在启动期把隐藏窗口拉出来;读一遍插件源码的 restore 路径(而非只看 README)才挡住这次违约,而真机只测非最大化几何根本测不出来。

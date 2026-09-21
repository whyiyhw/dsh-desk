# docs/ 索引

| 文档 | 类型 | 说明 |
|---|---|---|
| [spec-and-plan.md](spec-and-plan.md) | 规范(唯一真源) | 阶段门禁、S1-S13 规范与验收标准、S14-S20 UX 债务候选池(择优不承诺)、§5 决策记录 |
| [postmortem-2026-09-04-webview2-114.md](postmortem/postmortem-2026-09-04-webview2-114.md) | 事故复盘 | WebView2 运行时冻结在 114,GUI 渲染但"连接异常";升级到 152 的完整踩坑路径 |
| [verification-2026-09-05-S1.md](verification/verification-2026-09-05-S1.md) | 交付自验 | S1 就绪感知与超时降级:验收四条 + §2.4 六条走查 + 托盘程序化驱动配方 |
| [verification-2026-09-05-S6.md](verification/verification-2026-09-05-S6.md) | 交付自验 | S6 品牌图标:图标管线缓存坑、cursor-agent CLI 用法 |
| [verification-2026-09-05-S4.md](verification/verification-2026-09-05-S4.md) | 交付自验 | S4 首跑引导/托盘 Edit config/WebView2 版本门禁:验收三条 + §2.4 六条 + 溢出浮层托盘驱动配方 + 并行会话互踩处置 |
| [verification-2026-09-05-S2.md](verification/verification-2026-09-05-S2.md) | 交付自验 | S2 生命周期代数标记:Restart 风暴 16 周期/Quit 零残留验收 + P1(child 交接原子性)审查修复 + Chrome 焦点杀手/浮层格子漂移 |
| [verification-2026-09-05-phase0.md](verification/verification-2026-09-05-phase0.md) | 阶段基线 | Phase 0 立整基线:release 构建冒烟 §2.4 六条 + 托盘驱动配方 v3(6002 直投) |
| [verification-2026-09-05-S3.md](verification/verification-2026-09-05-S3.md) | 交付自验 | S3 CI+安装包:tag→Release 链路(v0.1.0 已发)+ 安装版 §2.4 六条 + prerelease fixture 流程 |
| [verification-2026-09-05-S12.md](verification/verification-2026-09-05-S12.md) | 交付自验 | S12 发布物料:README 四节 + embedBootstrapper 语义勘误 + 版本真源 CI 守卫 |
| [verification-2026-09-05-S5a.md](verification/verification-2026-09-05-S5a.md) | 交付自验 | S5a 更新检查:semver 全序 + 负向/正向真机验收(v0.1.1 fixture 打开 Releases 页) |
| [verification-2026-09-05-S9.md](verification/verification-2026-09-05-S9.md) | 交付自验 | S9 窗口状态记忆:MAXIMIZED 显示隐藏窗口的 P1 审查修复 + 几何持久化/hidden-start 真机回归 |
| [verification-2026-09-05-S13.md](verification/verification-2026-09-05-S13.md) | 交付自验 | S13 崩溃可诊断:panic hook/横幅(含命令行)/轮转挪 setup 的跨进程理由 + §2.4 六条(v0.2.0) |
| [verification-2026-09-05-S7.md](verification/verification-2026-09-05-S7.md) | 交付自验 | S7 托盘状态可视化:ready/not-ready 双色 + 意外退出 toast + 审查三修;发布哈希/SmartScreen FAQ 增补;Win+B 开浮层配方 v4(v0.2.1) |
| [postmortem-2026-09-05-host-hyperv-broken.md](postmortem/postmortem-2026-09-05-host-hyperv-broken.md) | 事故复盘 | 宿主 Hyper-V 组件库损坏(载荷停 2020 版):Phase 2 虚机门禁放弃执行的完整证据链 + VM 排障仪器/免交互安装配方沉淀 |
| [verification-2026-09-06-S14-S20.md](verification/verification-2026-09-06-S14-S20.md) | 交付自验 | S14-S20 UX 批次:更新检查 toast/文案分离/可访性/视觉身份与暗色(WebView2 主题钉死 light 的发现与 data-theme 双通道)/Use detected dsh/首次关窗一次性通知;toast 送达级判定仪器(Action Center 时间戳)+FA 抑制环境事实 |
| [verification-2026-09-06-S22.md](verification/verification-2026-09-06-S22.md) | 交付自验 | S22 子进程控制台窗口抑制(CREATE_NO_WINDOW×5 站点):探针对照 pre/post 修复(43→0 命中)、按进程树归属的控制台窗枚举仪器、与用户在跑实例共存的探针补丁披露 |
| [verification-2026-09-06-S23.md](verification/verification-2026-09-06-S23.md) | 交付自验 | S23 多实例(--instance):Phase A 命名实例全过(共存/认证 OCR/cookie 隔离/同实例弹回唤醒)+ 独立审查 1×P1+2×P2+6×P3 修复后重建复验;Phase B 默认实例 §2.4 六条全量走查亦过(合并门禁清空);single-instance 插件不可参数化/cookie 不分端口互踩的实证 |
| [verification-2026-09-21-S21.md](verification/verification-2026-09-21-S21.md) | 交付自验 | S21 无缝 caption 真机验证:抓到并修复 P0(setup 内 with_webview 同步自死锁——v0.3.0 首启必挂,旁路线程修复)+关闭钮 CSS;修复后无边框双轨全项过(像素直证侧栏 y=0 起无白带、app-region 拖拽/双击最大化/系统菜单、自家页三键);tao 0.35 set_decorations 不摘 WS_CAPTION 位的行为级偏差记录 |

## 命名约定(即分类)

- `verification/verification-YYYY-MM-DD-<S项>.md` —— S 项交付自验(怎么测的、证据在哪、哪些没测到)
- `postmortem/postmortem-YYYY-MM-DD-<主题>.md` —— 事故复盘(现象 → 根因 → 处置 → 规矩)
- 根级仅留 `README.md`(本索引)与 `spec-and-plan.md`(唯一规范源)

**2026-09-06 已拆子目录**(S22/S23 收尾时双触发齐备:docs/ 18 个文件>10、verification 前缀 14 个>5),拆时全量互链已同步(spec §5 / AGENTS.md / skill / 本索引)。新文档按前缀进对应子目录;再膨胀时按年份或 Phase 二次分组即可,不再设触发条件。

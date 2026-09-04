# docs/ 索引

| 文档 | 类型 | 说明 |
|---|---|---|
| [spec-and-plan.md](spec-and-plan.md) | 规范(唯一真源) | 阶段门禁、S1-S13 规范与验收标准、§5 决策记录 |
| [postmortem-2026-09-04-webview2-114.md](postmortem-2026-09-04-webview2-114.md) | 事故复盘 | WebView2 运行时冻结在 114,GUI 渲染但"连接异常";升级到 152 的完整踩坑路径 |
| [verification-2026-09-05-S1.md](verification-2026-09-05-S1.md) | 交付自验 | S1 就绪感知与超时降级:验收四条 + §2.4 六条走查 + 托盘程序化驱动配方 |
| [verification-2026-09-05-S6.md](verification-2026-09-05-S6.md) | 交付自验 | S6 品牌图标:图标管线缓存坑、cursor-agent CLI 用法 |
| [verification-2026-09-05-S4.md](verification-2026-09-05-S4.md) | 交付自验 | S4 首跑引导/托盘 Edit config/WebView2 版本门禁:验收三条 + §2.4 六条 + 溢出浮层托盘驱动配方 + 并行会话互踩处置 |
| [verification-2026-09-05-S2.md](verification-2026-09-05-S2.md) | 交付自验 | S2 生命周期代数标记:Restart 风暴 16 周期/Quit 零残留验收 + P1(child 交接原子性)审查修复 + Chrome 焦点杀手/浮层格子漂移 |
| [verification-2026-09-05-phase0.md](verification-2026-09-05-phase0.md) | 阶段基线 | Phase 0 立整基线:release 构建冒烟 §2.4 六条 + 托盘驱动配方 v3(6002 直投) |
| [verification-2026-09-05-S3.md](verification-2026-09-05-S3.md) | 交付自验 | S3 CI+安装包:tag→Release 链路(v0.1.0 已发)+ 安装版 §2.4 六条 + prerelease fixture 流程 |
| [verification-2026-09-05-S12.md](verification-2026-09-05-S12.md) | 交付自验 | S12 发布物料:README 四节 + embedBootstrapper 语义勘误 + 版本真源 CI 守卫 |
| [verification-2026-09-05-S5a.md](verification-2026-09-05-S5a.md) | 交付自验 | S5a 更新检查:semver 全序 + 负向/正向真机验收(v0.1.1 fixture 打开 Releases 页) |

## 命名约定(即分类)

- `spec-*.md` —— 规范与计划(唯一真源,只此一份)
- `verification-YYYY-MM-DD-<S项>.md` —— S 项交付自验(怎么测的、证据在哪、哪些没测到)
- `postmortem-YYYY-MM-DD-<主题>.md` —— 事故复盘(现象 → 根因 → 处置 → 规矩)

文件名前缀即类型,新文档一律沿用;**暂不建子目录**。拆目录的触发条件(满足其一再拆,拆时同步更新 AGENTS.md / skill / spec §5 / 各验证记录里的互链):

1. docs/ 超过约 10 个文件;
2. 单一前缀(如 verification/)超过 5 个。

结构是负债——在它挡路之前,平铺 + 前缀 + 本索引够用。

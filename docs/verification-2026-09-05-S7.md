# S7 交付自验 — 托盘状态可视化 + 发布物料增补(2026-09-05,v0.2.1)

对应 spec §2.3 P2 表 S7 行的验收口径 ①-⑤ + 两项范围增补(release SHA-256 / README SmartScreen FAQ)。
测试机 = 本机(安装版 NSIS `/S` 覆盖安装自本分支构建的 `dsh-desk_0.2.1...` 前身 `0.2.0+S7` 构建)。

## ① toast 仅"当前代数"的意外退出才弹

- 实现依据:`server_exited` 只在 `claim_exit` 成功(= 该 watcher 代数仍当前)后才调通知;Quit/Restart/二次启动的 kill 都先 bump 代数(S2 语义),旧 watcher 的 EOF 落在 `None` 分支只写日志。
- 实测:taskkill 服务器进程树(pid 14356)→ toast 出现(视觉逐字核对:"DSH Desk"/"The dsh server exited unexpectedly. Open the dsh-desk window for what to do next.");日志同步出现 `the dsh server (pid 14356) exited`。
- 用户主动路径实测:托盘 Restart(6002 直投开菜单点第 3 行)→ 新 pid 起、ready、**无 toast**;托盘 Quit → 退出、**无 toast**(证据在 D:\tmp\s7-smoke\contract.log 全程无 exit-toast 相关异常)。

## ② 托盘双色 = ready / not-ready

措辞按独立审查 P3 修正:"stopped" 在慢启动降级态(服务器活着、仅无就绪行)会说谎,故灰面=not-ready(涵盖 启动中/降级/引导态/已死)。

- 就绪(彩色):饱和度均值 **17.5**(icon-running.png,50×50 采样 625px);溢出浮层目视确认蓝色 dsh 字标。
- 意外退出(灰):**1.1**(icon-stopped.png)——Rec.601 亮度 55% 合成,对比度肉眼可辨。
- Restart → ready 后彩色回归:浮层整体截图目视确认(flyout-now.png,蓝色 dsh 在场)。
- 已知小瑕疵(记录不修):Restart 的 kill→再起间隙图标保持彩色(瞬态数秒、用户主动触发);"EOF≈退出"边界(孙进程持管道)属 S2 已知,非本轮引入。

## ③ 灰标运行时合成(无第二资产)

`gray_image` 单元测试锚定(`cargo test` 15 passed,新增 `gray_image_desaturates_dims_and_keeps_alpha`:Rec.601→124→55%=68、RGB 相等、alpha 不动、全透明像素不动)。规避 S6 的 embed-resource 图标缓存坑。

## ④ toast 失败必须落日志

按独立审查 P2 修正:`let _ =` → `if let Err(e) = ... log_line(...)`。附注:Windows 下 toast 仅对安装版生效(开始菜单快捷方式/AUMID),dev/portable 静默失败属预期但必须留痕。本轮 toast 实测即在安装版上进行。

## ⑤ §2.4 六条回归(本构建)

| 契约 | 结果 | 证据 |
|---|---|---|
| 认证跳转(GUI 在线) | ✅ | 服务器端口(5238)4 条 ESTABLISHED(netstat 金标准) |
| 托盘项 | ✅ | Restart(菜单点行→新 pid→ready)、Quit(应用退出+零残留)、Edit config(日志 `opened ...config.json in system viewer`)、Show(同机制点行成功;hidden→shown 隔离验证沿用 S1/S4/S13 期记录,本轮热键 True→False→True 等价覆盖显示/隐藏回路) |
| 热键 Alt+Shift+D | ✅ | True→False→True 两轮 |
| 二次启动 | ✅ | 进程数 1→1,窗口显示(single-instance 让路) |
| 关窗藏托盘 | ✅ | WM_CLOSE 后 visible=False、进程活 |
| Quit 零残留 | ✅ | 应用退出;仅存 1 个 node = 常驻 dev 实例(`tsx/esm apps/cli/src/bin.ts web`,非本应用子进程) |

## 范围增补 A:release 正文发布 SHA-256

release.yml release job 新增 checksum 步骤(`sha256sum` → `body` 输入,与 `generate_release_notes: true` 并用 = 自定义 body 在前、自动笔记拼接在后,GitHub API 语义经查证)。
**实测**:v0.2.1 构建(此提交树)的 Release 正文含 `## SHA-256 checksums` 段且两个安装包各有一行哈希;`certutil -hashfile` 本地复算与发布值一致。
(本段于 CI 构建完成后回填实测值——见下方"CI 实测"节。)

## 范围增补 B:README SmartScreen FAQ

Install 节简注改为指向 FAQ;FAQ 新增条目:为何弹(未知发布者信誉缺口≠恶意检出)/Run anyway 的安全依据(仅官方 Releases + CI 可复现构建)/`certutil -hashfile` 校验方法(依赖增补 A 的发布哈希)。

## 独立审查(回合末第四道门)

无作者上下文子代理,diff + skill 硬约束 + AGENTS.md 红线三问。结论:0×P1、2×P2(toast 错误被吞;spec 缺 S7 验收定义/交付行——并指出 release 哈希与 README FAQ 属未记录范围增补)、1×P3(降级态 tooltip 说谎)。**全部修复**:错误落日志、§2.3 S7 行验收口径 + §5 交付行(本行)补齐、措辞改 not-ready。审查确认无锁跨 spawn、无 token 新泄露面、无通知风暴路径。

## CI 实测(回填区)

- `cargo test`:15 passed(本机)。
- **v0.2.1 正式构建绿**(run 33940461089,10m05s):NSIS 4,445,951B + MSI 5,718,016B 挂 Release。
- **Release 正文含 `## SHA-256 checksums` 段**(两文件各一行,自动变更日志拼接在自定义 body 之后,与 softprops/API 语义查证一致)。
- **端到端校验链实测**:下载 Release 的 setup.exe → `Get-FileHash` 本地复算 = `992b2356e6eb7ae68adfea95c64c89033743e4fc1cbec426aa00bb727c27be7b` = 发布值逐位一致。
- **发版产物真机冒烟**(NSIS `/S` 安装):横幅 `dsh-desk v0.2.1 starting`、Ready、服务器端口 4 条 ESTABLISHED。

## 未测项(如实)

- Windows 通知的"专注助手关闭勿扰"场景(系统抑制 toast 不弹属 OS 行为,应用层无法也无需干预)。
- toast 在 **MSI** 安装版上的表现(仅测 NSIS 安装版;两者都建开始菜单快捷方式,机制相同)。
- 灰标在浅色/深色任务栏下的对比度(仅在默认深色下目检)。

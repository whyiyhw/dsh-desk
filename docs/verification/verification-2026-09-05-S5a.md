# S5a 验证记录 — 轻量更新检查(2026-09-05)

> 交付物:托盘 **Check for updates** 菜单项 → 独立线程 `ureq` GET releases 列表 → semver 比对 → 有新版打开 Releases 页。提交 `8c34503` + 审查修正。约 90 行(含注释与测试),无签名密钥。

## 验收对照(spec §2.3 S5a)

**验收:在真实仓库发一个更高版本号的 prerelease,旧安装能检出并打开 Releases 页 — 通过(2026-09-05 真机)**

- **旧安装**:v0.1.0 安装版(NSIS 装于本机,即 S3 验收产物)。
- **负向先行**(顺手验证):仓库最新 release 为 v0.1.0(等于本机版本)时点击 → 日志 `dsh-desk: no newer release than 0.1.0; newest published is v0.1.0`,不打开浏览器。
- **fixture**:分支 bump 三处版本至 0.1.1 → `gh release create v0.1.1 --target <分支> --prerelease`(cascade 构建全绿、prerelease 标记保留、产物挂载;流程见 [S3 记录](verification-2026-09-05-S3.md)方法备忘)。
- **正向**:点击 Check for updates → 日志:
  ```
  dsh-desk: checking for updates...
  dsh-desk: release v0.1.1 is newer than this build (0.1.0) — opening the releases page
  dsh-desk: opened the releases page
  ```
  默认浏览器窗口标题实锤:`Release v0.1.1 (S5a acceptance fixture) · whyiyhw/dsh-desk - Google Chrome`——打开的正是该 release 页。
- fixture 测后已删(release/tag/分支)。

## 实现与审查修正

- **端点偏离 spec 原稿并已回写**(审查 P2-2):用 `GET /releases?per_page=1`(取最新一条)而非 `/releases/latest`——后者**不含 prerelease**,与"用 prerelease 测验收"的条款自相矛盾;列表端点含 prerelease,draft 对未认证读取不可见。spec §2.3 已修订。
- **semver 全序**(审查 P2-1):`ReleaseVersion` 按三段数字 + 可选 `-prerelease` 后缀排序——纯发布 > 同号预发布(`1.0.0 > 1.0.0-rc1`)、预发布标识符数值按值比较(`rc.2 < rc.10`)、前缀 < 扩展(`rc < rc.1`)。不可解析 tag → `None` → 按无更新处理(日志说明),绝不猜。
- **并发模型**(审查三问确认):`check_for_updates` 在独立线程执行 HTTP,全程零 `ServerState` 锁,托盘 handler 立即返回;`UPDATE_CHECK_IN_FLIGHT` 原子去抖合并连点(实测两轮顺序执行,无并发);10s 整体超时;网络/解析失败仅 `log_line`,绝不干扰运行中的 server。
- **日志措辞**(审查 P3-9):无更新时为 `no newer release than {current}`,不说 "up to date"(本地构建可能新于已发布版)。
- `cargo test` 12 passed,其中 S5a 两条:`release_tags_compare_numerically`、`prerelease_tags_follow_semver_ordering`。

## 已知缺口(记录在案)

- **无更新的点击无用户可见反馈**(仅日志)——托盘气泡/通知属 S7(系统通知)范畴,本项 spec 也只要求"有新版则打开";留待 S7 若启用则补。
- 更新检查不自动运行,仅手动触发(spec 如此定义)。

## 环境还原

见 [S3 记录](verification-2026-09-05-S3.md)环境还原节(同批真机会话)。

## 一句反思

验收条款自己会打架——"用 prerelease 测"与"`/releases/latest` 端点"在 spec 里共存了两天,实现时才暴露;写验收时顺手推演一遍数据流,矛盾当场可见。

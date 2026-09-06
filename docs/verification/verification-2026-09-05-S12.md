# S12 验证记录 — 发布物料(2026-09-05)

> 交付物:README(Install from release / Support / FAQ / versioning)、`tauri.conf.json` webviewInstallMode=embedBootstrapper、版本真源策略(tauri.conf.json)+ CI 守卫(`scripts/check-versions.mjs`,在 S3 构建链前置执行)。提交 `62e430b` + 审查修正。

## 验收对照(spec §2.3 S12)

**验收:一名非技术 Windows 用户仅按 README 从 Releases 完成安装并到达 Ready — 本机部分验证,虚机门禁保持开放**

已验证的链路(本机,2026-09-05):
- README 描述的下载物(NSIS `dsh-desk_0.1.0_x64-setup.exe`)真实存在于 Releases,静默安装 exit 0,装完启动 10.8s 到达 READY + WS ESTABLISHED(见 [S3 记录](verification-2026-09-05-S3.md));
- README 各新增节与实际行为逐项核对(独立审查复核):安装器产物名、embedBootstrapper 联网语义、Update 行为(不自动原地更新)、版本真源声明、FAQ 的 90s 降级与 401 描述,均与实现一致。

未覆盖:非技术用户在**干净虚机**上"仅按 README"走通(本机非干净机、config 非默认)。**Phase 2 虚机门禁因此仍开放**。

## 物料清单与勘误

1. **README 新增节**:
   - *Install (from a release)*:Win10+、下载 NSIS(推荐)/MSI、SmartScreen"More info → Run anyway"指引与"仅从官方 Releases 下载"声明、无 WebView2 且离线机器的处理(直接给出独立运行时直链 fwlink 2124701,不引用应用内链接——该场景窗口开不出来,应用内链接不可达,审查 P3-5)、Updates 说明;
   - *Support*:GitHub Issues + 附日志,脱敏声明**收窄为"URLs are redacted"**(审查 P3-6:机制只裁 URL,声明不能宽于机制);
   - *FAQ*:401 预期差(spec 2026-09-05 增补条目)、"still starting…"面板含义、为什么只依赖一行 stdout(架构赌注 + 漂移时的降级行为,Q3 关闭的落盘);
   - *Notes*:版本真源策略一句(tauri.conf.json 唯一真源,tag 时三处一致,CI 强制)。
2. **embedBootstrapper 语义勘误**(审查 P2-3,已回写 spec §2.3/§5):官方 schema 原文——*"Embed the bootstrapper and run it. **Requires an internet connection.** Increases the installer size by around 1.8MB"*。原稿"离线可用"为事实错误;真离线仅 offlineInstaller(+约 127MB),未选。README 按"装运行时仍需联网"如实描述并注明 +~2 MB 体积原因。社区若出现真实离线装机需求,一行配置可翻。
3. **版本真源**:tauri.conf.json `version` 为真源;`Cargo.toml`/`package.json` 随动;`check-versions.mjs` 在 release/PR 构建前置强制(三处一致 + tag 匹配;正则锚定 `[package]` 段,审查 P3-10)。tag `v0.1.0` 与 fixture `v0.1.1` 两次真实构建均先过守卫。

## 体积实测

NSIS 4.35 MB / MSI 5.59 MB(embedBootstrapper 内嵌引导器,+~1.8 MB 属实)。

## 已知缺口

- 干净虚机 README-only 验收未做(用户侧资源,Phase 2 门禁开放中);
- Q1 代码签名 / Q4 winget 仍开放(SmartScreen 摩擦由 README 指引缓解)。

## 环境还原

见 [S3 记录](verification-2026-09-05-S3.md)环境还原节(同批真机会话)。

## 一句反思

spec 的技术性断言("离线可用")在落地前对一次官方 schema,比落地后被用户戳穿便宜得多——本次勘误花了 5 分钟,若带错发布就是社区信任成本。

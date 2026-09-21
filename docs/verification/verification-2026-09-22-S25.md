# S25 GUI 页可见三键（窗口控制在远端页面）— 2026-09-22

- 验证人：agent（ZCode 会话）；用户拍板（"拍板了 GUI 页加可见三键"，含红线豁免扩宽）
- 被验构建：本地 `pnpm tauri build` NSIS 0.3.3（= 发布候选，签名链同 S24）
- 结论：**全项通过**（三键功能 + 拖拽/双击/右键回归 + 视觉验收 + 独立审查修复后复验）；过程抓到并修复 **SPA 拔条带**问题（S21 的拖拽条同样受害，一并修复）

## 0. 设计（红线的二次扩宽，用户拍板）

S21 的 §5 豁免原为"上游 GUI 页仅一条透明 drag div"。S25 扩宽为：**一条 drag strip + 其右端三个窗口控制按钮（min/max/close），别无一物**。按钮经 Tauri IPC 调既有三命令（`window_minimize`/`window_toggle_maximize`/`window_hide`，close=hide_to_tray 与原生 X/CloseRequested 同契约，S19 exactly-once 提示共享）。

IPC 授权（Tauri 2 ACL，源码级确认 + 实测）：
- 远端 origin 的应用命令**必须有显式 ACL**（tauri webview/mod.rs:1823：`!is_local` 即走闸门；本地页的应用命令无 ACL 也放行——这就是自家页一直好使的原因）
- `src-tauri/permissions/gui-titlebar.toml`：应用级权限 `allow-gui-titlebar-controls`，恰含三命令
- `src-tauri/capabilities/remote-gui-titlebar.json`：`windows:["main"]` + `remote.urls:["http://127.0.0.1:*"]`（端口 OS 随机 → URLPattern port 通配；**残余风险已注明**：主窗口若导航到任何本机 127.0.0.1 页面即继承这三个纯扰级命令——无参数无数据无执行）

## 1. 过程缺陷：React SPA 拔条带（S21 拖拽条同病，一并修复）

- 症状链：首轮真机验证三键+拖拽**全灭**；hover 探针证明条带曾活着（mouseenter 变红）；数分钟后按钮**彻底消失**（截图+视觉确认）。
- 根因：GUI 是 React SPA，渲染会整批替换 body 子节点，注入的条带被连根拔掉（点击落在空处）。S21 的纯拖拽条**同样会被拔**，只是此前未暴露。
- 修复：注入后挂 **MutationObserver**（观察 `document.documentElement` + `subtree`——连整个 body 被替换都能自愈；回调先以 `removedNodes` 预过滤再做 `getElementById` 检查后重挂）+ **window 标志防 observer 叠加**（同文档重复注入直接 no-op，自愈归 observer 管）。
- 验证：等 SPA 折腾期（20s+）后测全绿；连续两轮 4/4。

## 2. 通过项

| 项 | 方法 | 结果 |
|---|---|---|
| GUI 页三键功能 | 坐标点击（R-137/R-68/R-27, T+20）：min→iconic / max→2578x1420 / close→隐藏到托盘 | **PASS**（两轮；一轮 MIN 偶发失手系首启焦点未稳，复跑即过） |
| 拖拽回归（条带中部） | 按拖 (110,60) | **PASS** 精确 |
| 双击最大化 / 右键系统菜单 | 既有 S21 脚本 | **PASS** |
| 视觉验收 | 右上角 3× 放大截图 + 视觉复核：三键读作独立控件簇（半透明底常驻、图标清晰、无伪影） | **PASS**（用户初报的"重叠"乱象 = 条带被拔后的残局 + 无底透明叠标签，稳定后消失） |
| 降级路径 | invoke 首次 reject → console.error + 移除三按钮退化为纯拖拽条（防死按钮） | 代码级（审查修复项；触发需 ACL 漂移，正常路径不可达） |
| 测试锚定 | 注入测试扩（三键/no-drag/invoke/observer/degrade 字串）+ 新增 ACL 文件锚定测试（`include_str!` 三文件命令名互锁，防改名静默死键） | **25 测试全绿** |
| 版本一致性 | 三处 0.3.3（审查逮出 package.json 漏暂存，已补） | check-versions 过 |

## 3. 独立审查（无作者上下文子代理）

判决 FIX FIRST，全部落实：P2×3（invoke 静默失败→可见化+真降级、package.json 入暂存、交付文档）+ P3×4（observer 防叠加、documentElement+subtree 抗 body 替换、ACL 文件测试锚定、能力描述补残余风险注记）。审查另确认：format! 转义生成的 JS 经 `node --check` 合法、观察器无重入/放大风险、无跨导航泄漏、端口通配为正解（钉死端口不可行——`--port 0` 是本机负载性配置）。

## 4. 仪器与环境

脚本沿用 `D:\tmp\s21-verify\`（s25-buttons 三键+拖拽、hover-test、click-verdict、dblclick-menu）+ `s25-final-visual.png`/`topright-buttons.png` 留证。注入节点稳定性诊断法沉淀：**hover 变色探针**（mouseenter 是纯 DOM 事件，能把"z-order/位置问题"与"节点被移除"二分）。

## 5. 未覆盖

- 降级路径与 ACL 漂移场景未实机触发（需人为改坏 ACL 才能触发，代码级审查确认）。
- 暗色 GUI 下的按钮配色（#808080 中性灰 + 半透明底）未逐色验收——本机 GUI 当前为亮色；暗色可读性属低风险（中性灰双底通用）。

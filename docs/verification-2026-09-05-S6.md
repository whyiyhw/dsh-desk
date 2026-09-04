# S6 交付自验记录(2026-09-05 深夜)

> 对象:S6 · 品牌图标(规范见 [spec-and-plan.md](spec-and-plan.md) §2.3)
> 产出:`design/app-icon.svg` 源图 + `src-tauri/icons/` 全套 16 文件(ico/icns/各尺寸 png/Appx 磁贴/StoreLogo)
> 生成方式:cursor-agent CLI(composer-2.5)按 [design/S6-icon-brief.md](../design/S6-icon-brief.md) 执行,经三轮修订;最终方案 = opentype.js 提取 Segoe UI Bold "dsh" 轮廓 + 渐变瓦片
> 构建:`src-tauri/target/release/dsh-desk.exe`(工作树在途状态;`pnpm tauri build --no-bundle` exit 0)

## S6 验收对照(规范 §2.3)

| 验收条目 | 怎么测 | 结果 |
|---|---|---|
| `pnpm tauri build` 无图标报错 | `pnpm tauri build --no-bundle`(共跑两次:首建暴露陈旧资源缓存,`cargo clean --release -p dsh-desk` 后重建) | 两次 exit 0;最终 `Finished release [optimized] in 2m 37s`,产出 dsh-desk.exe ✓ |
| 托盘出现新图标 | 从编译产物抽取内嵌图标目视:`[System.Drawing.Icon]::ExtractAssociatedIcon(exe)` → 32×32 png | 内嵌图标 = 新 dsh 标,32px 清晰可读 ✓(托盘图标与 exe 资源同源;真机托盘**实时目视**未做,见已知边界) |
| 安装后开始菜单新图标 | 未做(依赖 S3 安装包) | Appx 磁贴(Square*/StoreLogo)已按新源图生成;NSIS 装包后开始菜单图标与 exe 同源 |

## 产物与证据

- `icon.ico` 内嵌 **6 尺寸**(16/24/32/48/64/256,含托盘 16px)——node `readUInt16LE` 逐项解析 + GNU `file` 双重确认
- `tauri.conf.json` `bundle.icon` 引用的 5 个路径全部存在且为新产物
- 源图:1024×1024 纯矢量;Segoe UI Bold fontSize 456(词宽 715px,视觉中心对齐 512,512);单条字体轮廓 path,**无 `<text>`、无外部引用**;SVG 注释记录字体/尺寸/bbox
- 视觉验收:128px、32px(png 直读)与 exe 抽取 32px 三个口径全部目视通过
- 改动面:`git status` 仅 `design/**` + `src-tauri/icons/**`(新增 64x64.png);lib.rs/README 等在途改动与本项无关,未触碰

## 方法备忘(重要坑,后续 S 项必读)

1. **盲画字形不收敛**:代码型 agent 手绘字母贝塞尔路径(没有渲染反馈),v1 读作 "D21"、R1 读作 "bsP"——**两轮 agent 自报成功均被视觉复核推翻**。改用字体轮廓(opentype.js `font.getPath("dsh",…).toPathData()`)后一次收敛。教训:**视觉类交付必须有"能看见的复核者"逐轮目检,agent 自报不作数**。
2. **embed-resource 陈旧 `.res` 缓存**:替换 `src-tauri/icons/*` 后 `pnpm tauri build` 照样成功,但 exe 仍嵌**旧图标**——`.rc` 文本不变时 embed-resource 复用旧 `.res`,不感知 ico 内容变化。`cargo clean --release -p dsh-desk` 后重建即愈(清 449MB,依赖缓存 1.1GB 不受影响)。判定手段 = ExtractAssociatedIcon 抽 exe 验,**别只看构建 exit 0**。
3. `pnpm tauri icon <svg>` 直接吃 SVG(内置 resvg),默认**额外产出 `ios/`、`android/` 子目录**——本项目 Windows 优先已删;下次重新生成会再出现,记得再删。
4. cursor-agent CLI 环境事实:官方二进制装在 `%LOCALAPPDATA%\cursor-agent`(npm 的 `cursor-agent` 包是同名第三方库,`bin` 为空,勿装);无头用法 `-p "…" --force --trust --output-format stream-json`;CLI 会加载 `~/.cursor/mcp.json` 的 MCP,`ones`(mcp-remote OAuth)会**卡死无头会话**——已在 CLI 侧 `mcp disable zai-mcp-server` 与 `mcp disable ones`(不影响 IDE,恢复用 `cursor-agent mcp enable <名>`);API 偶发长时间挂起,用 stream-json 事件 + 盘上文件时间戳双信号判断进度,别只看进程 CPU(主进程 CPU 常年接近 0)。

## 已知边界(不阻塞 S6 机器侧验收)

1. 真机托盘**实时目视**、安装后开始菜单目视未做:托盘需 GUI 真机跑(可沿用 S1 的程序化托盘驱动脚本截图取证);开始菜单依赖 S3 安装包落地。
2. `ios/`、`android/` 目录每次 `tauri icon` 重新生成(已删,提交前确认不存在)。

## 环境清点

- `%TEMP%\dsh-icon-tools`(opentype.js 转换脚本工作目录):已删除
- 测试期清理的进程:cursor-agent node 树 ×N、遗留 npx MCP 孤儿 ×4;**用户运行中的 debug 版 dsh-desk 实例与 3080 dsh dev 实例未动**
- 误装后卸载:npm 全局 `cursor-agent`(第三方同名库)已卸载;官方 CLI 安装位置见上

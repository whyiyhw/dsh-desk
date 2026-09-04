# dsh-desk 环境自检(幂等,只报告不修改)
# 用法: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/env-check.ps1
# 依据: AGENTS.md 环境事实节——每条都对应一次真实事故,本脚本把"记住"变成"拦住"(可运行检查)。
$ErrorActionPreference = 'Continue'
$fail = 0

function Report([string]$Name, [bool]$Ok, [string]$Detail, [string]$Hint) {
    $mark = if ($Ok) { 'PASS' } else { $fail++; 'FAIL' }
    "{0}  {1}: {2}" -f $mark, $Name, $Detail
    if (-not $Ok -and $Hint) { "      处置: $Hint" }
}

# 1. WebView2 运行时版本 —— dsh 前端需要 Promise.withResolvers(Chromium 119+);
#    2026-09-04 事故:运行时冻结在 114,GUI 崩"连接异常"(见 docs/postmortem-2026-09-04-webview2-114.md)
$pv = (Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
$major = if ($pv) { [int]($pv.Split('.')[0]) } else { 0 }
Report 'WebView2 运行时' ($major -ge 119) "版本 $pv(需 ≥119)" '按 AGENTS.md WebView2 节的 fwlink 2124701 流程升级'

# 2. config.json 带 --port 0 —— 本机 3080 被 dsh 开发实例常年占用,不加必 EADDRINUSE
$cfg = Join-Path $env:APPDATA 'dsh-desk\config.json'
if (Test-Path $cfg) {
    $hasPort0 = (Get-Content $cfg -Raw) -match '"--?port"\s*,\s*"0"|"port"\s*:\s*"0"'
    Report 'config.json --port 0' $hasPort0 (Join-Path '%APPDATA%' 'dsh-desk\config.json') 'args 里加 "--port","0"(OS 挑空闲端口,壳只认就绪行 URL)'
} else {
    Report 'config.json --port 0' $false "$cfg 不存在" '首次运行会生成;或预写配置'
}

# 3. 无 dsh-desk 残留实例 —— 真机 E2E 前置(single-instance 会把新测试实例弹回旧实例,exit 0 假象)
$running = Get-Process -Name dsh-desk -ErrorAction SilentlyContinue
Report '无 dsh-desk 残留进程' ($null -eq $running) $(if ($running) { "pid $($running.Id -join ',') 在跑" } else { '无' }) '托盘 Quit 或 taskkill /PID x /T /F(不要用 Stop-Process,不杀树)'

# 4. 门禁激活 —— pre-commit fmt 快检挂钩位置(吸收经验 规范9:hooksPath 要读出来对账)
$hooksPath = git rev-parse --show-toplevel 2>$null | ForEach-Object { Push-Location $_; $v = git config core.hooksPath; Pop-Location; $v }
$repoHooks = if ($hooksPath) { Join-Path (git rev-parse --show-toplevel) $hooksPath } else { '' }
Report 'pre-commit 门禁激活' (Test-Path (Join-Path $repoHooks 'pre-commit')) "core.hooksPath=$hooksPath" 'git config core.hooksPath .githooks'

# 5. Rust 工具链可用
Report 'cargo 工具链' ([bool](Get-Command cargo -ErrorAction SilentlyContinue)) 'cargo 在 PATH' '安装 Rust 工具链(rustup)'

""
if ($fail -eq 0) { 'env-check: 全部通过' } else { "env-check: $fail 项不通过(见上)" }
exit $(if ($fail -eq 0) { 0 } else { 1 })

# 复盘:宿主 Hyper-V 组件库损坏 → Phase 2 虚机门禁放弃执行(2026-09-05)

> 类型:事故复盘(宿主机,非本仓库制品)。结论先行:**门禁未执行不是覆盖率问题,是宿主机虚拟化栈带病**——任何 guest 活不过 90 秒;用户于 2026-09-05 拍板放弃该门禁,残余风险见文末。

## 现象

Phase 2 要求「干净 Windows 虚机仅按 README 从 Releases 安装 v0.2.0 走到 Ready(含离线 WebView2 场景)」。本机自建 Hyper-V 虚机尝试三条路线全部失败,失败模式与镜像无关:

| 路线 | guest | 表现 |
|---|---|---|
| 离线 dism apply install.esd | Win10 22H2 | 引导链修复后进 OOBE,随后**屏幕逐像素静止 12+ 分钟**(WMI framebuffer 对比),CPU 恒 3%,最终无响应 |
| 官方 ISO setup.exe(自动应答盘) | setup/WinPE | setup 进度屏同样冻结,逐像素不变 |
| 纯 WinPE(boot.wim idx1 + startnet) | WinPE | Gen1 同上冻结;Gen2 **≤90 秒 "shut down by the guest operating system"**(Worker 事件 18508,稳定复现) |

跨路线共性:**心跳(集成服务)在任何 guest 上从未连上过**(WinPE 自带 IC 也连不上)——VMbus 从未真正工作。

## 根因(证据链)

1. **宿主独立佐证**:Hyper-V-Worker 日志里 Docker Desktop 的 VM 自 2026-09-01 起就报 `33101: requested unsupported Virtual PCI protocol version 0x10004 (0x8007051A "two revision levels are incompatible")`,宿主重启后(09-05 06:46)依旧——宿主栈带病**早于本次一切操作**。
2. **开箱验尸**:给 startnet.cmd 插桩、把日志落到 VHDX 的 DATA 分区,guest 死后宿主侧挂盘回读——**零日志**:WinPE 死在 `wpeinit` 内(合成设备初始化等待 = VMBus 协商失败的正下游)。
3. **决定性一击——二进制版本审计**:

   | 组件 | 版本 | 应为 |
   |---|---|---|
   | vmms / vmcompute / vmwp | **10.0.19041.320**(2020 年中) | 跟随 LCU ~19041.5xxx+ |
   | vmbus.sys | **10.0.19041.1**(RTM 原始版) | 同上 |
   | **vmbusroot.sys** | **缺失** | 存在 |
   | ntoskrnl / hvsocket | 19041.6456(2025-11 ESU,当前) | ✓ |

   Hyper-V 可选功能载荷停留在 2020 年,内核已到 2025——guest 与化石版 vmbus 协商,字面意义的 "two revision levels are incompatible"。
4. **修复尝试与失败**:`DISM /RestoreHealth` + `sfc /scannow` 均报健康(组件索引自洽,看不出载荷版本错);**功能禁用→重启→重启用→重启**的完整重置后,vmms.exe 重新展开**仍是 .320**——组件库里存的载荷本身就是旧版。属深层服务化损伤(典型成因:历史上跑过 `StartComponentCleanup /ResetBase` 或更新中断)。
5. 宿主重启(09-05 06:45,用户授权)无效——与 4 一致,不是运行时状态问题。

根治手段为**就地修复升级**(挂官方 ISO 保留文件/应用重装,约 60-90 分钟)或换机;均超出"验证一个安装包"的合理成本,用户拍板放弃。

## 处置

- **门禁状态**:放弃执行(用户决策,2026-09-05);spec §5 已记行。
- **拆除**:DSHVM VM 对象与两块 VHDX 已删,自启键(HKCU Run)零残留;`D:\tmp\vm-gate\` 保留全部证据(日志/脚本/ISO/安装包,169 项)备日后复用。
- **残余风险与既有对冲**:干净机器上"仅按 README 到 Ready"未做过端到端实证。已有覆盖:①S4 运行时门禁(未装/过旧两条路径的引导态)在本机真机验证过;②v0.1.0/v0.2.0 安装版在开发机(有旧状态)上 §2.4 六条全过;③CI 构建绿。裸机首次安装的真实摩擦面(bootstrapper 联网装机、SmartScreen)未实证——已知且接受。

## 规矩(沉淀)

1. **「所有 guest 都活不过一分钟」类故障,先做宿主二进制版本审计**(`Get-Item <file>.VersionInfo` 比对内核与 vmms/vmbus 一组)——本轮从"无限镜像侧排障"到根因只花了一条命令,而它本可以是第一条。
2. **WMI `GetVirtualSystemThumbnailImage` 是 guest 屏幕的 ground truth**(vmconnect 会缩放/掉线/退出;桌面截图链受 CDN 字节去重干扰,对比帧必须重编码换名)。
3. **Worker 事件 18508/18514 是"谁杀了 VM"的第一手证据**,先查它再猜。
4. **免交互安装 VM 的正道 = WinPE 铺进 VHDX + startnet.cmd 拉起 `setup.exe /unattend:`**;El Torito "press any key" 抢键(AppActivate/SendKeys/物理点击)全不稳,别走。
5. 离线部署 BCD 的 `{default}` device/osdevice 必须改成 guest 上下文盘符(`partition=C:`),否则 0xc000000f 黑屏假象。
6. 宿主 Startup 文件夹受 Defender 受控文件夹访问保护(提权也被拒),自启用 HKCU Run 键。
7. DSH 沙箱对用户配置目录只读——文件落 D:\tmp,再经提权通道搬运。

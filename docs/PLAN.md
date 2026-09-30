# MetaClean 项目方案与工程路线图

> 本文是当前代码库的事实入口，不是早期立项草案。任何“已完成”都必须能在源码、测试、构建产物或运行证据中找到对应证据。

## 1. 当前状态

| 项目 | 当前事实 |
| --- | --- |
| 产品 | MetaClean：本地文件隐私清理桌面应用 |
| 技术栈 | Rust 清理内核 + Tauri 2 + React 19 + Vite + Tailwind CSS 4 |
| 当前包版本 | 0.11.9 |
| 当前本地提交 | 以 `git log -1` 为准；v0.11.9 候选版补齐 PNG 内嵌修改时间清理，正在进行发布前验证 |
| 发布状态 | v0.11.8 已公开发布，五平台构建、公开包签名与升级检查通过；内存实测与崩溃临时文件边界见 VALIDATION.md |
| 支持范围 | 116 个扩展名，Rust、前端、Windows shell、NSIS、MSI、README 与支持策略由门禁保持一致 |
| 隐私边界 | 文件内容只在本机处理；界面、历史、剪贴板和审计导出不展示原始元数据值 |
| 主要未闭环 | Word/新版 WPS 复杂文档兼容性、Apple 签名/公证、安装中断恢复及任意历史数据迁移、慢盘与完整应用峰值内存；已验证的更新与回退范围见 VALIDATION.md |

## 2. 产品边界

MetaClean 是“分享前的文件隐私护栏”，不是取证查看器，也不是 AI 检测绕过器。

### 做什么

- 在本地扫描并清理图片、音频、视频容器、Office/OpenDocument、EPUB、PDF、UTF-8 与带 BOM 的 UTF-16 文本、标记和常见配置文件中的隐私痕迹。
- 扫描只读；用户确认后才生成安全副本或替换原文件。
- 对候选字节重新识别和复检，确认清理结果后才分配输出、创建备份或替换源文件。
- 提供 Windows 资源管理器入口、批量队列、审计 JSON、历史记录、签名更新检查和 About 支持面板。

### 明确不做

- 统计型文本水印或需要语言模型重写的内容。
- 像素域可见水印移除。
- 旧版二进制 Office（.doc、.xls、.ppt）。
- 未识别的二进制格式或无法证明偏移安全的容器。

拒绝不是失败体验，而是避免把用户原文件交给未经验证的通用重写器。

## 3. 已实现架构

- src：React 页面、交互、国际化和前端测试。
- src-tauri/src/engine.rs：格式识别、扫描、候选复检和清理编排。
- src-tauri/src/cleaners：按容器实现的原生解析与原位/结构性清理器。
- src-tauri/src/intake.rs：递归导入、符号链接/重解析点和预算限制。
- src-tauri/src/safe_io.rs：快照、竞态防护、原子写入、备份和扩展属性。
- src-tauri/src/shell_integration.rs：Windows 右键菜单与安装状态。
- scripts：格式清单、安全、发布和供应链门禁。
- e2e：已安装桌面 WebView 的真实交互检查。
- docs：项目事实、路线、设计和验证文档。

核心调用链保持单向：导入 → 受限快照 → 扫描 → 用户确认 → 内存候选 → 重新识别/复检 → 快照再次校验 → 安全副本或备份加原子替换 → 结果、历史和审计。

## 4. 已验证里程碑

| 里程碑 | 状态 | 证据与边界 |
| --- | --- | --- |
| M0：容器安全性 | 完成 | PDF 增量历史重写、TIFF/RAW、HEIF/AVIF/CR3、AVI、Matroska/WebM、ASF/WMV、WAV C2PA、ID3 前置 FLAC 等均有原生测试；不满足结构约束的输入失败关闭。 |
| M0：Office 包完整性 | 部分完成 | LibreOffice 26.2.5 样例可打开并导出；WPS 2019（11.8.6.11825）三个 OOXML 样例在 VML 修复下通过打开、保存与再次读取，修复已随 v0.11.1 发布。Word、新版 WPS 与复杂文档版式仍未验证。 |
| M1：桌面工作流 | 完成 | 固定 1180 × 720 工作区、264/64px 可折叠侧栏、批量导入、扫描确认、清理模式、历史、五个页面、菜单、命令面板、主题、托盘策略和本地状态栏。 |
| M2：桌面集成与支持面 | 完成 | Windows shell、About、运行时诊断、复制路径、JSON 审计导出、签名更新状态和 116 扩展名清单一致性门禁。 |
| M3：发布自动化 | v0.11.8 已验证 | CI、双语 release notes、五平台资产收集、签名 updater manifest、SHA-256 清单、安装包 smoke gate 和公开下载校验全部通过；外部资格限制仍单独记录。 |
| M4：持续强化 | 进行中 | 依赖审计、路径别名去重、焦点可访问性、批量计数进度和文档事实同步持续推进。 |

## 5. 当前能力清单

### 格式与清理

- 116 个入口扩展名：图片、23 种 RAW、音频、视频容器、Office/OpenDocument/EPUB、PDF、文本/标记/配置文件。
- Ogg Opus/Vorbis 支持跨页注释和连续音轨，保留分页、解码配置、编码音频和有效播放增益；十组独立 FFmpeg 样本验证清理前后 PCM 一致。
- 原位或格式自带 padding 策略优先，避免移动图像 strip、媒体 cue、RIFF index、EBML element 或 ISO item extent。
- 文本输入要求有效 UTF-8 或带 BOM 的 UTF-16 LE/BE；清理保留原编码、BOM 和换行，Unicode 隐形字符、Markdown/HTML/SVG 适用的生成器痕迹和有界嵌入图片均经过边界检查。
- ICC/sRGB、JPEG 方向、时间戳、macOS 扩展属性等保留策略独立可选。

### 安全与可靠性

- 单文件 256 MiB，Office 解压 512 MiB，嵌入图片 16 MiB，嵌套 SVG 四层，批量 10,000 项；IPC 单路径 32 KiB、原始路径批次 64 MiB 等硬预算。
- 拒绝符号链接、Windows 重解析点、未知二进制和结构不完整数据。
- 源文件字节、修改时间、只读权限和扩展属性在输出分配前及替换前都重新校验。
- 输出采用无覆盖唯一创建；替换模式先备份，再通过原子写入提交。
- Windows 普通、斜杠、设备前缀和 UNC 路径统一去重，避免同一文件重复处理。

### 桌面体验

- 工作区侧栏支持 264px 展开态和 64px 图标态，状态本地持久化，可通过标题栏、命令面板或 `Ctrl/Cmd+B` 切换。
- 标题栏搜索、系统窗口按钮、侧栏与主内容使用连续中性表面；深色、浅色和跟随系统主题共享 14px 基础字阶与语义状态色。
- 队列支持排序、复制路径、复制文件名、显示输出、详情展开、失败重试和审计 JSON 导出。
- 更新对话框与命令面板锁定键盘焦点，Escape 关闭后恢复触发控件焦点。
- 批量扫描与清理都只广播 operation/completed/total/failed/cancelled 计数，不广播路径或文件内容；每批次可在文件边界安全取消，已完成结果保留、未处理项可重试；只有清理进度会写入异常恢复标记。
- 32 个完整界面语言目录，含 RTL 布局；支持范围数字在各语言运行时保持当前事实。

## 6. 后续路线（按证据优先级）

### P0：尚未闭环的外部资格与持续发布门禁

历史路线图曾将外部应用资格列为发布前条件。v0.11.1 已在明确披露限制的情况下发布；发布成功不代表 Word/新版 WPS 或 Apple 公证等未验证项已经通过。这些缺口继续保留，不以自动化测试代替外部资格证明。

1. 补真实 Microsoft Word 与 WPS 新版的复杂对象/版式验证。已在本机自定义安装目录找到 WPS 2019，并完成三个 OOXML 样例的语义往返；其中发现的 XLSX 空批注残留已修复并随 v0.11.1 发布，详见 VALIDATION.md。
2. 持续验证安装、更新和恢复。v0.11.4 的适用安装包 smoke checks、公开下载包签名、篡改拒绝和五平台更新源已核验。Windows x64/x86 NSIS 已实测 0.11.3 → 0.11.4 的应用内签名更新、自动重启、合成设置/历史保留与手动降级；截断安装包拒绝后旧版仍可用。MSI（含损坏程序修复）、Linux DEB 与 macOS 双架构 DMG 也已验证该版本对的手动替换与回退。Windows x86 和 macOS Intel 包在兼容环境运行；安装中断恢复、任意历史数据迁移及其它更新路径仍待验证。
3. 在 CI 中保持官方 npm audit 与 cargo audit；Cargo 上游警告已在 VALIDATION.md 逐项分类。v0.11.4 将可升级的已撤回 `chacha20 0.10.1` 更新为 0.10.2；GLib 健全性问题受 Linux GTK 依赖版本约束，六条维护状态警告继续跟踪，不等同于无风险。

### P1：能力和性能

1. 建立真实大批量基准：不同文件大小、嵌套目录、慢盘和失败混合批次，记录吞吐、峰值内存、首个结果时间和尾部延迟。当前 release-only native benchmark 包含 128 个嵌套文本夹具（4 KiB/64 KiB/512 KiB 混合），以及 96 个有效、16 个不支持格式、16 个缺失路径的混合批次，输出 scan/clean 吞吐、首个结果与 p95。Windows 六次独立原生进程测量观察到的最大工作集峰值为 14.63 MiB，不含 WebView。另以公开 v0.11.3 完整桌面进程树各测三轮 64/256 MiB 文本；聚合工作集采样最大值分别为 503.79–571.55 MiB、887.26–1,243.54 MiB，全部通过完整性验证。共享页可能重复计数，且采样不保证捕获瞬时峰值；慢盘、复杂文档和其它系统仍待验证，详见 VALIDATION.md。
2. 评估有限并发和背压；只有在不破坏源快照、写入顺序和隐私事件边界时才启用并发。扫描最多使用两个动态取件 worker，不均匀任务完成后仍按输入顺序返回；目录展开与扫描共享 read-task close guard。扫描与清理都能在文件边界取消，扫描通过 `cancel_scan_batch` 只返回已完成报告，清理仍保持顺序执行。取消延迟和真实慢盘背压仍需专门机器夹具。
3. 为文本/配置扩展建立独立的编码、超长行、BOM、换行、只读和恶意嵌套样例矩阵。UTF-8（含/不含 BOM）与 BOM 标记的 UTF-16 LE/BE、CRLF 保留、无效代理项、1 MiB 单行输入和超过递归预算的 SVG 已覆盖；零宽字符先规范化再匹配 HTML 结构化元数据；Windows 只读源文件复制安全、替换失败关闭。隔离 Linux 运行 36493274324 已验证真实 tmpfs 空间耗尽与只读挂载：复制和替换均安全失败，源文件不变且无残留。物理可移动介质、慢盘及断电仍未验证，详见 VALIDATION.md。
4. 对 PDF、Office、HEIF、RAW 增加跨工具 round-trip 夹具，优先补真实失败样本，而不是盲目增加格式数量。PDF 已新增六组矢量、表单和内嵌 JPEG 的复制/替换保真样本，通过独立 MuPDF/Poppler 渲染及字段、像素、元数据核对。HEIC/AVIF 新增六组单图、多图和透明图样本，本地独立解码的像素、旋转显示、ICC、透明度与元数据检查通过；托管环境结果见 VALIDATION.md。复杂表单、XFA、签名文档、HDR/深度图及 RAW 跨工具资格仍待验证。

### P1：桌面成熟度

1. 补齐崩溃恢复和中断批次的明确 UX；清理取消已经在文件边界实现，活动扫描/清理任务会阻止窗口关闭。公开 v0.11.2 的强制退出测试发现 WebView 计数标记不能可靠提供重启提示。v0.11.3 增加清理前落盘的无路径原生记录，以文件锁区分活动任务，正常结束删除、异常退出后仅消费一次提示，不自动恢复文件操作。候选运行 36486936517 和公开包运行 36490451100 均通过 64 文件真实中断、完整性和两次重启检查；断电与安装中断恢复仍未验证。
2. 将安装包、更新源、签名、回滚和失败恢复做成可重复的验证流程。Windows 候选可运行 `scripts/preflight-windows.ps1` 完成 debug NSIS、MSI、x64 便携包烟测；`verify-upgrade.yml`、`verify-msi-upgrade.yml`、`verify-linux-upgrade.yml` 和 `verify-macos-upgrade.yml` 使用一次性运行器验证公开旧版与新版转换。MSI 的 0.11.0 → 0.11.1 → 0.11.0 登记、版本和启动检查也已通过。现有证据限于上述版本对与合成数据，不能代替安装中断恢复或任意用户环境资格。
3. 将 About、诊断、隐私政策、支持策略和 release notes 统一从同一份事实清单生成，减少多语言和版本漂移。

### P2：文档与生态

1. 以 docs/README.md 作为文档导航入口，并用 VitePress 提供可搜索的任务型站点；每份文档标明适用版本、证据、未验证边界和责任人。Windows 队列路径身份已与 Rust 端统一，大小写、斜杠、设备前缀和 UNC 别名不会再制造重复队列或“未返回结果”。
2. 按“用户指南 / 安全模型 / 格式策略 / 贡献与发布 / 设计系统”分层，避免把早期市场调研和当前运行事实混在一起。
3. 公开能力说明只保留可复查的本项目源码、测试或构建证据，不使用未经验证的普遍优势表述。

## 7. 质量门禁

提交前至少运行：

- pnpm test
- pnpm test:coverage
- pnpm test:formats
- pnpm test:docs
- pnpm test:security
- pnpm test:supply-chain
- pnpm test:release
- pnpm docs:build
- pnpm audit --audit-level moderate --registry=https://registry.npmjs.org
- pnpm build
- cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
- cargo test --manifest-path src-tauri/Cargo.toml
- cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
- cargo audit -f src-tauri/Cargo.lock --no-fetch
- `pnpm test:e2e`（命令会先构建带 WebDriver 插件的 E2E 二进制）

cargo audit --no-fetch 只表示使用本机已缓存的 advisory 数据；若 fetch 失败，必须在报告中说明网络限制，不能把它当作新鲜数据库证明。

## 8. 来源与事实约束

- 当前格式清单的权威源是 src-tauri/src/engine.rs；其它清单由 pnpm test:formats 校验。
- 当前自动化证据集中在 VALIDATION.md；历史 release notes 不得反向描述当前工作树。
- 产品安全边界见 SUPPORT_POLICY.md 和 SECURITY.md。
- 视觉与交互原则见 DESIGN.md；参考站点只提供布局和信息层级启发，不复制第三方资产。
- 能力验收见 CAPABILITY_AUDIT.md，每项能力必须指出实现、证据和未覆盖范围。

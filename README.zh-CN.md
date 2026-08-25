<div align="center">

<img src="assets/metaclean-icon.svg" alt="MetaClean" width="88" height="88">

# MetaClean

**分享文件之前，先清掉里面的隐私痕迹。**

文件纯本地处理 · 应用内签名更新 · 无需 ExifTool / Python / Perl · Rust 内核

[![CI](https://img.shields.io/github/actions/workflow/status/Moresyl/metaclean/ci.yml?branch=master&style=flat-square&label=CI)](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Moresyl/metaclean?include_prereleases&style=flat-square&color=35966d)](https://github.com/Moresyl/metaclean/releases)
[![License](https://img.shields.io/badge/license-MIT-35966d?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-35966d?style=flat-square)](https://github.com/Moresyl/metaclean/releases/latest)
[![Core](https://img.shields.io/badge/core-Rust-35966d?style=flat-square)](src-tauri)

[English](README.md) · **简体中文**

<img src="assets/metaclean-screenshot.png" alt="MetaClean 桌面界面" width="820">

</div>

---

照片里藏着 GPS 坐标。Word 文档里留着你的姓名、单位，以及那些你以为已经删干净的修订记录。PDF 会把每一版旧草稿的元数据一并带上。从大模型里复制出来的文字，则夹带着看不见的 Unicode 字符。

MetaClean 把这些统统找出来并清除——全过程只在你自己的电脑上完成。

## 为什么用它

- **文件绝不出本机。** 没有上传接口、遥测或云端处理。可选的签名更新检查只请求 MetaClean 官方 GitHub 托管更新源，并且可以关闭。
- **不用先装一堆东西。** 单个可执行文件，无需 Python、Perl、ExifTool 或任何运行时。
- **先扫描，再决定。** 扫描是只读操作，逐个文件列出检测结果，确认之后才会写入。
- **默认就是安全的。** 替换原文件前强制备份，写入为原子操作，默认模式只生成 `.cleaned` 副本而不覆盖原件。
- **能力边界写在明处。** 无法安全处理的格式直接拒绝，绝不假装处理过。

## 下载

前往 [GitHub Releases](https://github.com/Moresyl/metaclean/releases/latest) 获取最新安装包。

| 平台 | 安装包 |
| --- | --- |
| Windows | x64 `.exe`（NSIS）/`.msi`/便携 ZIP · x86 `.exe`（NSIS）/便携 ZIP |
| macOS | `.dmg` —— Apple Silicon 与 Intel 双架构 |
| Linux | `.deb` · `.rpm` · `.AppImage` |

## 清理范围

91 种扩展名，全部由原生 Rust 代码处理——不依赖 ExifTool，也不会重新编码。

| 格式 | 扩展名 | 清理内容 |
| --- | --- | --- |
| JPEG | `.jpg` `.jpeg` `.jpe` | EXIF/GPS、XMP、IPTC、图片注释、JUMBF/C2PA 段；ICC 默认保留，也可明确移除 |
| PNG | `.png` | EXIF、文本元数据、C2PA/JUMBF 块；可选移除 ICC 配置 |
| WebP | `.webp` | EXIF、XMP、C2PA 块；可选移除 ICC 配置 |
| GIF | `.gif` | 注释与 XMP 应用元数据，不重新编码动画帧 |
| BMP | `.bmp` `.dib` | 被编辑器拿来写编号的两个保留字段、V5 头里内嵌的 ICC 配置，以及贴在最后一个像素之后、任何看图软件都不会显示的 EXIF/XMP |
| TIFF | `.tif` `.tiff` | EXIF、GPS、IPTC、XMP 目录；采用原地压缩 IFD 的方式删除，条带、分块与预览图的偏移量全部保持有效 |
| 相机 RAW | `.cr2` `.cr3` `.crw` `.nef` `.nrw` `.arw` `.srf` `.sr2` `.orf` `.rw2` `.rwl` `.dng` `.pef` `.srw` `.raf` `.3fr` `.erf` `.mef` `.mos` `.iiq` `.kdc` `.dcr` `.k25` | 同样的原地目录压缩，另外处理 MakerNote 与 GPS 目录。富士的内嵌 JPEG 预览图、佳能 CR3 的条目数据都在原位清理，传感器数据从不重写 |
| HEIF 与 AVIF | `.heic` `.heif` `.heics` `.heifs` `.hif` `.avif` `.avifs` | 按条目粒度清空 EXIF、XMP、C2PA 条目，完整保留定位图像所需的条目表 |
| 音频 | `.mp3` `.wav` `.flac` | ID3/APEv2、RIFF INFO/XMP/BWF/iXML/C2PA、FLAC Vorbis 评论、封面、XMP 与前置 ID3/C2PA |
| ISO 媒体 | `.mp4` `.mov` `.m4v` `.m4a` `.3g2` `.3gp` `.3gp2` `.3gpp` `.f4a` `.f4b` `.f4p` `.f4v` `.lrv` `.m4b` `.m4p` `.mqv` `.qt` | ISO BMFF/QuickTime 用户数据、XMP、作者与位置原子，不移动媒体字节 |
| AVI | `.avi` | 元数据块被改名为 RIFF 自带的 `JUNK` 填充标记并清零，无论 `idx1` 索引采用哪种偏移基准都不会错位 |
| Matroska 与 WebM | `.mkv` `.mka` `.mks` `.mk3d` `.webm` | 标签、附件与写入程序信息，通过在原字节上覆写 EBML `Void` 元素来作废，索引表继续有效 |
| ASF | `.asf` `.wmv` `.wma` | 内容描述与整个 `WM/` 属性空间被格式自带的填充对象覆盖，头部对象计数保持真实 |
| 文档 | `.docx` `.xlsx` `.pptx` `.odt` `.epub` | 作者与应用属性、批注、自定义 XML；DOCX 修订会被固化——接受插入内容，移除删除标记内容。EPUB 会清掉 Dublin Core 中的人名与日期，以及 Calibre/Sigil/Kobo/Apple/Adobe 留下的痕迹 |
| PDF | `.pdf` | 移除 Info 字典、XMP 与内嵌 JPEG 图片中的元数据，再完整重序列化，丢弃残留在增量更新历史里的元数据 |
| 文本与标记 | `.txt` `.md` `.markdown` `.html` `.htm` `.xhtml` `.svg` `.xml` `.json` `.csv` `.tsv` `.yaml` `.yml` `.log` `.srt` `.vtt` | 不可见 Unicode、Markdown Front Matter、HTML/XHTML/SVG 的作者与生成器信息，以及内嵌 Data URI 图片中的元数据 |

上面每一种容器的字节偏移量都不会改变。凡是靠位置索引自身的文件，我们从不做删除——元数据要么被原地压缩，要么被清零，要么被该格式本身就定义好的填充元素覆盖。所以清理之后，一张 RAW 底片、一份 Matroska 索引表或一个 AVI 索引，和清理之前一样有效。

**明确不做的事：** 统计型文本水印、像素域水印、旧版二进制 Office 文件（`.doc` / `.xls` / `.ppt`）以及未知二进制格式。遇到这些，MetaClean 会直接拒绝，而不是冒险改坏你的文件。

## 安全保障

- 替换原文件前，一定先写入 `.bak` 备份
- 默认生成 `.cleaned` 安全副本
- 先写临时文件，再原子替换目标文件
- 创建备份或写入任何输出前，复检即将提交的精确清理后字节
- 输入和输出路径都拒绝符号链接
- 单文件上限 256 MiB，Office 解压总量上限 512 MiB
- 文件损坏或格式不支持时直接失败，绝不改动源文件

## 桌面端

- 支持批量拖入文件或文件夹，也可以用系统原生选择器递归导入目录
- 四个页面：**文件净化**、**处理记录**、**隐私说明**、**设置**
- 固定 1180 × 720 企业工作台，使用紧凑图标导航与持续可见的“仅本地”状态栏
- 可导出带版本的本地 JSON 审计报告，记录逐文件发现项与处理结果，但不包含原始元数据值
- 可选的 Windows 资源管理器右键菜单，覆盖全部 91 种受支持扩展名（Windows 11 上位于**显示更多选项**中）
- 关闭窗口默认彻底退出；也可在设置中改为驻留系统托盘，再从托盘菜单重新打开或退出
- 设置 → 系统与更新中提供项目主页与问题反馈入口
- 默认保留 JPEG 显示方向、ICC/sRGB 色彩配置与文件时间戳，三项均可独立关闭
- 安装版可在应用内检查、下载并安装通过签名验证的稳定版，GitHub Release 清单不可达时自动回退到官方 Pages 更新源；Windows 便携版与 Linux 非 AppImage 包会回退到官方 Releases 页面
- 启动更新检查可以单独关闭，恢复完全离线运行
- 32 种完整界面语言，覆盖欧洲、东亚与东南亚、南亚，以及阿拉伯语和波斯语的 RTL 布局；跟随系统/浅色/深色主题、输出方式、保真选项、本地处理记录都会持久保存

## 从源码构建

需要 [Rust](https://rustup.rs)、[Node.js](https://nodejs.org) 和 [pnpm](https://pnpm.io)，以及对应平台的 [Tauri 系统依赖](https://tauri.app/start/prerequisites/)。

```bash
pnpm install
pnpm tauri dev      # 以开发模式运行
```

提交 Pull Request 前请跑完整套检查：

```bash
pnpm test:coverage                              # 前端测试，覆盖率不低于 80%
pnpm test:formats                               # 扩展名清单一致性检查
pnpm test:security                              # 生产 WebView CSP 安全策略
pnpm test:release                               # 发行说明与校验和自动化
pnpm test:supply-chain                          # 已修补依赖的安全回归测试
pnpm build                                      # 类型检查 + 生产构建
cargo test --manifest-path src-tauri/Cargo.toml # Rust 内核测试
pnpm test:e2e:build && pnpm test:e2e             # 真实桌面程序 E2E
pnpm tauri build                                # 各平台安装包
```

每次分支构建还会在 Windows、macOS 和 Linux 启动 E2E 专用桌面二进制；内嵌 WebDriver 与测试命令受 Cargo feature 隔离，不会进入生产包。推送版本标签后，GitHub Actions 会构建完整发布矩阵：Windows x64 的 NSIS/MSI/便携 ZIP 与 x86 的 NSIS/便携 ZIP、macOS 的 Apple Silicon 与 Intel 双 DMG、Linux 的 DEB/RPM/AppImage。Release 还会生成五个平台目标的签名更新包与静态 `latest.json`，全部安装包冒烟通过并生成完整 SHA-256 清单后才公开发布。更新签名不等于操作系统代码签名；macOS 签名与公证仍需 Apple 凭据，未配置时 DMG 仍是未签名状态。

测试覆盖率与发布验收证据记录在 [VALIDATION.md](VALIDATION.md)。
版本变更记录在 [CHANGELOG.md](CHANGELOG.md)。
不显示元数据具体内容的设计取舍、各格式的清理策略，以及明确不做的边界，都写在 [SUPPORT_POLICY.md](SUPPORT_POLICY.md)。

## 参与贡献

欢迎提交 Pull Request，请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。安全相关问题请按 [SECURITY.md](SECURITY.md) 的流程，走私密漏洞报告渠道，不要开公开 issue。

## 责任使用

请只处理你拥有或已获授权的内容。MetaClean 是为隐私保护和文件卫生而做的——不是用来学术造假、伪造来源，或对文件出处作出误导性声明。

## 许可证

[MIT](LICENSE) © Moresyl

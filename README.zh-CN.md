# MetaClean

[English](README.md)

[![CI](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml/badge.svg)](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Moresyl/metaclean?include_prereleases)](https://github.com/Moresyl/metaclean/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/core-Rust-orange.svg)](src-tauri)

纯本地文件隐私净化器。扫描并清除文件中的 EXIF/GPS、Office 文档属性与修订、PDF 元数据，以及不可见 Unicode 字符。

## 软件界面

![MetaClean 本地文件隐私净化器桌面界面](assets/metaclean-screenshot.png)

## 下载

请从 [GitHub Releases](https://github.com/Moresyl/metaclean/releases/latest) 下载最新 Windows、macOS 或 Linux 安装包。无需 Python、Perl、ExifTool 或云端账号。

## 支持范围

- JPEG：EXIF、XMP、IPTC、图片注释、JUMBF/C2PA 段
- PNG：EXIF、文本元数据、C2PA/JUMBF 块
- WebP：EXIF、XMP、C2PA 块
- DOCX/XLSX/PPTX/ODT：作者和应用属性、批注、自定义 XML；DOCX 接受插入修订并删除已删内容
- PDF：移除 Info/XMP 后完整重序列化，丢弃增量更新中的旧元数据
- TXT/Markdown/HTML/SVG/XML/JSON/CSV：不可见 Unicode；Markdown、HTML、SVG 的作者或生成器元数据

统计型文本水印、像素域水印、视频、旧版二进制 Office 文件和未知二进制格式明确不在处理范围内。

## 桌面体验

- 支持批量拖拽和系统原生文件选择器
- 可为 18 类受支持扩展名启用 Windows 资源管理器菜单（Windows 11 位于“显示更多选项”中）
- 关闭主窗口后驻留系统托盘；右键托盘图标可重新打开或彻底退出
- 保留原生最小化/最大化按钮，并持久保存中英文、输出方式和本地处理记录

## 安全策略

- 单文件最大 256 MiB，Office 解压总量最大 512 MiB
- 拒绝处理或写入符号链接
- 输出先写同目录临时文件，再原子替换
- 替换原文件前强制创建 `.bak` 备份
- 默认生成 `.cleaned` 安全副本
- 没有上传接口、遥测或云端处理

## 开发

```powershell
pnpm install
pnpm tauri dev
```

```powershell
pnpm test
pnpm test:coverage
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

推送版本标签后，GitHub Actions 会生成 Windows NSIS/MSI、Apple Silicon 与 Intel 双架构 macOS DMG，以及 Linux DEB/RPM/AppImage。macOS 签名和公证需要配置发布工作流中列出的 Apple 密钥。

## 责任使用

仅处理你拥有或获授权处理的内容。本工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或误导性声明。

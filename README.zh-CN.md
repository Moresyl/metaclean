# MetaClean

[English](README.md)

纯本地文件隐私净化器。扫描并清除文件中的 EXIF/GPS、Office 文档属性与修订、PDF 元数据，以及不可见 Unicode 字符。

## 支持范围

- JPEG：EXIF、XMP、IPTC、图片注释、JUMBF/C2PA 段
- PNG：EXIF、文本元数据、C2PA/JUMBF 块
- WebP：EXIF、XMP、C2PA 块
- DOCX/XLSX/PPTX/ODT：作者和应用属性、批注、自定义 XML；DOCX 接受插入修订并删除已删内容
- PDF：移除 Info/XMP 后完整重序列化，丢弃增量更新中的旧元数据
- TXT/Markdown/HTML/SVG/XML/JSON/CSV：不可见 Unicode；Markdown、HTML、SVG 的作者或生成器元数据

统计型文本水印、像素域水印、视频、旧版二进制 Office 文件和未知二进制格式明确不在处理范围内。

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
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

## 责任使用

仅处理你拥有或获授权处理的内容。本工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或误导性声明。

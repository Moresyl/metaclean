---
title: 产品能力
description: MetaClean 的产品能力、安装方式和明确边界
---

# 产品能力

MetaClean 是本地优先的文件隐私清理桌面应用。它用 Rust 内核执行只读扫描、候选清理、格式重识别和残留复检，再由 Tauri 桌面界面提供批量队列、确认、历史和审计工作流。

## 用户能得到什么

- 图片、音视频容器、Office/OpenDocument/EPUB、PDF 与文本类文件的格式感知处理。
- 默认保留原文件的安全副本模式，以及先备份再原子替换的替换模式。
- 不展示原始元数据值的结果、历史与审计导出。
- Windows 资源管理器入口、跨平台桌面构建与签名更新检查。

## 产品承诺

所有文件内容都留在本机。损坏、未知、超预算或无法证明安全的输入会失败关闭；可见水印、统计型文本水印和旧版二进制 Office 不在支持范围内。

完整下载说明、格式数量和当前版本事实由仓库根目录的 [中文 README](https://github.com/Moresyl/metaclean/blob/master/README.zh-CN.md) 与 [English README](https://github.com/Moresyl/metaclean/blob/master/README.md) 维护。安全细节见 [支持策略](./support-policy)，实测状态见 [验证账本](./validation)。

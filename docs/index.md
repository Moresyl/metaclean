---
title: MetaClean 文档中心
description: 面向用户、贡献者和发布维护者的 MetaClean 事实文档
---

<div class="hero-grid">
  <div>
    <p class="eyebrow">LOCAL-FIRST · VERIFIED · FAIL-CLOSED</p>
    <h1>把每个文件交给分享前的安全护栏。</h1>
    <p class="lead">MetaClean 在本机扫描、复检并清理隐私痕迹。文档站只负责呈现和导航，实际能力、限制与验证结果仍以仓库源码和质量门禁为准。</p>
    <div class="hero-actions">
      <a class="vp-button brand" href="./user-guide">开始使用</a>
      <a class="vp-button" href="./ARCHITECTURE">阅读架构</a>
    </div>
  </div>
  <div class="hero-proof">
    <span class="proof-label">CURRENT SOURCE LINE</span>
    <strong>0.7.x</strong>
    <span>113 个受限格式入口 · 32 个完整界面语言 · 本地优先</span>
  </div>
</div>

## 按任务进入

<div class="card-grid">
  <a class="doc-card" href="./user-guide">
    <span class="card-kicker">用户</span>
    <h2>安全地清理文件</h2>
    <p>了解扫描、确认、安全副本、替换备份和失败重试。</p>
  </a>
  <a class="doc-card" href="./ARCHITECTURE">
    <span class="card-kicker">维护者</span>
    <h2>理解处理链</h2>
    <p>从受限导入到候选字节复检，查看 IPC、并发和写入不变量。</p>
  </a>
  <a class="doc-card" href="./PLAN">
    <span class="card-kicker">路线</span>
    <h2>查看真实完成度</h2>
    <p>区分源码完成、实机验证、外部依赖和仍未闭环的项目。</p>
  </a>
  <a class="doc-card" href="./release">
    <span class="card-kicker">发布</span>
    <h2>运行发布前门禁</h2>
    <p>构建、E2E、安装包 smoke、右键菜单和签名边界都有明确证据。</p>
  </a>
</div>

## 三条不变规则

<div class="rule-strip">
  <div><b>不上传</b><span>文件内容只留在本机进程。</span></div>
  <div><b>先复检</b><span>候选字节通过格式重识别和残留检查后才写盘。</span></div>
  <div><b>可恢复</b><span>替换前创建唯一备份，失败关闭而不是猜测。</span></div>
</div>

## 事实入口

- [产品能力](./product) · [用户指南](./user-guide) · [架构](./ARCHITECTURE)
- [验证账本](./validation) · [支持策略](./support-policy) · [安全报告](./security)
- [设计参考](./design) · [竞品审计](./competitive-audit) · [变更记录](./changelog)

> 文档站是展示层，不是第二个产品事实源。数字、完成状态和安全边界以源码、测试与 [验证账本](./validation) 的证据顺序为准。

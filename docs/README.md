# MetaClean 文档中心

> 适用代码线：`0.7.x` · 维护者：MetaClean maintainers · 原则：事实与计划分离

文档按读者要完成的任务组织。第一次了解项目从 README 开始；修改清理
内核先读安全策略和架构；准备发布时以验证账本为准。

## 快速入口

| 你要做什么 | 从这里开始 | 最终证据 |
| --- | --- | --- |
| 了解产品、下载或构建 | [`README.zh-CN.md`](../README.zh-CN.md) / [`README.md`](../README.md) | 当前用户能力与明确边界 |
| 修改架构、IPC 或状态流 | [`ARCHITECTURE.md`](ARCHITECTURE.md) | 模块责任、数据边界与不变量 |
| 新增或修改格式 | [`SUPPORT_POLICY.md`](../SUPPORT_POLICY.md) | 格式安全门槛与失败关闭策略 |
| 审核当前完成度 | [`VALIDATION.md`](../VALIDATION.md) | 实际测试、构建、E2E 和外部限制 |
| 安排下一阶段工作 | [`PLAN.md`](PLAN.md) | 当前事实、风险和证据优先级 |
| 准备贡献或安全报告 | [`CONTRIBUTING.md`](../CONTRIBUTING.md) / [`SECURITY.md`](../SECURITY.md) | 提交流程和私密漏洞渠道 |

## 完整地图

| 分类 | 文档 | 责任与用途 |
| --- | --- | --- |
| 产品 | [`README.zh-CN.md`](../README.zh-CN.md), [`README.md`](../README.md) | 面向用户的能力、安装方式、处理模型和边界 |
| 架构 | [`ARCHITECTURE.md`](ARCHITECTURE.md) | 处理链、模块所有权、IPC、持久化、并发和发布边界 |
| 安全 | [`SUPPORT_POLICY.md`](../SUPPORT_POLICY.md), [`SECURITY.md`](../SECURITY.md) | 格式准入、隐私模型、更新信任根和漏洞报告 |
| 证据 | [`VALIDATION.md`](../VALIDATION.md) | 当前代码线已经实际运行的门禁与尚未验证项 |
| 设计 | [`DESIGN.md`](../DESIGN.md) | 设计 tokens、信息层级、可访问性和禁止回退项 |
| 规划 | [`PLAN.md`](PLAN.md) | 已验证能力、开放风险和后续路线 |
| 对比 | [`COMPETITIVE_AUDIT.md`](../COMPETITIVE_AUDIT.md) | 带日期、提交和证据边界的竞品审计 |
| 历史 | [`CHANGELOG.md`](../CHANGELOG.md) | 已发布版本与未发布改动，不代替当前能力说明 |

## 文档规则

- 先写当前事实，再写计划；不要把未验证假设写成完成状态。
- 每个数字必须能追溯到源码或门禁输出。
- 历史版本可以保留历史数字，但必须放在对应版本章节，不得冒充当前能力。
- 涉及用户文件、更新签名、权限或格式解析的说明，必须同时写出拒绝边界。
- “跨平台支持”要区分代码支持、CI 验证、实机验证和签名/公证状态。
- 出现冲突时按“源码与测试 → SUPPORT_POLICY → VALIDATION → README → 对比/历史文档”的顺序校正，详见 [`ARCHITECTURE.md`](ARCHITECTURE.md)。

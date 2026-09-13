---
title: 贡献与发布
description: MetaClean 的本地质量门禁、Windows preflight 和发布边界
---

# 贡献与发布

## 本地检查

```powershell
pnpm test
pnpm test:coverage
pnpm test:formats
pnpm test:docs
pnpm test:release
pnpm test:security
pnpm test:supply-chain
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

## Windows 候选 preflight

这条命令只构建和验证本地候选，不会 push、tag 或创建发布：

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\preflight-windows.ps1
```

它会依次验证 Debug NSIS、MSI、x64 便携包和 113 个扩展名的当前用户右键菜单安装/清理。MSI 与右键菜单脚本都会拒绝覆盖已有 MetaClean 状态。

## 自动发布门禁

GitHub Release workflow 在任何平台打包前，会先 checkout 同一个
`RELEASE_TAG` 并运行完整的源码校验 job：供应链、CSP、发布脚本、格式清单、
文档构建、npm audit、前端覆盖率与生产构建、Linux Xvfb 下的桌面 E2E，以及 Rust
格式化、测试、覆盖率和 Cargo audit。只有这个 job 成功，Windows、macOS、Linux 的矩阵构建才会开始；
所有平台 smoke test 和资产校验完成后才允许生成公开 Release。

## 发布边界

正式发布还需要签名 updater 密钥、平台代码签名以及对应平台的真实运行证据。Apple Developer 签名/公证、Word/WPS 打开保存回读和旧版本升级回滚不能用本地源码构建替代。

质量证据以 [验证账本](./validation) 为准，具体路线以 [PLAN](./PLAN) 为准。

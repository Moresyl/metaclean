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

## 发布边界

正式发布还需要签名 updater 密钥、平台代码签名以及对应平台的真实运行证据。Apple Developer 签名/公证、Word/WPS 打开保存回读和旧版本升级回滚不能用本地源码构建替代。

质量证据以 [验证账本](./validation) 为准，具体路线以 [PLAN](./PLAN) 为准。

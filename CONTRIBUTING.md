# Contributing to MetaClean

Thanks for helping make private file sharing safer.

## Ground rules

MetaClean handles files people are about to share, and often files they cannot easily replace. Two principles follow from that:

- **Cleanup stays conservative.** Unsupported or malformed input must fail without modifying the source. Never guess at a format.
- **Nothing phones home.** No telemetry, no uploads, no bundled interpreters, and no statistical watermark rewriting.

## Before you start

Open an issue first for anything that changes behavior in a visible way — new formats, new cleanup rules, or changes to the output modes. Small fixes and docs can go straight to a pull request.

## Before you open a pull request

Add tests for every new public behavior *and* its malformed-input path, then run the full suite:

```bash
pnpm test:coverage    # frontend tests, 80% floor across all dimensions
pnpm test:formats     # Rust/frontend/shell/document format parity
pnpm test:security    # production WebView network and opener boundaries
pnpm test:supply-chain
pnpm test:release
pnpm test:docs
pnpm docs:build        # 构建可搜索的 VitePress 文档站
pnpm build            # typecheck + production bundle
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo audit -f src-tauri/Cargo.lock --no-fetch
```

CI runs the same checks and enforces the coverage floor, so a green local run should mean a green pull request.

## License

By contributing, you agree that your contribution is licensed under the [MIT License](LICENSE).

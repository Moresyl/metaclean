# Contributing to MetaClean

Thanks for helping make private file sharing safer.

1. Open an issue before large behavioral changes.
2. Keep cleanup conservative: unsupported or malformed files must fail without modifying the source.
3. Add tests for every new public behavior and malformed-input path.
4. Run `pnpm test:coverage`, `pnpm build`, `cargo fmt --check`, `cargo test`, and `cargo audit` before opening a pull request.
5. Do not add telemetry, uploads, bundled interpreters, or statistical watermark rewriting.

By contributing, you agree that your contribution is licensed under the MIT License.

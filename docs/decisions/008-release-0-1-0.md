# 发布 0.1.0

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：无，主 agent 自己做

## 背景

路由 B（[007](007-cli-route-b.md)）落地后，Claude 问路由 B 加了公开 API，要不要发 eink-rs 和 eink-rs-macros 的新版本，用户答「发」。crates.io 上没有这两个名字，这次是首发。

## 决策

- 发布 eink-rs 和 eink-rs-macros。用户定（「发」）。
- 以下由 Claude 按用户的授权（「发」）定：
  - 版本号用 0.1.0：两个包没有发过，`Cargo.toml` 里的版本不改。
  - 发布前补元数据：
    - eink-rs 加 `[package.metadata.docs.rs] all-features = true`。没有默认 feature，不加的话 docs.rs 上没有 `eink_rs::cli`。
    - eink-rs-macros 的描述改成两个宏，加 keywords、categories 和指向根目录许可证的软链接，同 ink-rs-macros。
    - README 删掉「未发布」一句，换成 `cargo add` 的用法；feature 表的 `cli` 一行加 `commands!`。
  - 顺序同 ink-rs 的 [062](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/062-release-0-1-3.md)：本地全量、推送 main、等 CI 通过、打 `v0.1.0` 标签、先发 `eink-rs-macros` 再发 `eink-rs`。不建 GitHub Release。推送准备提交属于发布流程。

## 方案

主 agent 自己执行：

1. 改元数据和 README，提交。
2. 本地 fmt、clippy、无 feature 和 `--workspace --all-features` 的测试、`cargo doc`；`cargo package -p eink-rs-macros -p eink-rs` 检查打包的文件和编译。
3. 推送 main，等 CI。
4. 打 `v0.1.0` 标签并推送，`cargo publish -p eink-rs-macros`，再 `cargo publish -p eink-rs`。

# 发布 0.3.1

- 日期：2026-10-23
- 状态：实施中
- 提交：无
- Brief：无，主 agent 自己做

## 背景

[023](023-image.md) 的图片在 Ghostty 直连里核对后，用户回复「可以了 发布吧」。

0.3.0 之后 `src` 的改动是新增的：`image` 模块（`Image`、`ImageTheme`、`ImageLoader`、`ImageLoaders`、`FileLoader`、`HttpLoader`、`LoadError`），`MarkdownImages`，`MarkdownStream` 的 `set_images`、`set_columns` 和 `wait_for_images`。这些项在新的 feature（`image`、`image-http`、`image-svg`、`markdown-image`）后面。没有删改公开的项，没有给字段全公开的结构体加字段。

## 决策

- 用户定：「可以了 发布吧」：发布带图片的版本。
- 以下由 Claude 按用户的授权定：
  - 版本号 0.3.1：改动是兼容的新增，0.x 的版本里升第三位。
  - 只发 eink-rs。`macros/` 没有改动，eink-rs-macros 留在 0.2.0；依赖的 ink-rs 留在 0.1.7。
  - 顺序同 [030](030-release-0-3-0.md)：本地全量、推送 main、等 CI 通过、打 `v0.3.1` 标签、`cargo publish -p eink-rs`。不建 GitHub Release。

## 方案

主 agent 自己执行：

1. 本记录先提交。
2. 升版本号到 0.3.1，提交 `chore: release 0.3.1`。
3. 本地全量：fmt；clippy 的 `--workspace --all-features`、无 feature、`markdown`、`markdown-image`、`image`、`image-svg`；`--workspace --all-features` 和无 feature 的测试；`cargo test --features markdown --test markdown`；`cargo doc` 带 `-D warnings`；`cargo package -p eink-rs` 检查打包的文件和编译；`cargo publish --dry-run`。
4. 推送 main，等 CI，打 `v0.3.1` 标签并推送，`cargo publish -p eink-rs`。
5. 补本记录的结果，删图片的 worktree 和分支，告诉 bingo 会话。

# 发布 0.3.2：动图、视频和 Select 行标记的修复

- 日期：2026-10-23
- 状态：实施中
- 提交：无
- Brief：无，主 agent 自己做

## 背景

[033](033-option-row-markers.md) 修了 Select 长标签换行时行标记旁的 gap。bingo-refactor 会话转来用户的话「发版吧 让eink」，要发一个带这个修复的版本，bingo 随后升级 eink-rs。

main 上 0.3.1 之后没有发版的改动还有 [032](032-animation-video.md) 的动图、视频解码器、`Video` 组件和两个视频修复（代理隧道的白名单、迟到帧跳帧）。推送 033 的提交会把它们一起带上。主 agent 在本会话问用户发版范围，用户选「发 main 全部，0.3.2 (Recommended)」。

0.3.1 之后公开 API 的改动是新增：`video` 模块在默认不开的 `video` feature 后面。`Image` 遇到动图时播放。没有删改公开的项，没有给字段全公开的结构体加字段。

## 决策

- 用户定：「发 main 全部，0.3.2 (Recommended)」：main 上的全部改动发成 0.3.2。
- 以下由 Claude 按用户的授权定：
  - 版本号 0.3.2：兼容的新增和修复，同 [031](031-release-0-3-1.md) 升第三位。
  - 只发 eink-rs。`macros/` 没有改动，eink-rs-macros 留在 0.2.0；依赖的 ink-rs 留在 0.1.7。
  - 顺序同 [031](031-release-0-3-1.md)：本地全量、推送 main、等 CI 通过、打 `v0.3.2` 标签、`cargo publish -p eink-rs`。不建 GitHub Release。CI 失败时修正单独提交，标签打在 CI 通过的提交上。
  - 视频还有两件事等用户定（帧尺寸上限要不要做成示例参数或改默认值，从零开始时去掉 `-ss`），不阻塞这次发版。

## 方案

主 agent 自己执行：

1. 本记录先提交。
2. 升版本号到 0.3.2，提交 `chore: release 0.3.2`。
3. 本地全量：fmt；clippy 的 `--workspace --all-features`、无 feature、`markdown`、`markdown-image`、`image`、`image-svg`、`video`；`--workspace --all-features` 和无 feature 的测试；`cargo test --features markdown --test markdown`；`cargo doc` 带 `-D warnings`；`cargo publish --dry-run`。
4. 推送 main，等 CI，打 `v0.3.2` 标签并推送，`cargo publish -p eink-rs`。
5. 补本记录的结果，告诉 bingo-refactor 会话版本号。

# 发布 0.3.2：动图、视频和 Select 行标记的修复

- 日期：2026-10-23
- 状态：已实施
- 提交：`3f4b27c` chore: release 0.3.2；`e414045` test(video): build expected local inputs with the platform separator；`cc0f2af` fix(video): keep the paint tick until the clock advances
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
  - 推送 `3f4b27c` 后 CI run 39907784271 的 Windows job 失败：ffmpeg 参数的单元测试把本地来源写成 `file:videos/-x`，Windows 上 `Path::join` 拼出 `file:videos\-x`。代码按平台拼路径，ffmpeg 在 Windows 上接受反斜杠；测试的期望值改用 `PathBuf::join` 拼。修正单独一个提交。`cargo test` 在失败的测试二进制处停止，Windows 上的集成测试这次没有跑到，看修正后的 CI。
  - 推送 `e414045` 后 CI run 39908474304 的 ubuntu 和 Windows job 在 `playback_pause_seek_and_unmount_keep_uploads_off_the_drawn_id` 失败，本地 40 次复现 2 次，换回 188549e 之前的 `feed.rs` 跑 60 次没有复现。原因在播放器：暂停时拖动进度，新流的第一帧和时钟的重启落在同一次提交里。重启后的第一个 tick 增量为零，它清掉 `pending_paint`，这时视图的 passive effect 没有上传这一帧；时钟停下，画面停在旧帧。跳帧让 seek 后的第一帧来得快，落进这个时序。修法：时钟走过一个间隔的 tick 清掉 `pending_paint`，增量为零的 tick 保留它。修正单独一个提交。
  - 视频还有两件事等用户定（帧尺寸上限要不要做成示例参数或改默认值，从零开始时去掉 `-ss`），不阻塞这次发版。

## 方案

主 agent 自己执行：

1. 本记录先提交。
2. 升版本号到 0.3.2，提交 `chore: release 0.3.2`。
3. 本地全量：fmt；clippy 的 `--workspace --all-features`、无 feature、`markdown`、`markdown-image`、`image`、`image-svg`、`video`；`--workspace --all-features` 和无 feature 的测试；`cargo test --features markdown --test markdown`；`cargo doc` 带 `-D warnings`；`cargo publish --dry-run`。
4. 推送 main，等 CI，打 `v0.3.2` 标签并推送，`cargo publish -p eink-rs`。
5. 补本记录的结果，告诉 bingo-refactor 会话版本号。

## 结果

- 提交顺序：本记录 `5606036`；`3f4b27c` 升版本号；`e414045` Windows 路径的测试修正；`cc0f2af` 播放器的修正。032 的提交和 033 的 `b43948b` 在本记录之前合入 main。
- 本地全量（`cc0f2af`）：fmt；clippy 的 `--workspace --all-features` 和无 feature、`markdown`、`markdown-image`、`image`、`image-svg`、`video` 六组；`--workspace --all-features` 216 个测试通过，无 feature 的测试通过；`cargo doc` 带 `-D warnings` 通过；`cargo publish --dry-run` 通过。
- 播放器的修正：修正前那个测试本地 40 次失败 2 次；修正后顺序跑 150 次、8 路并发跑 80 次，没有失败。
- CI：run 39907784271（`3f4b27c`）Windows 失败；run 39908474304（`e414045`）ubuntu 和 Windows 失败，原因见决策一节；run 39910469905（`cc0f2af`）4 个 job 通过。
- `v0.3.2` 标签打在 `cc0f2af` 并推送。`cargo publish -p eink-rs` 成功：eink-rs 0.3.2 依赖 ink-rs 0.1.7 和 eink-rs-macros 0.2.0。
- 版本号发给了 bingo-refactor 会话。

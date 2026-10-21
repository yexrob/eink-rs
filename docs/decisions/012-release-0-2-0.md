# 发布 0.2.0

- 日期：2026-10-21
- 状态：已实施
- 提交：`f283b3d` chore: release 0.2.0
- Brief：无，主 agent 自己做

## 背景

应用内路由（[011](011-router.md)）落地后，Claude 问两件事：main 上 11 个提交推不推，删掉 `commands!` 的破坏性改动要不要升 0.2.0 发布。用户答「1. 推送 2发布」。

## 决策

- 推送 main，发布 eink-rs 和 eink-rs-macros。用户定（「1. 推送 2发布」）。
- 以下由 Claude 按用户的授权（同上）定：
  - 两个包都升 0.2.0：删掉 `commands!` 是破坏性改动，0.x 下升次版本号。eink-rs 对 eink-rs-macros 的依赖改成 `0.2.0`。
  - 元数据不变：keywords 已满 5 个；描述、README 和 crate 文档在 `b4420ae` 已加上 router。
  - 顺序同 [008](008-release-0-1-0.md)：本地全量、推送 main、等 CI 通过、打 `v0.2.0` 标签、先发 `eink-rs-macros` 再发 `eink-rs`。不建 GitHub Release。

## 方案

主 agent 自己执行：

1. 改版本号，提交。
2. 本地 fmt、clippy、无 feature 和 `--workspace --all-features` 的测试、`cargo doc`；`cargo package -p eink-rs-macros -p eink-rs` 检查打包的文件和编译。
3. 推送 main，等 CI。
4. 打 `v0.2.0` 标签并推送，`cargo publish -p eink-rs-macros`，再 `cargo publish -p eink-rs`。

## 结果

- 本地全量：`--workspace --all-features` 132 个、无 feature 113 个测试通过；fmt、clippy、`cargo doc` 通过。`cargo package` 的文件表不含 `docs/` 和 `examples/file-routes/`，两个包的打包编译通过。
- main 推送到 `f283b3d`，CI run 39569161859 的 4 个 job 通过。
- `v0.2.0` 标签推送；eink-rs-macros 0.2.0 和 eink-rs 0.2.0 发布到 crates.io。

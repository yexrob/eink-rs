# 发布 0.2.1

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：无，主 agent 自己做

## 背景

[013](013-ship-demo.md) 修了 0.2.0 的一个 bug：`RouteArgs` 让第一个词之后的路由词也去匹配根上的子命令，路径里出现子命令名就解析错。`ship` 示例落地后，Claude 问 main 上 4 个提交推不推、修正要不要发 0.2.1。用户答「推送」「发布」。

## 决策

- 推送 main，发布修正。用户定（「推送」「发布」）。
- 以下由 Claude 按用户的授权（同上）定：
  - 只发 eink-rs 0.2.1。0.2.0 之后 `macros/` 没有改动，eink-rs-macros 留在 0.2.0，eink-rs 对它的依赖不变。
  - 修正只改了解析行为，没有改 API，升补丁号。
  - 顺序同 [012](012-release-0-2-0.md)：本地全量、推送 main、等 CI 通过、打 `v0.2.1` 标签、`cargo publish -p eink-rs`。不建 GitHub Release。

## 方案

主 agent 自己执行：

1. 改版本号，和本记录一起提交。
2. 本地 fmt、clippy、无 feature 和 `--workspace --all-features` 的测试、`cargo doc`；`cargo package -p eink-rs` 检查打包的文件和编译。
3. 推送 main，等 CI。
4. 打 `v0.2.1` 标签并推送，`cargo publish -p eink-rs`。

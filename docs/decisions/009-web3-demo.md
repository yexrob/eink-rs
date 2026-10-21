# 用 eink-rs 改写 web3 demo，加 `cli::run_with`

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[web3-demo](briefs/web3-demo.md)

## 背景

用户：「Pastel 版本的Rust ， 你用刚开发的这个 改写一下昨天的 ~/tmp下的web3的那个demo」。

那个 demo（仓库外，`~/tmp/web3-market`）是 ink-rs 写的全屏行情终端：模拟数据经 `Feed` 接口提供，界面有自选、链上事件、K 线、盘口和成交。它用备用屏渲染。

改写前发现缺口：`cli::run` 固定用 `Options::default()`，命令开不了备用屏。ink-rs 的备用屏只能在 render 时设，运行中不能切换。全屏命令要备用屏，一次性命令的输出要留在终端历史里，两者在同一个 CLI 里。

Claude 问了三个问题，用户都选了推荐项。

## 决策

用户定：

- **命令**：8 个命令，路由 B 生成。`web3` 不带命令时同 `watch`（原全屏终端）；`price`、`book`、`trades`、`chain`（带 index 的组，含 `events`）、`wallet` 输出一屏数据，`--follow` 实时；`swap` 用 eink-rs 的 Select、TextInput、ConfirmInput、Spinner、StatusMessage 走一遍兑换；未知币对报错，退出码 1；开 `completion`。用户选「8 个命令 (Recommended)」。
- **缺口**：eink-rs 加 `cli::run_with`，按解析结果给 render 选项。用户选「eink-rs 加 run_with (Recommended)」。
- **位置**：原地改写；demo 不是 git 仓库，先 `git init` 提交原样。用户选「原地改写，先存原版 (Recommended)」。

以下由 Claude 按用户的授权（以上选择）定：

- `run_with::<C>(options: impl FnOnce(&C) -> Options) -> ExitCode` 解析进程参数，用闭包的返回值渲染，补全和退出处理同 `run`。内部把解析改成返回 `C`，`run_from` 和 `run_with` 共用一个私有函数。
- 只加同步的 `run_with`。不加 `run_from_with` 和 `run_async_with`：demo 用不到，等有人要再加。
- `run_with` 不加测试：它和 `run` 的差别是把闭包的返回值传给 `ink_rs::render`，验证要真终端；demo 在 tmux 里的全屏运行覆盖它。
- demo 按路径依赖本地的 eink-rs。发 0.1.1 等 demo 跑通后另问用户。
- 数据只经 `Feed` 读，`Simulator` 是它唯一的实现；根组件建一次 feed，用 ink-rs 的 context 提供给命令。
- demo 不加测试：原 demo 没有测试，它是演示。

## 备选与取舍

- **demo 自己 parse 再 render**：不改框架，但退出码处理要在 demo 里重写一份。
- **全屏命令不用备用屏**：输出铺满终端时每帧清屏，会闪，冲掉终端历史。
- **命令声明自己的选项**（trait 方法或属性）：要改 `Commands` derive 或宏，按命令分派选项；闭包拿到整个解析结果，能表达同样的事。
- **新目录**：原版不动，但多一份副本；git 历史能保留原版和 diff。

## 方案

1. 主 agent：`src/cli/mod.rs` 加 `run_with` 和 rustdoc，`docs/design.md` 的入口一节加一句；跑 clippy、doc 和 `--workspace --all-features` 的测试，和本记录一起提交。
2. 派一个 Codex，在 `~/tmp/web3-market` 改写，brief 见 [web3-demo](briefs/web3-demo.md)。不跑需要终端的命令。
3. 集成（主 agent）：review，clippy，在新的 tmux 会话 `web3` 里运行全部命令，在 demo 的 git 里提交，补本记录的结果。

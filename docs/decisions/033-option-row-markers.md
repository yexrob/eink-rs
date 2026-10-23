# Select 行的标记不随长标签收缩

- 日期：2026-10-23
- 状态：实施中
- 提交：无
- Brief：无，主 agent 自己做

## 背景

bingo-refactor 会话转来一个问题（对方说用户同意转来）：Select 的选项文字长到换行时，序号和文字之间的空格消失，画成 `2.Allow and keep Write(...)`。对方判断是 ink-rs 0.1.7 在收缩时忽略 gap，Ink（Yoga）会保留 gap。

主 agent 核对：

- 30 列下渲染 `View::new().gap(1).padding_left(2).child(Text("2.")).child(Text(label))`，ink-rs 和 Ink 8.0.0（npm 安装，`renderToString`）的输出逐字相同，都是 `2.Allow and keep`。ink-rs 没有偏离 Ink。
- 原因：一行里的子节点按 flex basis 的比例收缩。标签按 28 列折行，量出的 basis 是 28；序号 2，gap 1，合计超出 3 列。序号收缩到 1.8，标签的起点落在 4.8，文字节点的起点向下取整成 4，gap 在画面上消失。焦点指针 `❯` 和选中标记 `✔` 一样会收缩，Ink UI 的 Select 对指针有同样的问题。
- 把序号包进 `flexShrink: 0` 的 Box 后，Ink 和 ink-rs 的输出相同：`2. Allow and keep`，标签从第 5 列开始。

## 决策

- Claude 定（对方会话的请求，对方说经用户同意）：不改 ink-rs，改 eink-rs 的 `option_list`。指针、序号、选中标记各包进 `flex_shrink(0.0)` 的 `View`，收缩全部留给标签。这一点偏离 Ink UI：Ink UI 的指针在长标签时会丢 gap。
- 加一个测试：30 列、不带颜色，一行长标签带指针、序号和选中标记，期望值取 Ink 8.0.0 渲染同样结构的输出。理由：这个问题出现在 bingo 的权限询问里，行的结构以后改动时没有测试会退回去。

## 方案

主 agent 自己做：改 `src/ui/option_list.rs`，在 `tests/select.rs` 加测试；fmt、clippy、`--workspace --all-features` 的全量测试。完成后告诉 bingo-refactor 会话；发版由用户定。

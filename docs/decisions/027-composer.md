# 受控的多行输入框 `Composer`

- 日期：2026-10-22
- 状态：实施中
- 提交：无
- Brief：[composer](briefs/composer.md)；需求：[bingo-composer](briefs/bingo-composer.md)

## 背景

bingo 要一个编码 agent 的提示输入框，同 Claude Code。文字归宿主：插件改写编辑、填草稿、读框里的内容、监听改动、加补全行。所以组件是受控的：文字和光标从 props 进来，编辑从回调出去，框里下一刻是什么由宿主定。eink-rs 有单行的 `TextInput`，`reduce_input` 按字形计光标，没有词、行、上下移动、删除键和 kill 缓冲；quill 示例手写了一个多行输入框。

调研报告（`<scratchpad>/research/bingo/composer.md`）的结论：

- 光标单位不同：bingo 要字节偏移，`InputState` 按字形计数。改 `InputState` 破坏它的公开 API，所以并列一个新的 reducer。
- 折行由组件自己做：按 `text_width` 累加字形，放不下的移到下一行，每个视觉行画成一个 `Text`。ink-rs 的 `wrap_text` 不公开，不需要它。
- 光标：`use_cursor` 收帧坐标，组件从 `measure_element` 推算位置，祖先移动不触发重渲染，光标慢一帧，输入法候选框跟着跳。
- 同一批输入的多个按键之间不重渲染，处理器读到上一次渲染的 props，受控组件要一份乐观值。
- Ctrl+J 到达为 `input == "\n"`、`enter == false`；Shift+Enter 在 kitty 键盘下可分，传统终端里和 Enter 相同；kitty 的事件类型下 Ctrl+C 的松开事件会到达。

## 决策

- 用户定：
  - 「加两个 (Recommended)」：ink-rs 0.1.7 加锚定节点的光标（ink-rs 的记录 074），Composer 用它，光标在绘制时按节点的位置算出。
- 主 agent 在第四轮方案里提出、用户随方案确认：
  - 新增 `ui::Composer` 和无头的 `reduce_composer`，与 `reduce_input` 并列，共用私有的字形 helper，主题沿用 `InputTheme`。
  - 折行和逐行绘制由组件做，不加 ink-rs 的折行原语。
  - 每次改动调用 `on_edit(Edit)`，宿主不采纳时框里的内容不变；组件保留一份乐观值，每次渲染用 props 覆盖。
  - kill 缓冲归组件；无头的使用者自己传给 `reduce_composer`。
  - Ctrl+A、Ctrl+E、Home、End 按逻辑行移动。
  - Composer 不进焦点环，宿主用 `is_disabled` 决定按键归它还是归条带。
- Claude 按以上定细节：
  - 宽度取 `use_box_metrics` 量到的宽度，第一次量到之前用窗口列数；减去提示符的宽度和行尾光标占的 1 格。制表符按 4 列显示，写进文档的限制。
  - 光标用 `use_node_cursor` 锚在文字所在的 `View` 上；画一个反色的光标格，给不读光标的路径（字符串渲染、测试）。禁用时声明 `None`，不画光标格。
  - `max_rows` 之内框随内容长高，超过后在框内滚动，首行存在 `use_memo` 里，每次渲染调整到光标行可见。
  - 按键表见 brief：Enter 在光标前是 `\` 时换成换行，否则 `on_submit`；Shift+Enter、Alt+Enter、Ctrl+J 换行；Up 和 Down 在首末视觉行调用 `on_history`；Ctrl+W、Ctrl+U、Ctrl+K 填 kill 缓冲，Ctrl+Y 插入；Ctrl+C 滤掉 kitty 的松开事件。
  - 粘贴：`\r\n` 和 `\r` 换成 `\n`，整段是一次 `Insert`。
  - quill 的 `composer.rs` 换成 Composer，去掉它的前缀、提及的绕行、手画的光标和 `\n` 处理，quill 的行为和测试不变。
  - 加示例 `examples/composer.rs`，主 agent 在 tmux 里手动核对，含输入法。

## 备选与取舍

- **扩展 `reduce_input`**：光标单位要从字形改成字节，破坏 `InputState` 的公开 API，三个单行输入框跟着改。
- **ink-rs 公开带字节区间的折行原语**：一份实现，要等 ink-rs 发版；组件自己画行约 40 行。
- **接受光标慢一帧**：祖先移动时（上方的流式输出变长）输入法候选框跳动。
- **Ctrl+A、Ctrl+E 按视觉行**：和 Claude Code、readline 不同。

## 方案

一个 Codex worker，worktree `wt/eink-composer`，基于 main。ink-rs 0.1.7 发布前，worktree 的 `Cargo.toml` 用 `[patch.crates-io]` 指向 ink-rs 合入 074 之后的快照，提交前去掉。worker 不提交，不改文档。brief 见 [composer](briefs/composer.md)：新增 `src/ui/composer.rs`、`tests/composer.rs`、`examples/composer.rs`；改 `src/ui/mod.rs`、`src/ui/input.rs`（共用 helper）、`examples/quill/`。测试：reducer 每组键一例，含 CJK、ZWJ emoji、折行后的上下移动、`\` 加 Enter、历史边界；渲染的长高和内滚、行尾的宽字形、`on_caret`；受控的改写、同批两键、不采纳；粘贴；光标随上方的行变长移动。验收：fmt；clippy 两组 feature；`--lib`；`--test text_input`；新增的测试文件；`-p quill`。

主 agent 集成：review，提交，全量测试，`docs/design.md` 的 ui 一节，在 tmux `ink-rs` 会话里跑示例核对（含输入法），补结果，删 worktree。进 0.2.2 之后的版本。

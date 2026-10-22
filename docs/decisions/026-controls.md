# Button、受控的 TextInput 和 Select、焦点环

- 日期：2026-10-22
- 状态：实施中
- 提交：无
- Brief：[controls](briefs/controls.md)；需求：[bingo-controls](briefs/bingo-controls.md)

## 背景

bingo 按 Claude Code 的 `ui.render` 渲染 mod 和插件返回的元素树：按下、输入、选择变成事件，界面画回来的树。它自己的对话框（权限确认、模型选择）用同一套控件。所以控件是受控的，一个站点（窗格的正文，或输入框上方的条带）的控件共用一个焦点环。eink-rs 有非受控的 `TextInput`、`Select`、`ConfirmInput`，没有 `Button`。

调研报告的结论：

- 给现有组件加 `value` 属性进入受控模式，内核、主题、测试共用。
- ink-rs 的焦点全应用一份，按注册顺序编号，Tab 和 Esc 在处理器之前处理，不能限定子树；焦点环在 eink-rs 里用 `provide`、`use_context`、`use_state`、`use_layout_effect`、`use_input` 自建，不需要 ink-rs 新原语。
- ink-rs 的按键没有消费机制，站点之间由宿主门控，同一时刻一个持有者。
- `Select` 的 Enter 在选中当前值时不调用回调，权限确认需要 `on_select`。`visible_option_count` 存在。

## 决策

- 用户定：
  - 焦点顺序：「你问问bingo那个session 看看应该用哪个？」。bingo-refactor-98 答选 A（按屏幕位置）：站点持有焦点时，hook 在渲染之间增删控件（按下后删掉一行、对话框换按钮、条带出现和消失），按挂载顺序会把插入的控件排到末尾。对方注明这是它的推荐，它的用户未表态；本会话的用户把选择交给了 bingo。
  - 其余按方案确认：数字热键默认关；Composer 不进焦点环。
- bingo 会话提出的需要（转述，按用户把焦点问题交给 bingo 的授权采纳）：
  1. 成员按字符串 key 寻址，焦点 key 跨重绘保留；焦点成员离开树时选邻居或无，规则由这里定。
  2. 每次移动由宿主控制：Tab 和方向键给出提议，宿主设焦点完成移动，可以异步拒绝或改道。
  3. 宿主能注册非控件的停靠点（窗格的关闭标记、别的窗格的标签）。
  4. `auto_focus`：站点拿到键盘时，第一个 `auto_focus` 成员得到焦点。没有 `auto_focus` 成员时不提议任何成员（对方第二条消息，按 Claude Code 的行为：Enter 不会作用在作者没标记的按钮上，例如有破坏性的按钮）。
  5. 方向键走环由宿主决定：对话框式的窗格走环，普通窗格滚动。
- Claude 按以上定细节：
  - 每次移动调用 `on_focus(提议)`；`focused` 由环内管理时环自己应用，由宿主设置时等宿主改 `focused`。`focused` 的形状由 worker 定，要能表达环内管理、某个 key、无焦点三种状态。
  - 焦点成员离开时，按它末次测得的位置提议：其后的下一个成员；没有下一个时取前一个；环空时不提议。
  - `arrows: bool` 默认关；打开时方向键在焦点成员不占用方向键时走环（`Select` 占用方向键，`TextInput` 占用文字和方向键）。
  - 文本输入获焦时，别的成员的字母、数字热键停用。
  - 受控的 `TextInput` 保留一份乐观值：同一批输入的多个按键之间不重渲染，处理器读到的是上一次渲染的 props。
  - 环不调用 `use_focus`；和 ink-rs 的焦点混用时 Tab 处理两次，文档写明。
  - 加示例 `examples/focus-ring.rs`，主 agent 手动核对用。

## 备选与取舍

- **按挂载顺序（B）**：调研报告的推荐，和 ink-rs 的 `use_focus` 相同；插入的控件排到末尾。
- **ink-rs 加树序原语（C）**：要等 ink-rs 发版，屏幕位置在 bingo 的布局里和树序一致，反向布局除外。
- **新建受控组件**：API 翻倍，Email 和 Password 要跟着分叉。

## 方案

一个 Codex worker，worktree `wt/eink-controls`，不提交，不改文档。brief 见 [controls](briefs/controls.md)：新增 `src/ui/button.rs`、`src/ui/focus_ring.rs`；改 `text_input.rs`、`input.rs`、`select.rs`、`option_list.rs`、`multi_select.rs`、`mod.rs`。测试按 brief 的规则逐条：环的循环、停用、Esc、受控提议、两环互不影响；插入和删除；自动聚焦；方向键；热键冲突；受控 TextInput 的同批两键；受控 Select 的 `on_select`、数字键、描述；Button 的画法；bingo 要求的集成用例。现有的 Select 和 TextInput 测试不改。验收：fmt；clippy 两组 feature；`--lib`；brief 列出的测试文件。

主 agent 集成：review，提交，全量测试，`docs/design.md` 的 ui 一节，在 tmux `ink-rs` 会话里跑示例核对，补结果，删 worktree。进 0.2.2 之后的版本。

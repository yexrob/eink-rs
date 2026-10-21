# Ink UI 示例

- 日期：2026-10-21
- 状态：已实施
- 提交：`e5a6c6b` docs: port Ink UI's examples
- Brief：[inkui-examples](briefs/inkui-examples.md)

## 背景

[002](002-inkui-port-plan.md) 落地后，用户说「推送， 然后和ink-rs一样 写一些examples 然后在新的tmux session运行让我看。」ink-rs 的 `examples/` 按原名移植了 Ink 的示例。Ink UI 2.0.0 有 17 个示例：14 个组件示例，加 `theming/` 下 3 个。

## 决策

- 推送 eink-rs 的 main，写示例，在新的 tmux 会话里运行给用户看。用户定（原话见背景）。
- 以下由 Claude 按用户的授权（「和ink-rs一样 写一些examples」）定：
  - 移植 Ink UI 全部 17 个示例，文件名和 Ink UI 相同；`theming/` 下的 3 个改名 `theming-<name>`，示例平铺在 `examples/` 下。
  - 写法同 ink-rs 的示例：文件头注明来源，和 Ink UI 不同的地方逐条加注释。
  - 主题示例用 `cx.provide` 提供组件的主题类型，代替 `ThemeProvider` 和 `extendTheme`。
  - 不给示例写测试：CI 的 clippy `--all-targets` 会编译它们，行为由组件测试覆盖。

## 方案

- 派一个 Codex，独立 worktree，brief 见 [inkui-examples](briefs/inkui-examples.md)。验收：`cargo build --examples`、fmt、clippy。
- 主 agent review 后提交，在 tmux 会话 `eink-rs` 里逐个运行示例给用户看，按键用 `send-keys` 驱动。

## 结果

Codex 移植了 17 个示例，`cargo build --examples`、fmt、clippy 通过。主 agent review 时删掉了文件头里描述组件行为的注释（字素光标、回调时机、Unicode 图标等），这些是组件和 Ink UI 的差异，已写在 `docs/design.md`；文件头只留示例本身的差异，比如 `visible_option_count` 代替 `defaultLimit`、子节点代替 `.Item`。

tmux 会话 `eink-rs` 里逐个运行，`send-keys` 驱动按键，截屏核对：

- 静态示例画完后自行退出，progress-bar 约 5 秒走到 100% 后退出，同 Ink UI。
- text-input 输入 `hello`、左移两格插入 `X` 得到 `helXlo`；email-input 输入 `vadim@g` 时补全 `mail.com`，Enter 后值为 `vadim@gmail.com`；password-input 显示六个 `*`。
- confirm-input 按 `y` 后显示同意的文案；select 下移两格 Enter 选中 yellow；multi-select 空格选中 red 和 yellow。
- autocomplete 输入 `e` 后过滤出含 `e` 的选项，`e` 加粗；下移 Enter 后显示 `You've selected green`。
- theming-spinner 的帧是品红，theming-unordered-list 的标记是 `+`。

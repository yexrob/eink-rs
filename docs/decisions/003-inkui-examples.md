# Ink UI 示例

- 日期：2026-10-21
- 状态：实施中
- 提交：无
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

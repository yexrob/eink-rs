# 应用内路由调研

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[router-survey](briefs/router-survey.md)、[router-check](briefs/router-check.md)

## 背景

路由 A、B（[004](004-cli-route-a.md)、[007](007-cli-route-b.md)）是命令行路由：启动时按 argv 选中一个命令组件，运行中不再切换。用户贴了 Claude 对两者区别的说明，答：「可以 你继续改进eink-rs吧，我们不只是和clap一样路由， 路由更多是指 类似react里面的router。 包括我发的那几个点也都一起讨论一下」。

贴文列的几点：

- 路由状态：当前路径和历史栈，经 context 提供。
- `use_router()`：`navigate`、`back`、路径参数。
- 嵌套布局：父路由的布局保持挂载，子路由渲染在 outlet；子路由切换时布局的 state 保留。
- 路由表：手写，或沿用 `commands!` 的目录约定从文件生成；动态段（如 `[pair].rs`）的规则另定。
- 和命令行衔接：`web3 market ETH` 作为初始路由打开，之后在应用内跳转。
- Claude 判断 ink-rs 不用加原语，没有核对源码。

## 决策

- 在 eink-rs 里加 React Router 式的应用内路由，命令行路由保留。用户定（「可以 你继续改进eink-rs吧，我们不只是和clap一样路由， 路由更多是指 类似react里面的router」）。
- 以下由 Claude 按用户的授权（同上，以及「包括我发的那几个点也都一起讨论一下」）定：
  - 先调研，带着方案和用户逐点讨论，定下后另开实施记录。先对方案是用户此前定下的做法。
  - 两路调研并行：
    - Sonnet 子 agent 调研现有路由：React Router、Rust 的 Dioxus、Yew、Leptos 路由、终端里的先例（Ink 生态、Textual 的 screen 栈、k9s 和 lazygit）、文件路由的约定、argv 打开初始路由的先例。用户此前说「文档探索任务以后不要拍Opus， sonnet就够了」。
    - Codex 在 scratch 包里实测 ink-rs：布局保持挂载、同类型新 props、context 传递、从输入和 effect 导航、焦点、`Static`、路由表能存什么。核对贴文里「不用加原语」的判断。
  - 进行中的 web3 demo（[009](009-web3-demo.md)）照常做完；应用内路由落地后再看 demo 要不要用。

## 方案

- Sonnet 子 agent，brief 见 [router-survey](briefs/router-survey.md)，报告写到 scratchpad。
- Codex，brief 见 [router-check](briefs/router-check.md)，scratchpad 里的 scratch 包，不改仓库。
- 主 agent 读完两份报告，按贴文的几点整理方案和取舍，和用户讨论。结论补进本记录。

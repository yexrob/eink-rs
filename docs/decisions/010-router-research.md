# 应用内路由调研

- 日期：2026-10-21
- 状态：已实施
- 提交：无（只有文档）
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

## 结果

### 调研

- Sonnet 调研了现有路由，结论：
  - React Router 的 MemoryRouter 在 push 时丢掉前进条目，`go` 越界时夹在两端，栈底返回原地不动。
  - Dioxus 和 Yew 用 enum 声明路由，参数是带类型的字段；Leptos 用字符串路径。
  - 终端先例：Ink 食谱用 MemoryRouter，不处理返回键和焦点；Textual 让栈里的屏幕保活；BubbleO 在栈空时退出；k9s 用 Esc 返回，`-c` 和 `:` 共用一套词。
  - `[id]`、`$id`、`(group)` 都不是合法的 Rust 模块名，axum_folder_router 用 `#[path]` 指向真实文件名。
- Codex 用 18 个运行时测试和 4 个编译失败测试实测了 ink-rs 0.1.3，结论是不用加原语：
  - 布局的 state 在子页面切换时保留；卸载的页面执行 Drop，输入和动画随之停止。
  - 同类型同位置的组件换 props 时保留 state，换 key 时重挂载。
  - context 能穿过 props 不变的父组件，把变化送到深层；放进 context 的 `Callback` 可以改祖先的 state。
- 实测中发现四个约束，路由要绕开：
  - `State` 不是 `PartialEq`，放不进 context；`get()` 也不订阅读取者。路由改为提供当前路由的值和导航用的 `Callback`。
  - 按键没有停止传播：触发导航的那个键，旧页面的其他处理器也会收到。
  - 焦点组件卸载后焦点为空；只有在没有焦点时 autofocus 才生效。
  - `component` 不接受函数指针，路由表只能用 enum 加 match。

### 讨论

用户定：

- **路由表示**：类型化 enum 加 `#[derive(Routes)]` 和路径串，`router.push(Route::Market { .. })` 和 `push_path("/market/ETH")` 都能用。用户选「类型化 enum + #[derive(Routes)] 路径串」，没选推荐项「命令 enum 直接当路由」。
- **返回键**：应用自己绑。Router 提供 `back()` 和 `can_go_back()`，栈底的 `back()` 不动。用户选「应用自己绑 (Recommended)」。
- **返回时**：重挂载，同 React Router。用户选「重挂载 (Recommended)」。
- **目录生成**：用户答「我之前说的路由就是现在要做的路由 之前的路由可以不用目录 只用enum就行了」。Claude 追问后用户选「删掉，只留 #[derive(Commands)]」：目录生成改给应用内路由，命令行路由的 `commands!` 删掉。
- **argv**：argv 的词就是路径段，`web3 market ETH` 对应 `/market/ETH`。用户选「argv 的词就是路径段 (Recommended)」。
- **嵌套**：嵌套 enum，带子路由的变体上写 `#[layout]`，布局里放 `Outlet`。用户选「嵌套 enum (Recommended)」。
- **动态段**：目录写 `[pair]/`，宏用 `#[path]` 指向它。用户选「`[pair]/` 加 `#[path]` (Recommended)」。

以下由 Claude 按用户的授权（「包括我发的那几个点也都一起讨论一下」）定，已在讨论时告知用户：

- 历史只做 push、replace、back；push 同一路由不入栈；不做 forward。
- 焦点交接和按键传播 Router 都不管，写进文档。ink-rs 不加停止传播，Ink 也没有。
- 不做 loader/action、blocker、命令面板。
- ink-rs 不改。

实施另开记录。

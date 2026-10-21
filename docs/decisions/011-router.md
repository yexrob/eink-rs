# 应用内路由：Router、`#[derive(Routes)]`、`routes!`

- 日期：2026-10-21
- 状态：实施中
- 提交：无
- Brief：[router-core](briefs/router-core.md)

## 背景

[010](010-router-research.md) 调研了现有路由和 ink-rs 的行为，和用户逐点定了应用内路由的形状：

- 路由表用 enum 加 `#[derive(Routes)]` 和路径串，嵌套用嵌套的 enum，布局用 `#[layout]` 加 `Outlet`。
- 历史只有 push、replace、back。返回键由应用自己绑，返回时页面重挂载。
- argv 的词就是路径段。
- 目录生成改给应用内路由：`[pair]/` 加 `#[path]`。命令行的 `commands!` 删掉。

Claude 按这些选择拼出完整方案发给用户，还剩一问：页面怎么拿路径参数。用户选「按类型从 context 取 (Recommended)」，没有对其余部分提出修改。

## 决策

用户定的见 010 的讨论一节，以及上面参数一问。以下由 Claude 按用户的授权定（010 的「包括我发的那几个点也都一起讨论一下」，加上用户对方案没有提出修改）：

- **模块**：`eink_rs::router`，新增 feature `router`，带上 `eink-rs-macros`。trait 和 derive 都叫 `Routes`，同 serde 的 `Serialize`。
- **derive**：
  - 每个变体写一个 `#[route("/a/:b")]`。第二个参数可以指定页面类型，不写就用和变体同名的类型。
  - 路径里的 `:name` 对应同名字段；不在路径里的字段就是子路由，一个变体最多一个，它的类型也实现 `Routes`。
  - 变体上的 `#[layout(L)]` 包住它的子路由，只能写在带子路由的变体上。enum 上的 `#[layout(L)]` 包住这一层的每条路由，根布局就这么写。
  - 页面和布局都是 `Default + Into<Element>` 的类型，按 rsx 的写法用 `T::default()` 构造，props 里不带参数。
  - 匹配时静态段优先于参数段；同一模式下，叶子路由优先于带子路由的变体；其余按书写顺序。参数的 `FromStr` 失败，算这条不匹配。两条叶子路由的模式相同时编译报错。
  - 生成 `Routes` 的实现，以及 `FromStr` 和 `Display`，用来转换路径字符串。参数值里不能有 `/`。
- **参数**：每个参数值经 context 提供给它下面的整棵子树，`use_param::<T>(cx)` 按类型取，取最近的那个；没有就 panic，并说明缺的类型。手写时参数字段用 newtype 以免重名；目录路由给每个 `[x]` 生成 `X(String)`。
- **运行时**：
  - `Router::new(initial)` 把历史放在 `use_state` 里。它提供当前路由，以及一个 `use_callback` 做的导航 `Callback`（`State` 放不进 context，见 010），然后渲染当前路由。初始路由只在挂载时读。
  - `use_router::<R>(cx)` 返回 `RouterHandle<R>`，提供 `route()`、`can_go_back()`、`push`、`replace`、`back`、`push_path`。push 和当前路由相同时不入栈，栈底 `back()` 不动。命名对齐 ink-rs 的 `use_app` 和 `AppHandle`。
  - 布局里的 `Outlet` 从 context 取子路由的元素，没有就渲染空元素，同 React Router。ink-rs 的 context 就近取值，嵌套布局各取各的。
- **argv**：`RouteArgs<R>` 实现 clap 的 `Args` 和 `FromArgMatches`，需要 `cli` 和 `router` 两个 feature。它把剩下的位置参数逐个作为路径段解析成 `R`；匹配不上时报 clap 的用法错误，退出码 2。clap 先匹配子命令。
- **目录**：`routes!("src/routes")` 生成同样的嵌套 enum。
  - `foo.rs` 对应 `/foo`，`index.rs` 对应本层的 `/`，`foo/` 生成子 enum，`[pair]/` 对应 `/:pair`，`_layout.rs` 是本层布局。
  - 动态段只能是目录，`[pair].rs` 报错：页面类型和参数类型会重名。
  - 沿用路由 B 的扫描、命名检查和 build.rs 规则。
- **删除 `commands!`**：连同 `examples/cli-routes`、文档和测试一起删。eink-rs 和 eink-rs-macros 升到 0.2.0，发布前问用户。web3 demo 改回手写的 enum。
- ink-rs 不改。

## 备选与取舍

- **页面 props 带参数（Dioxus 式）**：手写的子 enum 要声明从上层继承哪些参数，derive 复杂一层。用户选了 context。
- **只有 enum 级布局**：拿不到变体自己的参数，这种布局得从 context 取参数。现在两种都支持：变体级的布局能拿到自己那层的参数，因为参数的提供者在布局之上。
- **Router 带 children，由应用把 `Outlet` 放进外壳**：同 React Router 的 `MemoryRouter`。但它和 enum 级布局是一回事，多一种写法，所以不采用。

## 方案

1. 派一个 Codex，独立 worktree，brief 见 [router-core](briefs/router-core.md)：`router` 模块、`#[derive(Routes)]`、`RouteArgs`、`tests/router.rs`、`examples/router.rs`。只跑 brief 里列出的测试。
2. 第二个 Codex 在第一个合入后派出：`routes!`、删除 `commands!`、示例。brief 那时再写，并补进本记录。动手前先核对 rust-analyzer 对宏生成的 `#[path]` 的支持。
3. 主 agent 负责：
   - 每步 review 后合入，跑全量测试；
   - 写 `docs/design.md` 的 router 一节和 README；
   - 在 tmux 会话里跑示例给用户看；
   - 改 web3 demo；
   - 问用户是否推送和发布 0.2.0。

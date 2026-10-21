# eink-rs 设计

eink-rs 建在 [ink-rs](https://github.com/yexrob/ink-rs) 上，有两部分：

- `ui`：移植 Ink UI 2.0.0 的 13 个组件和 `useSpinner`。
- `cli`：参照 Pastel 4.0.1 的 CLI 框架，命令用 clap 声明，由组件渲染。
- `router`：参照 React Router 的应用内路由，路由表是 enum，可以从目录生成。

设计怎么来的见 [decisions/](decisions/README.md)。

## 结构

- workspace 的根 package 是 `eink-rs`。proc macro 在 Rust 里要独立成 crate：成员 `macros/`（`eink-rs-macros`）放 `Commands`、`Routes` 两个 derive 和 `routes!`。成员 `examples/file-routes/` 是目录路由的示例包，不发布。
- `ui` 只依赖 ink-rs。feature 默认都不开：`cli` 加 clap 和 derive，`router` 加路由和它的宏，`completion` 加 clap_complete，`tokio` 加异步入口并开 ink-rs 的 `tokio`；后两个都带上 `cli`。只用组件的项目不编译 clap。
- 依赖 crates.io 上的 ink-rs，关掉它的默认 feature，开 `macros`。缺的通用原语加进 ink-rs，不在这里绕过。

## ui

### 三层

每个组件一个模块，最多公开三样：

- **状态 hook**：有状态的组件公开 `use_x_state(cx, ..)`，不含按键绑定和渲染。状态更新写成纯函数，用 `State::get` 和 `State::set` 应用。回调在按键处理里调用，参数取这次更新后的值。三个文本输入框共用 `use_input_state` 和 `reduce_input`；Select、MultiSelect 各有 `use_select_state`、`use_multi_select_state`；Spinner 公开 `use_spinner`。ConfirmInput 和其余展示组件没有状态。
- **组件**：props 结构体派生 `Props`，渲染函数私有。交互组件有 `is_disabled`，为 false 时用 `use_input` 读键。焦点交给应用：用 `use_focus` 算出 `is_disabled`。
- **主题**：`XTheme` 结构体，字段是颜色、字符、样式开关和间距，实现 `From<&Palette>`。布局结构（方向、图标不收缩）写在组件里，不进主题。三个文本输入框共用 `InputTheme`（光标、占位符、补全的样式和掩码字符）；`SelectTheme` 和 `MultiSelectTheme` 字段相同，类型分开，可以分别 provide。

### 主题

组件用 `use_theme::<XTheme>(cx)` 读主题，顺序是：

1. 最近的祖先 `provide` 的 `XTheme`；
2. 用最近的 `Palette` 生成；
3. 用默认 `Palette` 生成。

```rust
cx.provide(Palette { primary: Color::Magenta, ..Palette::default() });      // 换焦点色
cx.provide(SelectTheme { focus_color: Color::Cyan, ..SelectTheme::from(&Palette::default()) }); // 只改 Select
```

`Palette` 的默认值取自 Ink UI 默认主题里的颜色：

| token | 默认 | 用在 |
|---|---|---|
| `primary` | 蓝 | 焦点行和指示符、Spinner |
| `info` | 蓝 | info 状态 |
| `success` | 绿 | success 状态、已选项 |
| `warning` | 黄 | warning 状态 |
| `error` | 红 | error 状态 |
| `accent` | 品红 | ProgressBar、Badge |

Ink UI 的主题是一组函数，返回任意 Box/Text 属性，用 deepmerge 合并。这里的主题是数据：只能改列出来的字段，换来类型检查和 `PartialEq`。

### 和 Ink UI 不同的地方

- **列表**：Rust 写不出 `OrderedList.Item`，也不按类型识别子节点。列表的每个子节点算一项，`None` 转成的空元素不算，编号也跳过它。一项里放多个元素时用 `View` 包起来，嵌套列表写在这个 `View` 里。
- **图标**：用 Unicode 字符（✔ ✘ ⚠ ℹ ❯ ─），不做 figures 的非 Unicode 回退。
- **Select**：选项是 `SelectOption<T>`，值是泛型。焦点记下标，值为空串或重复时导航照常。`highlight_text` 是 `String`，空串不高亮。
- **Spinner**：只带 cli-spinners 的 `dots`，其他动画用 `SpinnerFrames::new` 传帧和间隔。

移植时改掉的 Ink UI 行为：

- 回调在按键处理里调用。Ink UI 在 effect 里比较前后值，回调或选项换了引用会用同一个值再调一次。
- 输入框的光标按字素移动；Backspace 在开头什么也不删；Enter 有补全时把补全接在值的末尾，显示和提交的值一致；按住 Ctrl 或 Meta 的键不插入字符。
- Select 在空列表上按 Enter 保留默认值；选项变化在 layout effect 里重置，不画出旧状态的帧。
- Spinner 换帧后从第一帧开始。
- ProgressBar 的值为 NaN 时按 0 算。
- StatusMessage 和 Alert 按 Ink UI 标成 `test.failing` 的测试实现。

### 测试

`tests/` 下每个组件一个文件，移植 Ink UI 的测试。组件用 `ink_rs::testing::render` 渲染，期望值是 ink-rs 的元素树，用 `tests/support` 的 `frame` 按同样的设置（100 列、基本色）渲染，比较带颜色的帧。期望和 Ink UI 不同的地方有注释；改掉的行为里 Ink UI 测试没覆盖的，各加一个测试。


## cli

参照 Pastel 4.0.1，只借概念：命令即组件、包裹、index 与 default 命令、别名。help 格式、`-v` 和 Pastel 的测试不对齐。命令用 clap 的 derive 手写子命令 enum。0.1 的 `commands!`（从目录生成命令）在 0.2 删掉，目录生成改给应用内路由，见 [011](decisions/011-router.md)。

### 写法

- 命令的 `Args` 结构体同时 derive `Props`，结构体就是组件的 props，`From<Args> for Element` 来自 ink-rs。
- 子命令 enum 上 `#[derive(Commands)]` 生成 `From<Enum> for Element`，每个变体转换它唯一的字段。嵌套的组是持有另一个 `Commands` enum 的变体。泛型 enum、单元变体、具名字段和多字段的变体报错。
- 顶层 `Parser` 也是组件。Pastel 的 `_app`、index 命令和 default 命令写在它的渲染函数里：外面包一层，里面 `match` 子命令，没有专门的 API。
- 名称、版本、描述用 clap 的 `#[command(version, about)]` 从 `Cargo.toml` 读。`eink_rs::cli` 重新导出 `clap`，用户不必自己对齐版本。

### 入口与退出码

- `run::<C>()` 解析 `std::env::args_os()`，`run_from::<C>(argv)` 解析给定的参数，用 `Options::default()` 渲染，等到退出。`run_with::<C>(options)` 解析进程参数，把解析结果交给闭包取 render 选项，命令可以按自己开备用屏。`tokio` 下的 `run_async`、`run_async_from` 用 ink-rs 的 tokio 驱动，要在 local runtime 或 `LocalSet` 里运行。同步和异步共用解析与退出处理，只有渲染和等待不同。
- 退出码：clap 的错误打印后用它的码，help、version 为 0，用法错误为 2；`exit_with_error` 和其他 ink-rs 错误向 stderr 打印 `Error: {error}`，返回 1；`exit_with` 的值是 `ExitCode` 时用它；其他退出值、`exit()` 和应用自行结束返回 0；组件 panic 用原 payload 继续 unwind，保留 101。
- `completion` 下，`run` 系列给 `C::command()` 加 `completions <SHELL>`，匹配时向 stdout 写补全脚本，返回 0。用户的命令不能再叫 `completions`。

### 测试

`parse_from::<C>(argv) -> Result<Element, clap::Error>` 是测试辅助：元素交给 ink-rs 的 `testing::render` 或 `testing::mount`，不加 `completions`。`tests/cli.rs` 只测 derive 分派（含嵌套组和别名）、顶层组件的 default 命令、五种退出码和 `completions` 子命令。Pastel 的测试比对 commander 的 help 文本，不移植。

## router

参照 React Router 的 `MemoryRouter`：路由表、历史、嵌套布局和 `Outlet`、路径参数。没有前进、loader、导航拦截。讨论见 [010](decisions/010-router-research.md) 和 [011](decisions/011-router.md)。

### 路由表

- enum 上 `#[derive(Routes)]`，每个变体一个 `#[route("/a/:b")]`，第二个参数可以指定页面类型，不写就用变体名。derive 生成 `Routes`、`FromStr`（`Err = RouteError`）和 `Display`。
- 路径里的 `:name` 对应同名字段，类型实现 `FromStr + Display + Clone + PartialEq`。不在路径里的字段是子路由，一个变体最多一个，类型也实现 `Routes`。嵌套用嵌套的 enum。
- 变体上的 `#[layout(L)]` 包住它的子路由，enum 上的包住这一层的每条路由。页面和布局都是 `Default + Into<Element>`，用 `T::default()` 构造。布局用 `Outlet` 渲染子路由。
- 匹配：逐段比较，静态段优先于参数段；同一模式下叶子优先于带子路由的变体；其余按书写顺序。参数的 `FromStr` 失败时试下一条。两条叶子的模式相同时编译报错。`FromStr` 忽略空段，参数值里不能有 `/`。

### 渲染

- 每个变体的输出包在各自的闭包组件里：换变体时重挂载，只换参数时保留。enum 级布局在变体之外，换这一层的变体时保留状态。
- 参数经 context 提供给下面的整棵子树，`use_param::<T>(cx)` 按类型取最近的值。变体级布局在参数之内，能读到；enum 级布局读不到本层的参数。参数用 newtype 区分。
- `Outlet` 从 context 取布局给它的子元素，没有时渲染空元素；它向下提供空的子元素，页面里多放的 `Outlet` 不会把自己再渲染一遍。

### 历史

- `Router::new(initial)` 把历史 `Vec<R>` 放在 `use_state` 里，初始路由只在挂载时读。它向下提供 `RouterHandle<R>`：当前路由、深度和一个 `use_callback` 做的导航回调。`State` 放不进 context，所以提供回调。
- `use_router::<R>(cx)` 取 handle：`route()`、`can_go_back()`、`push`、`replace`、`back`、`push_path`。push 和当前路由相同时不入栈，栈底 `back()` 不动。导航写在按键处理和 effect 里，不在渲染时调用。
- 应用自己绑返回键。触发导航的键也会送到按下时已注册的其他按键处理。页面返回时重挂载。获得焦点的组件卸载后焦点为空，新页面的 `auto_focus` 生效。

### 目录路由

- `pub mod routes { eink_rs::router::routes!("src/routes"); }` 生成同样的嵌套 enum 和页面模块。路径相对 `CARGO_MANIFEST_DIR`，宏写在模块目录与该路径一致的内联模块里，包要有 `cargo::rerun-if-changed` 的 build.rs，展开时检查 `OUT_DIR`。
- 每层生成 `pub enum Route`：
  - `foo.rs` 是 `/foo` 的页面 `foo::Foo`；路径段用 kebab-case，`order_book.rs` 对应 `/order-book`，和 clap 的命令名一致。
  - `index.rs` 是本层 `/` 的页面，根目录也可以有。
  - `foo/` 生成子模块和 `Foo { route: foo::Route }`。
  - `[pair]/` 生成 `#[path = "[pair]"]` 的子模块、`Pair(pub String)` newtype 和 `Pair { pair: pair::Pair, route: pair::Route }`，页面用 `use_param::<super::Pair>(cx)`。rustc 和 rust-analyzer 都按 `#[path]` 找到 `[pair]/` 下的文件。
  - `_layout.rs` 的 `_layout::Layout` 是本层的 enum 级布局。
- 报错带路径：名字不是小写 snake_case，`self`、`super`、`crate`，`mod.rs`，`[pair].rs`（动态段只能是目录），`index/` 目录，`_layout.rs` 以外的下划线开头的条目，同名模块（`foo.rs` 与 `foo/`、`pair.rs` 与 `[pair]/`），一层两个动态目录（后一个匹配不到），`[route]/`（和子路由字段重名），没有页面和子目录的目录。

### argv

`cli` 和 `router` 都开时，`RouteArgs<R>` 可以 flatten 进根 `Parser`：每个位置参数是一个路径段，flag 可以夹在中间，没有词时是根 index。匹配不上时报 clap 的 `InvalidValue`，退出码 2。根上的子命令优先；这时 `route` 取根 index，所以有子命令时路由 enum 要有 index。clap 用新建的 `Command` 格式化这条报错，用法行显示包名，不是 argv[0]。

### 测试

`tests/router.rs` 测解析和打印、历史、布局与页面的生命周期、参数，以及开 `cli` 时的 argv、退出码和子命令优先。`macros` 的单元测试覆盖路径解析、匹配顺序、derive 的报错和目录规划的各条报错。`examples/file-routes/tests/routes.rs` 测目录生成的 enum 和两层布局的渲染。

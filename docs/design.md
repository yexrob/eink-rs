# eink-rs 设计

eink-rs 建在 [ink-rs](https://github.com/yexrob/ink-rs) 上，有两部分：

- `ui`：移植 Ink UI 2.0.0 的 13 个组件和 `useSpinner`。
- `cli`：参照 Pastel 4.0.1 的 CLI 框架，命令用 clap 声明，由组件渲染。

设计怎么来的见 [decisions/](decisions/README.md)。

## 结构

- workspace 的根 package 是 `eink-rs`。proc macro 在 Rust 里要独立成 crate：成员 `macros/`（`eink-rs-macros`）放 `Commands` derive 和 `commands!`。成员 `examples/cli-routes/` 是路由 B 的示例包，不发布。
- `ui` 只依赖 ink-rs。feature 默认都不开：`cli` 加 clap 和 derive，`completion` 加 clap_complete，`tokio` 加异步入口并开 ink-rs 的 `tokio`；后两个都带上 `cli`。只用组件的项目不编译 clap。
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

参照 Pastel 4.0.1，只借概念：命令即组件、包裹、index 与 default 命令、别名。help 格式、`-v` 和 Pastel 的测试不对齐。命令有两种写法：路由 A 用 clap 的 derive 手写子命令 enum，路由 B 用 `commands!` 从目录生成同样的 enum。

### 写法

- 命令的 `Args` 结构体同时 derive `Props`，结构体就是组件的 props，`From<Args> for Element` 来自 ink-rs。
- 子命令 enum 上 `#[derive(Commands)]` 生成 `From<Enum> for Element`，每个变体转换它唯一的字段。嵌套的组是持有另一个 `Commands` enum 的变体。泛型 enum、单元变体、具名字段和多字段的变体报错。
- 顶层 `Parser` 也是组件。Pastel 的 `_app`、index 命令和 default 命令写在它的渲染函数里：外面包一层，里面 `match` 子命令，没有专门的 API。
- 名称、版本、描述用 clap 的 `#[command(version, about)]` 从 `Cargo.toml` 读。`eink_rs::cli` 重新导出 `clap`，用户不必自己对齐版本。

### 路由 B：目录即命令

- `pub mod commands { eink_rs::cli::commands!("src/commands"); }`：路径相对 `CARGO_MANIFEST_DIR`，宏写在模块目录与该路径一致的内联模块里。宏展开成裸的 `pub mod` 和各层的类型，rustc 和 rust-analyzer 按宏调用处的模块找文件，不用 `#[path]`。
- 每个 `.rs` 文件是一个命令。模块名是文件名，类型名是它的 PascalCase：`super_deploy.rs` 里是 `SuperDeploy`，clap 的命令名为 `super-deploy`。关键字用原始标识符，`type.rs` 是 `r#type`。描述、别名、隐藏写在文件里的结构体上；命令名只来自文件名，clap 不读结构体上的 `name`。
- 每层生成 `pub enum Command`，derive `Subcommand`、`Clone`、`PartialEq` 和 `Commands`。子目录是命令组：
  - 没有 `index.rs` 时，变体带 `#[command(subcommand)]`，不带子命令时 clap 打印 help，退出码 2。
  - 有 `index.rs` 时另生成 `Group`：flatten `index::Index`，加 `Option<Command>`，开 `args_conflicts_with_subcommands`。index 的文档注释和别名成为组的描述和别名，不带子命令时渲染 index。
- 根 `Parser` 手写，同路由 A。根目录的 `index.rs` 报错：Pastel 的根 index、`_app` 和 default 都写在根组件里。
- 报错用 `compile_error!` 指向宏调用处，消息带路径：文件名不是小写 snake_case；`self`、`super`、`crate`、`clap`（生成的模块导入 `clap`）；`mod.rs`；`foo.rs` 与 `foo/` 并存；没有命令的目录。Pastel 不检查，同名时静默覆盖。非 `.rs` 文件和隐藏条目跳过，条目按名字排序。
- 生成的项用 `#[doc = concat!(..)]` 写文档：clap 不读表达式形式的文档，命令的描述留给用户的类型。
- 重编译：稳定版的宏察觉不到新增文件，用户的包要有一行 `cargo::rerun-if-changed=src/commands` 的 build.rs。宏展开时检查 `OUT_DIR`，包没有 build script 时报错；build script 的输出查不到。不支持 nightly 的 `tracked::path`。

### 入口与退出码

- `run::<C>()` 解析 `std::env::args_os()`，`run_from::<C>(argv)` 解析给定的参数，用 `Options::default()` 渲染，等到退出。`tokio` 下的 `run_async`、`run_async_from` 用 ink-rs 的 tokio 驱动，要在 local runtime 或 `LocalSet` 里运行。同步和异步共用解析与退出处理，只有渲染和等待不同。
- 退出码：clap 的错误打印后用它的码，help、version 为 0，用法错误为 2；`exit_with_error` 和其他 ink-rs 错误向 stderr 打印 `Error: {error}`，返回 1；`exit_with` 的值是 `ExitCode` 时用它；其他退出值、`exit()` 和应用自行结束返回 0；组件 panic 用原 payload 继续 unwind，保留 101。
- `completion` 下，`run` 系列给 `C::command()` 加 `completions <SHELL>`，匹配时向 stdout 写补全脚本，返回 0。用户的命令不能再叫 `completions`。

### 测试

`parse_from::<C>(argv) -> Result<Element, clap::Error>` 是测试辅助：元素交给 ink-rs 的 `testing::render` 或 `testing::mount`，不加 `completions`。`tests/cli.rs` 只测 derive 分派（含嵌套组和别名）、顶层组件的 default 命令、五种退出码和 `completions` 子命令。路由 B 的规划是纯函数，`macros` 的单元测试覆盖名字转换和各条报错；`examples/cli-routes/tests/routes.rs` 覆盖目录生成的分派和 index 组的元数据。Pastel 的测试比对 commander 的 help 文本，不移植。

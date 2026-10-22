# eink-rs 设计

eink-rs 建在 [ink-rs](https://github.com/yexrob/ink-rs) 上，有四部分：

- `ui`：移植 Ink UI 2.0.0 的 13 个组件和 `useSpinner`。
- `cli`：参照 Pastel 4.0.1 的 CLI 框架，命令用 clap 声明，由组件渲染。
- `router`：参照 React Router 的应用内路由，路由表是 enum，可以从目录生成。
- `markdown`：Markdown 渲染和模型回复的流式显示，定稿的段进 `Static`，尾部重绘。

设计怎么来的见 [decisions/](decisions/README.md)。

## 结构

- workspace 的根 package 是 `eink-rs`。proc macro 在 Rust 里要独立成 crate：成员 `macros/`（`eink-rs-macros`）放 `Commands`、`Routes` 两个 derive 和 `routes!`。成员 `examples/file-routes/` 是目录路由的示例包，`examples/ship/` 是组合 ui、cli 和 router 的部署控制台示例包，`examples/quill/` 是内联渲染的编码助手示例包，都不发布。
- `ui` 只依赖 ink-rs。feature 默认都不开：`cli` 加 clap 和 derive，`router` 加路由和它的宏，`completion` 加 clap_complete，`tokio` 加异步入口并开 ink-rs 的 `tokio`；后两个都带上 `cli`。`markdown` 加 pulldown-cmark，`highlight` 加 syntect 和 two-face，`markdown-highlight` 合并 `markdown` 和 `highlight`。只用组件的项目不编译 clap。
- 依赖 crates.io 上的 ink-rs，关掉它的默认 feature，开 `macros`。缺的通用原语加进 ink-rs，不在这里绕过。

## ui

### 三层

每个组件一个模块，最多公开三样：

- **状态 hook**：有状态的组件公开 `use_x_state(cx, ..)`，不含按键绑定和渲染。状态更新写成纯函数，用 `State::get` 和 `State::set` 应用。回调在按键处理里调用，参数取这次更新后的值。三个文本输入框共用 `use_input_state` 和 `reduce_input`；Select、MultiSelect 各有 `use_select_state`、`use_multi_select_state`；Spinner 公开 `use_spinner`。ConfirmInput 和其余展示组件没有状态。
- **组件**：props 结构体派生 `Props`，渲染函数私有。交互组件有 `is_disabled`，为 false 时用 `use_input` 读键。焦点交给应用：用 `use_focus` 算出 `is_disabled`，或把 `Button`、`TextInput`、`Select` 放进 `FocusRing`，见「焦点环」。
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
| `accent` | 品红 | ProgressBar、Badge、主按钮 |

Ink UI 的主题是一组函数，返回任意 Box/Text 属性，用 deepmerge 合并。这里的主题是数据：只能改列出来的字段，换来类型检查和 `PartialEq`。

### 受控组件

`TextInput` 和 `Select` 的 `value` 为 `Some` 时是受控的，忽略 `default_value`。宿主（bingo 渲染插件返回的元素树）决定框里下一刻是什么。

- TextInput 的按键用 `reduce_input` 算出提议，调用 `on_change`，组件不应用它。同一批输入的按键之间不重渲染，处理器读到上一次渲染的 props，所以组件保留一份乐观值：每个键在它上面算，每次渲染按 `value` 校准。`value` 等于提议是采纳；等于上一帧的值是拒绝，光标留在原处；其他值是替换，光标到末尾。
- Select 的选中标记跟随 `value`，焦点行和窗口归组件，初始焦点在 `value` 所在的项。`on_select` 在每次 Enter 时调用，含选中当前值的 Enter（权限确认要它）；`on_change` 的含义不变。`number_hotkeys` 打开时数字 1–9 聚焦并选中对应的项，行首画 `1.`。
- `SelectOption::description` 画在标签下一行，缩进、变暗；窗口按选项计数。MultiSelect 共用这段画法。

### 焦点环

ink-rs 的焦点全应用一份，按注册顺序编号，不能限定子树。`FocusRing` 用 `provide`、`use_context`、`use_state`、`use_layout_effect`、`use_input` 自建，不调用 `use_focus`；和 ink-rs 的焦点混用时 Tab 处理两次。环不加布局节点，子节点按宿主的布局排列。

- 成员调用 `use_ring_focus(cx, RingItem)` 加入它所在的环（嵌套时取内层），返回自己是否持有按键：环在活动，它获焦。环外返回 true。`Button`、`TextInput`、`Select` 用自己的 `id` 加入；宿主的组件用这个 hook 加非控件的停靠点（窗格的关闭标记、标签）。
- 成员按字符串 key 寻址；没有 key 的成员每次挂载生成一个 `eink:ring:` 开头的 key，宿主不能指定它。
- 顺序按屏幕位置：移动时用 `measure_element` 量每个成员，按 y 排，y 相同时按 x 排。渲染之间增删的控件落在它在屏幕上的位置。
- 每次移动是一个提议：Tab、Shift+Tab、方向键、自动聚焦和焦点成员离开产生提议，交给 `on_focus`。`focused` 是 `RingFocus::Managed` 时环自己应用；是 `Key` 或 `None` 时等宿主改 `focused`，宿主可以采纳、改道或忽略，改动可以在异步检查之后。
- 焦点成员离开树时，按它末次测得的位置提议其后的成员，没有时取它前面的，环空时提议 `None`。环在渲染时量一次成员，赶在协调分离被删节点的引用之前。
- 环变成活动（挂载时 `is_active`，或 `is_active` 变成 true）且没有焦点时，提议第一个 `auto_focus` 成员；没有这样的成员时不提议，Enter 不作用在作者没标记的按钮上。环在活动、没有焦点时挂载的 `auto_focus` 成员得到同样的提议。
- `arrows` 打开时，焦点成员不占用方向键的情况下方向键走环。成员用 `Claims` 声明：Select 占用方向键，TextInput 占用文字和方向键。
- TextInput 获焦时，别的成员的字母、数字热键停用，打 `y` 不会按下按钮。
- 同一时刻一个站点持有按键，宿主用环的 `is_active` 和输入框的 `is_disabled` 保证。门控读上一次渲染的 props，离开站点的 Esc 不会在同一批输入里到达输入框。
- `Button` 画成 `[ label ]`，获焦时反色；括号和热键分隔符在 `ButtonTheme` 里，主按钮取 `accent`，次按钮无颜色。环外的 Button 视为获焦，Enter 和空格按下环外的每一个 Button，应用用 `is_disabled` 门控。

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

`cli` 和 `router` 都开时，`RouteArgs<R>` 可以 flatten 进根 `Parser`：每个位置参数是一个路径段，flag 可以夹在中间，没有词时是根 index。匹配不上时报 clap 的 `InvalidValue`，退出码 2。clap 拿第一个词和紧跟 flag 的词去匹配根上的子命令，匹配上就执行子命令，这时 `route` 取根 index，所以有子命令时路由 enum 要有 index。其余的词即使和子命令同名也是路径段。clap 用新建的 `Command` 格式化这条报错，用法行显示包名，不是 argv[0]。

### 测试

`tests/router.rs` 测解析和打印、历史、布局与页面的生命周期、参数，以及开 `cli` 时的 argv、退出码和子命令优先。`macros` 的单元测试覆盖路径解析、匹配顺序、derive 的报错和目录规划的各条报错。`examples/file-routes/tests/routes.rs` 测目录生成的 enum 和两层布局的渲染。`examples/ship/tests/ship.rs` 测每条路由能渲染和 `status` 的退出码。`examples/quill/tests/quill.rs` 测一轮带审批的交互和 `exec` 的退出码。

## markdown

pulldown-cmark 0.13 解析，开表格、删除线、任务列表、脚注和 GFM 的提示块。讨论见 [016](decisions/016-markdown-stream.md)、[018](decisions/018-markdown-stream-formal.md) 和 [025](decisions/025-list-fence.md)。

### 三层

- `MarkdownStream` 是 headless 的模型：`push` 追加文本，`finish` 结束，`tick` 匀速揭示，`take_pieces` 取定稿的段。它不依赖 UI，测试和压测用它。
- `use_markdown_stream(cx, options, on_piece)` 把模型放进组件，返回 `MarkdownStreamHandle`：`sender(app)` 给出线程安全的 `MarkdownSender`，`MarkdownTail` 渲染尾部，`progress` 给出计数，`set_pacing` 和 `reset` 改设置、开始新的一次流。
- `Markdown` 组件渲染整篇文档。主题 `MarkdownTheme` 从 `Palette` 生成，规则同 `ui`。

### 定稿

段在输出时定稿：之后到达的文本不改变它的渲染。

- 围栏代码块：开头一段，显示语言标签；每个写完的内容行一段；结束围栏不输出。
- 顶层列表：每个顶层项一段，下一项或下一个顶层块开始时输出。一项按单独的文档渲染，序号取列表首项的序号加项的下标。
- 含代码块的列表项在代码块那一行切开：之前的文字一段，代码块按行定稿，之后的部分剥掉项的内容列，在这一层按块定稿，其中含代码块的子项按这条规则切开，层数不限。段记下外层各项的标记，渲染时套上标记列；项的标记画在它第一个看得见的段上。
- 引用里的代码块、首行是代码块的任务项、制表符缩进的项按整项定稿。
- 表格整表一段；其余顶层块各一段。
- 每段自带前面的空行：流的第一段为 0，顶层块之前为 1，列表项之间看源码里有没有空行。应用按顺序叠起段，不加间距，结果和整篇交给 `Markdown` 相同。
- 例外：块之后才到的链接引用定义作用不到这个块。脚注引用用正文的颜色，后到的定义不改变已输出的引用。

### 尾部

尾部是最后一个未定稿的块，加上等高亮的代码行。`MarkdownTail` 的 props 是模型的计数，计数不变时跳过重渲染。

- 补齐（`settle`，默认开）：没换行的半行按写完的样子显示，未闭合的行内标记临时补上结束符，不写进源码；说不准的块标记等所在的行写完后显示。
- 代码块的尾部画最后一屏的行。ink-rs 的非全屏模式在动态区高过屏幕时从顶部裁掉，多画的行看不见。
- 缩进小于所在项的半行，在代码块里或空行之后，写完之前画在项外；其余情况等这一行写完，看它是不是懒续行。
- 每帧的开销随尾部的长度增长。长表格、长段落和按整项定稿的项写完前留在尾部，是已知限制，数字见 018 的结果。列表项里的代码块和顶层的开销相同，数字见 025 的结果。

### 发送和唤醒

- 模型放在 `State` 里。生产者在任意线程经 `MarkdownSender::push` 发文本，一批合成一次 `AppHandle::update`：UI 线程处理这批之前到达的文本进同一批。
- `sender(app)` 用调用方的 `AppHandle`。发送端在，应用不会自然结束；`finish` 或丢掉全部发送端后可以结束。hook 不持有 `AppHandle`，空闲时不留唤醒源。
- `reset` 换一个空的模型，代号加一，旧的发送端发来的文本被忽略。
- 匀速揭示（`pacing`）打开时，hook 的动画时钟在有积压时运行，每帧揭示积压的一份，在 `pacing` 给的时间内赶上；关掉时文本在到达的那一帧显示。
- `on_piece` 在 UI 线程、绘制前调用，这时模型被借用：回调里不能调用这个流的 handle。

### 高亮

高亮在 `src/highlight.rs`，`highlight` feature 编译它，Markdown 和其他组件共用。它用 syntect 的纯 Rust 正则后端，语法和主题取 two-face（bat 的一套）。默认主题 `ansi`，颜色跟随终端的 16 色。语言按代码块的标记查，或按文件名查（扩展名和 `Makefile` 这类全名，查不到时把扩展名当标记）。块 id 取自全局的原子计数器，不同组件的块不冲突。

- 流里顶层代码块的行交给一个后台线程高亮，结果回来后这一行定稿。语法第一次编译不占 UI 线程。
- 不认识的语言用代码块的纯色；它的头几行等后台线程认出语言后输出。
- `Markdown` 组件和列表项、引用里的代码块在 UI 线程高亮，按行缓存。
- 超过 1000 字节的行用主题的默认样式，不推进解析状态，下一行从它前面的状态开始，和 Shiki 相同。syntect 处理 1000 字节的一行不超过 6 ms，耗时增长比长度快。两条路径共用按行高亮的函数，流式和整篇的结果一致。
- 流被丢弃后，后台线程跳过它排着的行。

### 测试

`tests/markdown.rs` 有三个测试：

- 71 篇文档按 5 种切法分块推送，匀速开和关，80 和 40 列，叠起来的段和整篇渲染的结果相同。
- 列表项里的 500 行代码块逐行推送，每行之后尾部不超过两行。
- hook 在非交互模式下挂载，生产者线程推完一篇回答：输出和整篇相同，结束后没有渲染。

`examples/markdown-stress.rs` 不经终端，按真实的帧节拍逐帧计时，给出每帧的分位数。

# Ink UI 移植：结构、主题与分工

- 日期：2026-10-21
- 状态：已实施
- 提交：`26e4b2b` feat: add the crate skeleton with palette and theme lookup；`0288616` docs: add the worker briefs for the Ink UI port；`2eae235` feat(ui): port Ink UI's display components；`f276743` feat(ui): port Ink UI's inputs；`6f7dc4c` feat(ui): port Ink UI's Select and MultiSelect
- Brief：[inkui-display](briefs/inkui-display.md)、[inkui-inputs](briefs/inkui-inputs.md)、[inkui-select](briefs/inkui-select.md)

## 背景

[001](001-scope-and-rules.md) 之后，Claude 给出 Ink UI 移植的六节方案：仓库结构、每个组件的三层、主题、和 Ink UI API 不同的两处、移植做法、分工。用户提醒「这个你仓库不只是ink ui, 还有后续的那个框架 注意哈」，Claude 把第 1 节改成 `ui`、`cli` 两部分并列，问用户第 1、3、4 节是否同意、要不要推送 001、要不要把 ink-rs 的项目 memory 复制到 eink-rs。用户答「ok」。

Ink UI 2.0.0 的调研结论见 ink-rs 的 [056](https://github.com/yexrob/ink-rs/blob/main/docs/decisions/056-upper-framework.md)：13 个组件、`useSpinner` 和一套主题，约 3100 行；主题是返回 Box/Text 属性的函数；状态 hook 不导出；三个输入框是同一份代码复制三份；源码里有一批可见的 bug。

## 决策

以下由 Claude 提出，用户同意（「ok」）：

- **结构**：一个 crate，根 package 是 `eink-rs`。组件在 `eink_rs::ui`，框架在 `eink_rs::cli`，`cli` feature 带 clap，shell 补全放 `completion` feature，异步命令用 ink-rs 的 `tokio` feature；默认开哪些 feature 在做框架时定。proc macro 和 build.rs 用的库在 Rust 里要独立成 crate，需要时加成员：路由 A 要 derive 时加 `macros/`，做路由 B 时加 `build/`。
- **三层**：每个组件公开一个 headless 状态 hook、一个派生 `Props` 的组件和一个主题结构体。交互组件保留 `is_disabled`，焦点交给应用。
- **主题**：`Palette` 是颜色 token；每个组件一个主题结构体，实现 `From<&Palette>`；组件读主题时取最近的同类型 context，没有就用最近的 `Palette` 生成，再没有就用默认 `Palette`。主题是数据，不是函数：只能改列出来的字段，换来类型检查和 `PartialEq`。
- **列表**：每个子节点算一项，编号按子节点计数。一项里放多个元素时用 `View` 包起来。
- **图标**：用 Unicode 字符（✔ ✘ ⚠ ℹ ❯ ─），不做 figures 的非 Unicode 回退。
- **移植**：Ink UI 的 103 个测试移植到 ink-rs 的 `testing`，期望值用元素树构造，比较带颜色的帧。056 列的 bug 移植时修掉，改动的期望写明原因；StatusMessage 和 Alert 标成 `test.failing` 的测试按测试写的预期实现。Select 的选项变化用 `use_layout_effect` 重置，ProgressBar 用 `use_box_metrics` 量宽，Spinner 用 `use_animation`，输入框光标按字素移动。
- **分工**：P0 由主 agent 做骨架、文档、许可证、CI、`Palette` 和主题查找；P1 派三个 Codex 并行；主 agent 集成。

以下由 Claude 在上面的授权下定，写进 brief：

- `Palette` 的 token 和默认值取自 Ink UI 默认主题里的颜色：`primary` 蓝（焦点、Spinner）、`info` 蓝、`success` 绿、`warning` 黄、`error` 红、`accent` 品红（ProgressBar、Badge）。焦点色和 info 同为蓝色，分成两个 token，换焦点色不影响状态提示。
- 回调在按键处理里调用，参数取这次更新后的值，不用 effect 比较前后值。Ink UI 的 `onChange` 放在 effect 里，回调或选项换了引用会重复触发，这样改掉。
- Spinner 默认 `dots`（cli-spinners 的帧和 80 ms 间隔），可以传自定义帧和间隔；不移植 cli-spinners 的整张表。Ink UI 的文档没有列出 `type`。
- Select、MultiSelect 的选项值用泛型 `T`，标签是 `String`。Ink UI 的值是字符串。
- 输入框按住 Ctrl 或 Meta 的键不插入字符；Enter 有补全时把补全接在值的末尾，光标移到末尾，`on_change` 和 `on_submit` 拿到同一个值。
- 测试：移植 Ink UI 的测试，每个修掉的 bug 加一个测试；不另加单元测试。

## 备选与取舍

- **拆成 `eink-ui`、`eink-cli` 两个 crate**：多一套版本和发布。只用组件的项目不开 `cli` feature 就不依赖 clap，隔离效果一样。
- **主题保留函数**：要把每个样式函数包成 `Callback` 才能比较，函数能返回任意属性，组件改结构时主题会悄悄失效。不选。
- **移植 cli-spinners 全表**：80 多种动画，上千行数据，Ink UI 文档没有提到。用户需要时传自定义帧。
- **选项值用 `String`**：和 Ink UI 一致，但 Rust 里选枚举值要自己来回转换。泛型参数靠 `#[derive(Props)]` 支持（ink-rs 059 的测试覆盖了泛型 props）。

## 方案

- **P0（主 agent）**：
  - `Cargo.toml`：依赖 `ink-rs = "0.1.2"`，关掉默认 feature，开 `macros`。
  - `src/lib.rs`、`src/ui/mod.rs`、`src/ui/theme.rs`：`Palette` 和 `use_theme`。
  - `tests/support/mod.rs`：用 `testing::render` 的设置（100 列、基本色）渲染期望树。
  - README、`docs/design.md`、MIT/Apache 许可证、Ink UI 的 MIT 许可（`LICENSE-INK-UI`）、三平台 CI、`.gitignore`。
- **P1（三个 Codex 并行）**，每个一个 worktree，改的文件不重叠，`src/ui/mod.rs` 的模块声明在集成时合并：
  - [inkui-display](briefs/inkui-display.md)：Badge、StatusMessage、Alert、Spinner 和 `use_spinner`、ProgressBar、UnorderedList、OrderedList，Ink UI 测试 20 个。
  - [inkui-inputs](briefs/inkui-inputs.md)：TextInput 内核、EmailInput、PasswordInput、ConfirmInput，Ink UI 测试 60 个。
  - [inkui-select](briefs/inkui-select.md)：Select、MultiSelect，Ink UI 测试 23 个。
  - 每个 worker 跑自己的测试文件、fmt、clippy、`cargo doc`，不跑全量。
- **集成（主 agent）**：合并补丁和 `src/ui/mod.rs`，review，跑全量，更新 `docs/design.md` 和 README，补本记录的结果。

## 结果

三个 Codex 按 brief 完成，测试共 113 个：Ink UI 的 103 个，加 Spinner 换帧 1 个、输入框 5 个、Select 4 个。主 agent 集成后跑了全量 `cargo test`、fmt、clippy `--all-targets -D warnings` 和 `cargo doc -D warnings`，都通过。

review 发现的问题和处理：

- **UnorderedList**：OrderedList 跳过 `None` 转成的空子节点，UnorderedList 却给它画了标记。主 agent 改成两者都跳过。`docs/design.md` 的列表规则同步改了。
- **输入框的状态 hook**：worker 给 TextInput、EmailInput、PasswordInput 各加了一个只转调 `use_input_state` 的 hook。按「如非必要，勿增实体」删掉，三个组件直接用 `use_input_state`。
- **`highlight_text`**：worker 写成 `Option<String>`，调用处要写 `.highlight_text(Some("l".into()))`。改成 `String`，空串不高亮，和 `placeholder` 一致。

worker 在 brief 留给它的地方做的选择：

- 输入框加了共用的 `InputTheme`（光标反色、占位符和补全变暗、掩码字符），ConfirmInput 的两个提示放进 `ConfirmInputTheme`。
- Select 的导航状态里存一份选项。选项变短时，layout effect 重置之前的那次渲染还拿着旧窗口，存一份避免越界。
- `SelectTheme` 和 `MultiSelectTheme` 用一个私有宏生成，字段、默认值和行样式转换共用。

worker 报告的 ink-rs 行为：ink-rs 把一次读到的字节作为一块交给 `use_input`，`"\r\r"` 是一段文本，不是两次 Enter；连续按 Enter 的字节如果一次读到，TextInput 会把它们当文本插入。测试里改用方向键序列隔开按键。要不要在 ink-rs 里按键拆分，另开记录和用户讨论。

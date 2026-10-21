# eink-rs 设计

eink-rs 建在 [ink-rs](https://github.com/yexrob/ink-rs) 上，有两部分：

- `ui`：移植 Ink UI 2.0.0 的 13 个组件和 `useSpinner`。
- `cli`：参照 Pastel 的 CLI 框架，在 Ink UI 移植之后设计，设计定下后在本文加一节。

设计怎么来的见 [decisions/](decisions/README.md)。

## 结构

- 一个 crate，根 package 是 `eink-rs`。proc macro 和 build.rs 用的库在 Rust 里要独立成 crate，需要时加成员：路由 A 要 derive 时加 `macros/`，做文件路由 B 时加 `build/`。
- `ui` 只依赖 ink-rs。`cli` 放在 `cli` feature 下，依赖 clap；shell 补全放 `completion` feature；异步命令用 ink-rs 的 `tokio` feature。
- 依赖 crates.io 上的 ink-rs，关掉它的默认 feature，开 `macros`。缺的通用原语加进 ink-rs，不在这里绕过。

## ui

### 三层

每个组件一个模块，公开三样：

- **状态 hook**：`use_x_state(cx, ..)`，不含按键绑定和渲染。状态更新写成纯函数。回调在按键处理里调用，参数取这次更新后的值。
- **组件**：props 结构体派生 `Props`，渲染函数私有。交互组件有 `is_disabled`，为 false 时用 `use_input` 读键。焦点交给应用：用 `use_focus` 算出 `is_disabled`。
- **主题**：`XTheme` 结构体，字段是颜色、字符、样式开关和间距，实现 `From<&Palette>`。布局结构（方向、图标不收缩）写在组件里，不进主题。

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

- **列表**：Rust 写不出 `OrderedList.Item`，也不按类型识别子节点。列表的每个子节点算一项，编号按子节点计数。一项里放多个元素时用 `View` 包起来，嵌套列表写在这个 `View` 里。
- **图标**：用 Unicode 字符（✔ ✘ ⚠ ℹ ❯ ─），不做 figures 的非 Unicode 回退。

### 测试

`tests/` 下每个组件一个文件，移植 Ink UI 的测试。组件用 `ink_rs::testing::render` 渲染，期望值是 ink-rs 的元素树，用 `tests/support` 的 `frame` 按同样的设置（100 列、基本色）渲染，比较带颜色的帧。修掉的 Ink UI bug 各有一个测试。

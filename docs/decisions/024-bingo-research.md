# 调研 bingo 需要的 eink-rs 改动

- 日期：2026-10-22
- 状态：已实施
- 提交：无
- Brief：[bingo-research](briefs/bingo-research.md)；需求：[bingo-release-markdown](briefs/bingo-release-markdown.md)、[bingo-composer](briefs/bingo-composer.md)、[bingo-controls](briefs/bingo-controls.md)、[bingo-code](briefs/bingo-code.md)

## 背景

bingo 是建在 ink-rs 和 eink-rs 上的编码 agent。它的会话 bingo-refactor-98 发来 7 份需求 brief，原文存在 `briefs/bingo-*.md`。eink-rs 的四份是发布 markdown 模块、Composer、受控的 Button、Input、Select 和焦点环、Code 组件；ink-rs 的三份见 ink-rs 的记录 071，那里列了对方转述的顺序和用户在这个会话里的确认。

发布这一项，主 agent 查了现状：

- `tokio = ["cli", "ink-rs/tokio"]`，开 `tokio` 会带进 clap。`cfg(feature = "tokio")` 用在 `src/cli/mod.rs` 的 `run_async` 和 `run_async_from` 两处，没有别处。
- markdown 模块在 main 上，`v0.2.1` 之后有 30 个提交。
- 需求说长代码块写完前留在尾部。这对顶层代码块不成立：顶层的代码块每写完一行定稿一段。留在尾部的是列表项里的代码块，018 里用户选了「保留按项定稿」。主 agent 用 `examples/markdown-stress.rs` 测 500 行 Rust 代码块（每帧 64 B，120 fps，release，`markdown-highlight`，ink-rs 0.1.5），p50 / p99 / max（ms）：

  | 位置 | p50 / p99 / max | 超过 8.33 ms 的帧 |
  |---|---|---|
  | 顶层 | 0.21 / 6.18 / 9.79 | 1 / 267 |
  | 列表项里 | 6.52 / 14.93 / 59.38 | 102 / 291 |

## 决策

- 用户定：
  - 并行和计划见 ink-rs 的记录 071：各项并行调研，方案一轮谈定，每项一个 worker、一个 worktree，并行派出。
  - 列表项里的长代码块：主 agent 列了三个方案（不改、按行定稿、限制尾部高度）。用户问「这三个分别收益是什么」「极限性能和不破坏体验的情况应该选哪个？」，主 agent 答按行定稿：每帧开销和顶层相同，守住「各段拼起来和整篇渲染相同」时体验不变；限制高度省 ink-rs 一侧，Markdown 的解析每帧随长度增长。用户对「按这个计划开工吗？」答「按行定稿进 0.3.0」。
- 主 agent 提出、用户随计划确认：
  - 下一版是 0.3.0：`tokio = ["ink-rs/tokio"]`，`run_async` 要求 `cli` 和 `tokio`；开 `tokio` 不开 `cli` 调用 `run_async` 的项目编译失败，0.x 下升次版本。
  - 0.3.0 带 markdown 模块和列表项里代码块的按行定稿，等按行定稿做完后发。
  - 023 的图片不进 0.3.0。
  - Composer、控件、Code 和图片进 0.3.0 之后的版本。
- Claude 按用户的做法定（文档调研交给 Sonnet 或 Codex，不用 Opus）：
  - 派四个 Sonnet 子 agent 调研列表项里的代码块、Composer、控件和 Code。
  - `tokio` feature 的拆分由主 agent 做，不调研。023 有调研和方案稿，不重复调研。

## 方案

四个 Sonnet 子 agent（general-purpose），不改文件，报告写到 `<scratchpad>/research/bingo/<topic>.md`，交回五行以内的摘要。prompt：

> Research brief: `<repo>/docs/decisions/briefs/bingo-research.md`. Your topic: `list-fence`. `<repo>` is `<repo>`; `<scratchpad>` is `<scratchpad>`.

其余三个把 `list-fence` 换成 `composer`、`controls`、`code`。

- **list-fence**：现在的定稿规则；列表项拆段的设计和边界情况；其他把代码留在尾部的块；测试。
- **composer**：和现有 reducer 的关系；折行要不要 ink-rs 的原语；宽度；`use_cursor`；按键；受控的约定；quill 的 composer。
- **controls**：受控的做法；焦点环能否在 ink-rs 现有的原语上做；站点之间的按键；Button；Select 的补充。
- **code**：高亮线程的 API；`Code` 的位置和 feature；Markdown 是否复用它；diff 的解析、配色和跨行的高亮；折行和截断；2000 行的开销。

调研交回后，主 agent 把这四项的方案和 ink-rs 的三项合在一轮交给用户；谈定后各开实施记录。

## 结果

四份报告交回，在 `<scratchpad>/research/bingo/`。主 agent 把方案和 ink-rs 的三项合在一轮交给用户，用户的选择：

- 版本：「不拆 tokio，发 0.2.2 (Recommended)」。bingo 会话确认不需要拆：它自己开 ink-rs 的 `tokio`，eink-rs 开 `markdown` 和 `markdown-highlight`。主 agent 写编译检查确认 `MarkdownSender` 是 `Send + Sync`。v0.2.1 之后 `src` 的改动是新增。决策里「下一版是 0.3.0」「`tokio = ["ink-rs/tokio"]`」两条作废：下一版是 0.2.2，带 markdown 模块和列表项里代码块的按行定稿，`tokio` feature 不变。
- 原语：「加两个 (Recommended)」，`use_color_level` 和锚定节点的光标进 ink-rs 0.1.7（ink-rs 的记录 074），Code 和 Composer 用它们。
- 焦点顺序：「你问问bingo那个session 看看应该用哪个？」，结果见 026。

实施记录：[025](025-list-fence.md) 列表项代码块、[026](026-controls.md) 控件、[027](027-composer.md) Composer、[028](028-code.md) Code，以及 [023](023-image.md) 图片。027、028 和 023 等 ink-rs 的 074 合入后派出。

# `Code` 组件：行号和统一 diff

- 日期：2026-10-22
- 状态：已实施
- 提交：`53c0d41`、`6dd9107`
- Brief：[code](briefs/code.md)；需求：[bingo-code](briefs/bingo-code.md)

## 背景

bingo 渲染 Claude Code 的元素词表，其中有 `Code { source, language?, path?, startLine?, format: source | diff, wrap: wrap | truncate-end }`。工具行用它显示带行号的文件读取和统一 diff 形式的编辑。eink-rs 在 Markdown 里高亮代码，没有公开的组件。

调研报告（`<scratchpad>/research/bingo/code.md`）的结论：

- 高亮在 `src/markdown/highlight.rs`，`markdown-highlight` 编译它。后台线程的唤醒（`Wake`）、块 id 和语言查找绑定 Markdown，`Code` 用之前要泛化。语言按 token 查，没有按文件名查的入口。
- 实测 2000 行 Rust 代码块整篇渲染：高亮加渲染 195–230 ms，无语言 34 ms。
- 逐行高亮丢掉多行状态（块注释、字符串）。
- `Palette` 没有底色令牌，组件读不到色深；底色在 16 色终端降级后难看。
- Markdown 的代码块不能改走 `Code`：流式的各段拼起来要和整篇渲染逐字相同，代码块有自己的标签、缩进和逐行的段。

## 决策

- 用户定：
  - 「加两个 (Recommended)」：ink-rs 0.1.7 加 `Scope::use_color_level()`（ink-rs 的记录 074），`Code` 在真彩和 256 色下画底色，16 色和无色时靠 `+`、`-` 的前景色表意。
- 主 agent 在第四轮方案里提出、用户随方案确认：
  - 组件放在 `ui::Code`。新增共享的 `highlight` feature，保留 `markdown-highlight = ["markdown", "highlight"]`；不开 `highlight` 时 `Code` 用纯色，diff 可用。
  - `highlight.rs` 移到 `src/highlight.rs`，解除和 Markdown 的绑定。
  - diff 的解析自己写；每个 hunk 还原出旧文本和新文本，整块高亮；解析失败按纯代码显示，经 `on_fallback` 通知宿主。
  - 64 行以内在渲染线程同步高亮，超过 64 行的交给后台线程。宿主常把工具行放进 `Static`，纯色的第一版会留在滚动区。
  - Markdown 的代码块不改走 `Code`，两者共用高亮。
- Claude 按以上定细节：
  - 分两部分做，同一个 worker：第一部分是纯重构（移动 `highlight.rs`、`Lang::{Token, File}`、唤醒改成 `Arc<dyn Fn() + Send + Sync>`、全局原子块 id），主 agent 提交后，worker 做第二部分。第一部分合入在前，list-fence 和 023 在它之上 rebase，冲突限于 `use` 路径。
  - 不加 `on_highlighted` 回调：宿主按 64 行的阈值放小源码。
  - 截断的 hunk 回退成纯代码，不做容错解析。
  - `start_line` 对源码和回退有效；diff 的行号取自 hunk 头。
  - `CodeTheme` 按主题规则从 `Palette` 来：`added` 取 `success`，`removed` 取 `error`，底色缺省为暗绿、暗红的 `Rgb`。
  - `Wrap` 用 `TextWrap::Hard` 还是 `TextWrap::Wrap` 由 worker 核对哪个保留行首缩进后定，报告说明。
  - 加示例 `examples/code.rs`，`required-features = ["highlight"]`；主 agent 测 2000 行源码。

## 备选与取舍

- **`markdown::Code`**：`Code` 被迫依赖 `pulldown-cmark`。
- **独立的 `code` feature**：多一个 feature，没有收益。
- **现成的 diff crate（diffy 等）**：面向完整的补丁，半截输入失败。
- **逐行高亮 diff**：hunk 里的多行注释颜色错。
- **固定的 `Rgb` 底色，不读色深**：16 色终端降级后难看。
- **Markdown 代码块改走 `Code`**：行结构不同，各段拼起来和整篇渲染有对齐风险；和 list-fence、023 撞 `render.rs`。

## 方案

一个 Codex worker，worktree `wt/eink-code`，基于 main。ink-rs 0.1.7 发布前，worktree 的 `Cargo.toml` 用 `[patch.crates-io]` 指向 ink-rs 合入 074 之后的快照，提交前去掉。worker 不提交，不改文档。brief 见 [code](briefs/code.md)。

- 第一部分：移动 `highlight.rs`，泛化唤醒、语言和块 id，feature 改名。验收：fmt；clippy 三组 feature；`--lib --all-features`；`--features markdown-highlight --test markdown`。主 agent review 后提交，之后用 `codex exec resume` 让 worker 做第二部分。
- 第二部分：新增 `src/ui/code.rs`、`src/ui/diff.rs`、`tests/code.rs`、`examples/code.rs`。测试：行号栏的宽度和对齐；两个 hunk 的 diff；窄宽度下的 `Wrap` 和 `TruncateEnd`；坏 diff 的回退和 `on_fallback` 调用一次；开 `highlight` 时 hunk 还原保留多行注释的颜色；`Basic` 不画底色。验收同第一部分，加新的测试文件，开和不开 `highlight` 各一次。

主 agent 集成：review，提交，全量测试，测 2000 行源码的渲染开销，`docs/design.md` 的 ui 和高亮两节，在 tmux `ink-rs` 会话里跑示例核对，补结果，删 worktree。进 0.2.2 之后的版本。

## 结果

- `53c0d41`（第一部分，在 main 上）：`highlight.rs` 移到 `src/highlight.rs`，`Lang::{Token, File}`，唤醒改成回调，块 id 取自全局计数器。
- `be48b89`（第二部分）：`src/ui/code.rs`、`src/ui/diff.rs`、`tests/code.rs`、`examples/code.rs`；design.md 的 ui 加「Code」一节，「高亮」加一条；README 提到 `Code`。在分支 `wt/code` 上做成（`be48b89`），ink-rs 0.1.7 发布后 rebase 到 Composer 之上合入 main，`Cargo.toml` 的示例表和 `src/ui/mod.rs` 与 026、027 冲突，保留两边。用户选「带上控件、Composer、Code (Recommended)」，进 0.3.0（见 [030](030-release-0-3-0.md)）。
- worker 的选择和偏离，主 agent 接受：
  - `Wrap` 用 `TextWrap::Hard`，保留行首缩进。
  - `Text` 没有宽度和 `flex_shrink`，行号包在固定宽度、不收缩的 `View` 里。
  - diff 解析器 148 行，超过 brief 估的 120 行，多出的是错误类型和数值校验。
  - 高亮线程改名 `eink-highlight`。
  - 跨行注释的颜色在 `src/ui/code.rs` 的单元测试里测。
- 主 agent review 改了 diff 的解析：worker 的版本在第一个 hunk 之前认 `diff --git`、`---`、`+++`，git 的 `index`、权限和改名行让整份 diff 回退；hunk 开始后，每一行按正文解析，多文件 diff 的第二个文件头判成坏行。改成 hunk 的计数用完后，后面不以空格、`-`、`+` 开头的行和 `---`、`+++` 算文件头，画成变暗的行。两个 hunk 的测试加上 git 的头。
- 性能（release，100 列，`ink_rs::testing`，临时测试，没有提交）：

  | 源码 | 第一帧 | 高亮到达 | props 不变时重渲染 |
  |---|---|---|---|
  | 64 行 | 49 ms，含语法集在 UI 线程的第一次加载 | 同步 | 0.5 ms |
  | 2000 行 | 74 ms，纯色 | 360 ms | 28 ms |

  对照：一个 2000 行的 `Text` 重渲染 22 ms，2000 个各含一个 `Text` 的 `View` 是 76 ms。2000 行 `Code` 的开销在 ink-rs 的布局和输出，组件不是瓶颈。
- 全量：fmt；四组 feature 的 clippy（全开、不开、`markdown`、`highlight`）；`cargo test --workspace --all-features`、`--features markdown --test markdown`、不开 feature 的 `--test code`，共 153 个测试。
- 手动核对（tmux `ink-rs` 会话，`examples/code.rs`）：行号灰色，语法按 `ansi` 主题着色；删除行和新增行带 `-`、`+`，底色是 `Rgb(56, 24, 24)` 和 `Rgb(24, 48, 30)`；文件头、hunk 头和上下文变暗。

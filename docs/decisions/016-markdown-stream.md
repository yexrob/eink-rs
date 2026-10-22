# Markdown 流式渲染：先做 demo

- 日期：2026-10-22
- 状态：已实施
- 提交：`b38d92b` feat(markdown): draft a streaming markdown component；`fd0017a` docs(examples): add a markdown streaming demo
- Brief：无

## 背景

用户：「我们能不能为eink-rs 添加一个markdown流式渲染组件呢 ?」

Claude 查了现状和 ink-rs：

- `quill` 的 `markdown.rs` 是 50 行的子集（段落、`**`、反引号、围栏）。一条回复流式期间整条留在动态区，结束后才进 `Static`；回复高过一屏时，滚出屏幕的行光标回不去。
- ink-rs 0.1.3 有需要的原语：Text 的粗、斜、删除线和颜色，文本里的 OSC 8 超链接（换行时保留），`text_width`，分边框，`flex_basis`、`min_width`，`Static`，props 相等时跳过子树。不用改 ink-rs。

Claude 提的方案：feature `markdown` 引入 pulldown-cmark 0.13，模块 `eink_rs::markdown`，和 `cli`、`router` 并列；三层是 headless 的 `MarkdownStream`（`push`、`finish`，给出定稿的块和尾部）、`Markdown` 组件、`MarkdownTheme`。定稿规则：只解析最后一个换行之前的完整行，除最后一个顶层块外都定稿，依据是 CommonMark 按行解析、块关闭后不再打开。定稿的块交给应用的 `Static`，组件里不藏 `Static`。

用户回答三个取舍：

- 尾部没换行的半行：选「逐字，补齐未闭合标记」，并要求「解释一下」。Claude 解释：每帧解析前在尾部临时补上合法开头的结束符，不写进源码；`[文字](ht` 先显示文字；末尾一个说不准的符号、没写完的表格行、只有标记字符的半行先不显示。
- 定稿粒度：选「按顶层块定稿」。
- 代码高亮：选「现在做，放在独立 feature 后」。

用户又说：「对了 流式性能要高 你看看怎么处理，能否支持120hz/60hz 高刷」。Claude 在 scratchpad 写了基准：160×45，`max_fps: 120`，`Static` 里 200 条历史，动态区是 8 行段落加增长中的代码块，每 9ms 一步。每帧（重渲染、布局、绘制、差分）p50 0.70ms、p99 1.76ms，每帧写出 73 字节；pulldown-cmark 解析 3.2 KB 尾部 2.2µs。120Hz 的预算是 8.3ms。ink-rs 的节流窗口按整毫秒向上取整，120 实际约 111 帧/秒，Claude 建议不改。性能做法：收数据和画帧分开，从定稿位置增量解析，块源码用 `Rc<str>`，高亮按行缓存，可选的匀速吐字。

用户：「你写一个demo 让我先看看」。

## 决策

- 先做 demo，用户看过再定方案的其余部分、要不要派 Codex。用户定（「你写一个demo 让我先看看」）。
- 用户定（见背景的三个选择）：半行逐字显示并补齐未闭合标记；按顶层块定稿；代码高亮放在独立 feature 后。
- 以下由 Claude 按用户的授权（同上）定：
  - demo 由主 agent 写，不派 worker：用户说「你写」，方案的细节还在讨论。
  - demo 就是模块的初稿：`src/markdown/`、feature `markdown` 和 `markdown-highlight`、`examples/markdown.rs`。写在分支 `markdown-demo` 的 worktree 里，用户认可前不合入 main。
  - 高亮用 syntect 的纯 Rust 正则后端，语法和主题取 two-face（bat 的一套），默认 bat 的 `ansi` 主题，颜色跟随终端的 16 色。
  - 匀速吐字放进 demo，用开关对比；库里默认关。
  - demo 的回复由确定性的伪随机切块模拟：块长和间隔不均匀，有停顿，像网络到达。按键切换补齐、匀速吐字和帧率，`r` 重放，`q` 退出；底栏显示收到和显示的字节。
  - demo 阶段不加测试；方案定下后补定稿规则的切点测试。

## 方案

1. 主 agent：本记录先提交到 main。
2. 主 agent 在 worktree 写 demo，`cargo clippy`、`cargo build --example markdown --features markdown-highlight` 通过后，在 tmux 会话 `ink-rs` 里运行给用户看。
3. 用户看过后补结果，再开记录定实施方案。

## 结果

- 草稿和 demo 写在分支 `markdown-demo`：`b38d92b`、`fd0017a`。demo 在 tmux 里给用户看过。
- 状态栏 `max_fps: 120` 只有 89 fps，用户问「这个是终端的限制还是ink-rs的限制」。Claude 测出原因在 ink-rs 的节流：窗口取整、晚醒累积、tick 落格。用户让在 ink-rs 改（「可以 你再ink-rs里面修改一下吧」），见 ink-rs 的记录 063。背景里「Claude 建议不改」的那条由此作废。用本地 ink-rs 构建的 demo 状态栏 121–123 fps。ink-rs 0.1.4 发布后，本仓库升级依赖，demo 里 1ms interval 的绕法随之改写，另开记录。
- 用户定（「两个都要」）：草稿和 demo 合入 main。快进合并，worktree 和分支删掉。合并前 fmt、`--all-features`、`--features markdown`、`--no-default-features` 三套 clippy 通过，`cargo test --all-features` 124 个通过。
- 合入的是初稿，正式实施另开记录。Claude 写了基准（每次喂 16 字节、重渲染尾部、release 构建）测每帧开销：

| 尾部 | 无高亮 p50 / p90 | 高亮 p50 / p90 |
|---|---|---|
| demo 的回答（最长的尾部是 91 行代码块） | 0.18 / 1.0 ms | 0.46 / 2.9 ms |
| 91 行代码块 | 0.9 / 1.4 ms | 2.3 / 3.4 ms |
| 300 行代码块 | 4.8 / 5.9 ms | 12.9 / 15.6 ms |

  120 Hz 的预算是 8.33ms，60 Hz 是 16.67ms。写终端不是瓶颈：ink-rs 按格子差分。

- 由此得出三点，作为正式方案的输入。用户认可（「markdown的你说的非常好」）：
  1. 尾部的长度决定每帧开销。按顶层块定稿时，代码块写完之前留在尾部，每帧重建全部行；300 行加高亮每帧 13ms。代码块里写完的行分批进 `Static`，尾部留最后几行。代码块没有边框，分段叠起来和整块一样。高亮状态要从视图的 `use_memo` 缓存移到流里，后面的分段接上前面的语法状态。表格的列宽取决于所有行，维持整块定稿。
  2. syntect 第一次遇到一种语法时编译正则：TypeScript 代码块开头 4 帧用了 27、102、40、41ms。高亮放到后台线程，代码以无高亮的样子画出，结果回来后替换。
  3. 时钟只在匀速揭示时运行（`is_revealing()` 为真）。不开匀速时，文本到达时更新状态，帧窗口合并同一窗口内的多次到达。demo 的时钟在整个回答期间运行，300 行尾部时内容没变的帧也要 1.5ms。用组件里的一个 hook 管理时钟，应用不用自己接 `use_animation`。
- 用户要求（「做完记得压测大数据量的场景」）：正式实施的方案单列压测一步。场景包括数百到数千行的代码块、大表格、数百个块进 `Static` 的长回答、每帧到达数 KB 的高速流，在 120 Hz 下测每帧开销和实测帧率。

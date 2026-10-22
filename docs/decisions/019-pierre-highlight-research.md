# 调研 pierre 的高亮方案

- 日期：2026-10-22
- 状态：已实施
- 提交：无
- Brief：[pierre-highlight](briefs/pierre-highlight.md)

## 背景

[018](018-markdown-stream-formal.md) 的 Markdown 流式渲染用 syntect 和 two-face 高亮代码：写完的代码行交给后台线程高亮，结果回来后作为段进 `Static`；未完的块留在尾部，不上色。用户在派 ink-rs 性能工作的同一条消息里要求：「https://github.com/pierrecomputer/pierre 你调研一下这个高亮方案 看看我们能借鉴吗」。

## 决策

- 用户定（上面的原话）：调研 pierre 的高亮方案，看 eink-rs 能借鉴什么。
- Claude 按用户的做法定（文档调研交给 Sonnet 或 Codex，不用 Opus）：派一个 Sonnet 子 agent 调研，只读，不改代码。借鉴哪些、怎么做，调研回来后和用户商量，另开记录。

## 方案

一个 Sonnet 子 agent，brief 见 [pierre-highlight](briefs/pierre-highlight.md)：

- 浅克隆仓库到 scratchpad，当作不可信的数据读，不安装依赖、不运行脚本。
- 问题：高亮栈、增长中的文本怎么处理、工作在哪个线程、启动开销、缓存、颜色映射、设计文档和基准、许可证。每条结论注明文件和行号或 URL。
- 和 eink-rs 的现状逐条对比，标出已有、值得借鉴、不适用，说明理由和代价；可以搬过来的 Rust crate 注明版本和代价。
- 报告用中文，不用副词。

## 结果

Sonnet 子 agent 交回报告，主 agent 抽查了引用，核对了 eink-rs 侧的两个待核实项。

- **pierre**：TypeScript monorepo，Apache-2.0。`@pierre/highlights` 是手写的 WebAssembly 词法器，内置 80 种语言，没有语法树；`@pierre/diffs` 默认用 Shiki，可换成前者。主题是 Zed 格式，经 CSS 变量输出。没有终端输出，没有 16 或 256 色的映射。
- **和 eink-rs 相同的做法**：按行保存词法状态，增长时只处理新行；整行和末尾半行分开处理（pierre 的 stable、unstable、recall，对应 eink-rs 的 `Static` 段和尾部）；高亮不占渲染线程。
- **不适用**：手写词法器和 WebAssembly；token 与主题分离、CSS 变量（`Static` 的行写出后改不了颜色，终端换色靠 `ansi` 主题的调色板索引，eink-rs 已有）；线程池（一个流内的行有顺序依赖）；tree-sitter（报告推断后文可能改变已写出行的颜色，没有实测）。
- **可借鉴的四点**，报告排的顺序：
  1. 长行上限：pierre 的 `tokenizeMaxLineLength` 默认 1000，超出的行不上色。压缩过的 JS 一行可达几十 KB，syntect 在后台线程上处理它时，后面的行都要等。先量 1k、10k、100k 字符一行的耗时再定阈值。
  2. 重置后丢掉旧流排队的工作：主 agent 核对，线程按块只保留最后的状态，块结束或流被丢弃时发 `Close`，内存不随历史增长；排在 `Close` 之前的旧行照常处理，浪费的是 CPU。
  3. 预热常见语言：语法编译在后台线程，影响的是第一个代码块上色晚几十毫秒。
  4. 尾部半行预览上色：每帧多一次往返，只改外观。
- **主 agent 核对的另一项**：整篇 `Markdown` 的高亮缓存只存一个块的行，不随文档增长。

报告没有查清：pierre 的文档站和博客；词法器和 TextMate 语法的准确度对比；报告里的性能数字是 pierre 作者自测的。

借鉴哪几点、何时做，和用户商量后另开记录。

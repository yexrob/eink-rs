# 调研 pierre 的高亮方案

- 日期：2026-10-22
- 状态：实施中
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

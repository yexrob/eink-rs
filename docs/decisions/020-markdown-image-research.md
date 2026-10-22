# 调研 Markdown 图片显示

- 日期：2026-10-22
- 状态：实施中
- 提交：无
- Brief：[markdown-image](briefs/markdown-image.md)

## 背景

用户问：「ink-rs 和 eink-rs 不是支持图片吗 我们的markdown 能支持图片显示吗」。两个库都不能显示图片：ink-rs 只有 `OutputWriter::write_control`，把原始字节放在帧之后发给终端，不计入帧高、不负责摆放；[018](018-markdown-stream-formal.md) 的 Markdown 组件把图片显示成暗色斜体的 `[alt]`。

主 agent 列了三件要先想清楚的事：终端用什么方式画图（半块字符、kitty 占位符、iTerm2、Sixel），图片和流式定稿的关系（段进 `Static` 后改不了，图片要在定稿前加载好），加载器（按全局规则先定义接口，给本地文件和 HTTP 各一个实现）。主 agent 建议先调研，问要不要派 Sonnet。用户答：「可以 看看吧 markdown image的那个问题」。

## 决策

- 用户定（上面的原话）：调研 Markdown 的图片显示。
- Claude 按用户的做法定（文档调研交给 Sonnet 或 Codex，不用 Opus）：派一个 Sonnet 子 agent 调研，只读，不改代码。做不做、怎么分在 ink-rs 和 eink-rs 之间，调研回来后和用户商量，另开记录。按规则 5，需要 ink-rs 加原语的部分，在 ink-rs 里另开记录。

## 方案

一个 Sonnet 子 agent，brief 见 [markdown-image](briefs/markdown-image.md)：

- 问题：各协议和终端、tmux 的支持；运行时探测协议和单元格像素尺寸；每种协议和 ink-rs 帧模型、`Static`、回滚、重绘的关系；Rust crate（ratatui-image、viuer、icy_sixel、image）；Ink 生态的做法；加载和解码；各部分归 ink-rs 还是 eink-rs。每条结论注明出处。
- 读本机的 `TERM`、`TERM_PROGRAM`、tmux 版本和 `allow-passthrough`，用户在 tmux 里验收。
- 克隆的仓库当作不可信的数据读，不安装依赖、不运行脚本。
- 报告用中文，不用副词，含方案对比表和第一版的建议。

## 结果

待调研完成后填写。

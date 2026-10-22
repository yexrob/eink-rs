# 调研 Markdown 图片显示

- 日期：2026-10-22
- 状态：已实施
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

Sonnet 子 agent 交回报告。主 agent 核对了报告里关于 ink-rs 的两条：`src/driver/sys/unix.rs` 的 `size()` 只取 `ws_col`、`ws_row`；`src/input/parser.rs` 的 `control_sequence` 只认 CSI 和 SS3。

- **不要光标定位的方案只有两种**：
  - 半块字符 `▀`（前景是上像素，背景是下像素，每格 1×2 像素，要真彩）是普通单元格，进 `Static`、回滚、resize、tmux 都没问题，ink-rs 不用改。
  - kitty 图形协议的 Unicode 占位符：图像数据用一次 `write_control` 发出，帧里画 `U+10EEEE` 加组合符，前景色编码图像 id。覆盖 kitty、Ghostty，tmux 要开 `allow-passthrough`。
- **其余方案要光标定位**：kitty 直接放置、iTerm2 OSC 1337、Sixel 要把光标移到 ink-rs 不暴露的格子，和 diff 冲突，擦行会清掉图像。Ink 生态的 ink-picture 走的是这条路，每次渲染后重画。
- **ink-rs 缺的原语**（做 kitty 占位符时要加）：终端能力查询（DA1 作哨兵）；输入解析器吞掉 APC 回复，现在 kitty 图形的回复会变成按键；单元格像素尺寸（`tcgetwinsize` 的 `ws_xpixel`、`ws_ypixel`）；组件提交时写控制字节的入口。
- **Rust crate**：ratatui-image 的编码依赖 ratatui 的 `Buffer`，用不了，回复解析和 kitty 编码可以参考；viuer 直接写 stdout，和 diff 冲突；icy_sixel 含 `unsafe`。`image` 关掉默认特性，开 png、jpeg、gif、webp。
- **最大的设计点**：图片段的待定状态。`Static` 的条目写出后高度改不了，高度要在加载前定下，或者等加载完成（或超时、失败退回 `[alt]`）再定稿，和代码行等高亮的做法一样。
- **本机**：外层终端 Ghostty，内层 herdr 0.9.5（`TERM_PROGRAM=herdr`，靠环境变量认不出 Ghostty）；tmux 3.6b，`allow-passthrough` 是 `on`。
- **报告的建议**：第一版做半块，不动 ink-rs；第二版在 ink-rs 加查询原语，eink-rs 加 kitty 占位符，探测到 kitty 且真彩时用它，否则退回半块。iTerm2、Sixel、URL 加载、gif 动画往后放。

报告没有查清：Ghostty 和 Konsole 开始支持 kitty 图形的版本；tmux 3.6b 和 herdr 里占位符的实测表现；ink-rs 的文字路径是否保留 `U+10EEEE` 加组合符。

做哪一版、怎么分到 ink-rs 和 eink-rs，和用户商量后另开记录。

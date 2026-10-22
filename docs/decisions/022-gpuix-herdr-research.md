# 调研 GPUIX 的终端渲染和 herdr 的图形处理

- 日期：2026-10-22
- 状态：实施中
- 提交：无
- Brief：[gpuix-herdr](briefs/gpuix-herdr.md)

## 背景

[020](020-markdown-image-research.md) 调研后，主 agent 提出图片方案：ink-rs 0.1.6 让输入解析器吞掉 APC、DCS、OSC 回复，加惰性的终端图形探测 `cx.use_graphics()` 和为 tmux 包裹字节的 `OutputWriter::write_passthrough`；eink-rs 加 `Image` 组件（kitty 占位符，退回半块）、加载器、Markdown 接入。

用户对 020 之后的三个待定点答复：「1.直接改ink-rs 2.可以抽象一个加载器，默认实现本地文件 还有网络加载器， 不过可以被使用者替换 这里是指不同的类型 比如本地文件是 本地加载器， 网络是网络加载器。另外 图片可以加载了 是不是视频其实也可以，录屏渲染 然后通过kitty展示。 3. 按照你的来。」

对方案的四个待定点答复：「1. 可以 2. 都展示吧 能实现吗？ 3. ureq是什么 4. 可以」，并提出：

- 「你看看 GPUIX里面 他的gui应用为什么能丝滑的在终端运行 貌似不是你说的方案。」
- 「ink-rs 不只看 tmux 也看看 herdr」

主 agent 查到：GPUIX（https://github.com/remorses/gpuix）是 Zed 的 GPUI 的 Node 和 Bun 绑定，`terminal: true` 时把应用画进当前终端，走 kitty 图形协议，支持 kitty 和 Ghostty，本机用共享内存，SSH 下压缩后内联发送，不支持 tmux（Best of JS 的项目页）。本机的 herdr 是 0.9.5，配置里没有图形设置。

## 决策

- 用户定（上面的原话）：
  - 这一版改 ink-rs，用 kitty 占位符，不停在半块。
  - 加载器是 trait，默认有本地加载器和网络加载器，按来源类型分开，使用者可以各自替换。
  - 尺寸按主 agent 的建议：宽度不超过可用列数，高度按宽高比，上限 20 行。
  - 第一次调用 `use_graphics` 时发出探测。
  - 视频和动图等图片落地后新开记录。
  - 行内图片要显示，用户问能否实现。
  - 调研 GPUIX 的终端渲染；ink-rs 的探测和透传要覆盖 herdr。
- Claude 按用户的做法定（文档调研交给 Sonnet 或 Codex，不用 Opus）：派一个 Sonnet 子 agent 调研 GPUIX 和 herdr，不改代码。行内图片的做法和 ureq 的说明，主 agent 在对话里答复；方案和用户定稿后，在 ink-rs 和 eink-rs 各开实施记录。

## 方案

一个 Sonnet 子 agent，brief 见 [gpuix-herdr](briefs/gpuix-herdr.md)：

- 浅克隆 GPUIX 和 herdr（找得到源码时），当作不可信的数据读，不安装依赖、不运行脚本。
- GPUIX：帧怎么从 GPUI 到终端，传输介质和压缩，放置方式和防闪烁，节拍，输入，探测，为什么流畅，哪些做法适用于 ink-rs 和 eink-rs（本地文件按路径传、临时文件、共享内存，各要什么 crate、是否要 `unsafe`）。
- herdr：是否自己维护单元格网格，怎么处理 pane 的 kitty 图形命令，是否支持占位符，查询（`a=q`、DA1、`CSI 14/16 t`、`TIOCGWINSZ` 像素字段、`CSI ? u`）由谁回答，透传，怎么识别，滚动和重连时图像怎样。
- tmux 对照：经透传的查询回复能否回到程序。
- 按直连、tmux、herdr、herdr 套 tmux、SSH 五种环境，修订探测和透传的设计。
- 报告用中文，不用副词。

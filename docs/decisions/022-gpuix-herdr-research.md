# 调研 GPUIX 的终端渲染和 herdr 的图形处理

- 日期：2026-10-22
- 状态：已实施
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

## 结果

Sonnet 子 agent 交回报告。主 agent 核对了三处：herdr 的 `crates/ghostty-vt/src/lib.rs:208` 定义 `U+10EEEE` 占位符，`config.rs` 的 `kitty_graphics_enabled` 默认 `true`；tmux 3.6b 的 `input_exit_apc` 把 APC 内容设为窗格标题；ink-rs 的帧输出用 `?2026` 包裹（`src/app.rs`）。

- **GPUIX 的终端模式**：整屏一张位图。GPU 离屏渲染，回读 RGBA；本机用共享内存（`t=s`），每帧经 pty 的是约 100 字节的命令，SSH 下 zlib 压缩后内联分块；同一个图像 id 原子替换，外面包 `?2026`；直接放置，备用屏，没有文字层，没有 scrollback；SSH 下每帧后发 `a=q` 作围栏，在途不超过两帧。它不检测终端名字，靠 `a=q,t=s` 加 DA1 判断终端能否读共享内存。README 写明不支持 tmux。
- **对 ink-rs 和 eink-rs**：整屏位图不适合文字界面（没有 scrollback、不能选择复制、要 GPU 栈）。可借的是传输手法：本地文件按路径传（`t=f`）、临时文件（`t=t`），不需要 `unsafe`；共享内存要 `mmap`，需要 `unsafe`。同 id 替换、`?2026`、`a=q` 围栏留给视频。
- **herdr**（源码 https://github.com/herdrdev/herdr ，0.9.5）：和 tmux 一样，用 libghostty 解析每个 pane 的输出，自己重绘外层终端。pane 里的 kitty 图形命令由 herdr 接收，支持占位符和虚拟放置，再用直接放置发给外层终端；默认开启。查询由 herdr 自己回答：`a=q` 回 `OK`，DA1 回 `CSI ? 62;22 c`，`CSI 16 t` 和 `TIOCGWINSZ` 的像素字段按外层单元格尺寸填写。`ESC P tmux;` 透传被丢弃。pane 环境有 `TERM_PROGRAM=herdr`、`HERDR_ENV=1`，`TMUX`、`KITTY_WINDOW_ID` 等外层标记被删掉。herdr 跳过带压缩的放置。
- **tmux 3.6b**：没有 kitty 图形；`allow-passthrough on` 时转发包裹的字节；自己回答 DA1 和 `CSI 16 t`，所以 DA1 在 tmux 里做不了外层回复的哨兵，要包裹后发给外层。外层的 kitty 回复能否回到程序，报告读源码的推断和 GPUIX 作者的说法不一致，要实测。tmux 主分支新加的 kitty 支持拒绝 `U=1`。
- **修订的探测**：kitty 查询、`CSI 16 t`、DA1 哨兵；`TMUX` 有值时包裹，herdr 内不包裹；herdr 不需要专门的代码。herdr 套 tmux、tmux 套 herdr 的组合程序识别不了，超时后按不支持处理，列为限制。
- **待手测**：tmux 里包裹查询的回复、大块数据是否截断、占位符是否保留；herdr 里占位符、滚动、resize、detach 后的表现。

报告没有查清：tmux 3.6b 里外层回复的实际行为；herdr 里占位符的实际表现；tmux 哪个版本加入 `image-kitty.c`。

方案和用户定稿后，在 ink-rs 和 eink-rs 新开实施记录。

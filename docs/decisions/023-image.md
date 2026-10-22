# 图片：`Image` 组件、加载器和 Markdown 接入

- 日期：2026-10-22
- 状态：实施中
- 提交：无
- Brief：[image-component](briefs/image-component.md)、[image-markdown](briefs/image-markdown.md)

## 背景

用户问「ink-rs 和 eink-rs 不是支持图片吗 我们的markdown 能支持图片显示吗」。Markdown 组件把 `![alt](src)` 显示成暗色斜体的 `[alt]`。调研见 [020](020-markdown-image-research.md) 和 [022](022-gpuix-herdr-research.md)。ink-rs 的记录 069 加三项原语：输入解析器认出 APC、DCS、OSC 回复；`Scope::use_graphics()` 惰性探测 kitty 图形和单元格的像素尺寸；`OutputWriter::write_passthrough` 在 tmux 里包裹字节。本记录做 eink-rs 这一侧。

## 决策

- 用户定：
  - 「1.直接改ink-rs」：用 kitty 的 Unicode 占位符，原语放在 ink-rs。
  - 「2.可以抽象一个加载器，默认实现本地文件 还有网络加载器， 不过可以被使用者替换 这里是指不同的类型 比如本地文件是 本地加载器， 网络是网络加载器。」：加载器是 trait；本地文件和网络各有默认实现，按来源分开，使用者可以各自替换。
  - 「3. 按照你的来。」：宽度不超过可用列数，高度按宽高比，上限 20 行。
  - 「2. 都展示吧 能实现吗？」：行内图片要显示。
  - 「可以用ureq也行。」：网络加载器用 ureq。
  - 「不支持kitty的终端就先直接退化成alt。」：不支持 kitty 时显示 `[alt]`，不做半块。
  - 「1. 这一版就做吧。」：SVG 用 resvg，这一版做。
  - 「4. 可以」：视频和动图新开记录。
  - 「2. 可以的」：确认修订后的方案。
- 方案的要点（主 agent 提出，用户确认）：
  - `Image` 组件：终端支持 kitty 时画占位符，图像数据由 effect 发一次；不支持时显示 `[alt]`。
  - 传输一条路径：解码，缩到显示的像素尺寸，编码成 PNG（`f=100`），`t=d` 内联，每块 4096 字节。`t=f`、`t=t` 留给视频。
  - 独占一段的图片按块显示。行内图片按可用宽度的自然高度不超过一行半时，缩成 1 行嵌在文字里；超过一行半的把段落拆开，图片自成一块。
  - 流里含图片的段等探测和加载结束后定稿。本地加载超时 5 秒，网络 10 秒，失败显示 `[alt]`。文件上限 20 MB，解码用 `image` 的 `Limits`。
- Claude 按用户的确认（「2. 可以的」）定细节：
  - **feature**：`Image` 是通用组件，不属于 Markdown。方案原文是 `markdown-image` 和 `markdown-image-http` 两个 feature，改为：
    - `image`：`Image` 组件、加载器 trait、本地加载器、png、jpeg、gif（第一帧）、webp 的解码；
    - `image-http`：ureq 的网络加载器；
    - `image-svg`：resvg；
    - `markdown-image = ["markdown", "image"]`：Markdown 接入。
  - **加载器的替换**：`ImageLoaders` 有本地和网络两个槽，经 `Scope::provide` 提供；没有提供时，本地槽是以当前目录为基准的本地加载器，网络槽在开 `image-http` 时是 ureq 加载器，否则为空，网络图片显示 `[alt]`。开 `image-http` 是打开网络加载的开关。
  - **图像 id**：24 位，进程启动时取随机的起点（`std` 的 `RandomState`），之后递增。tmux 里各窗格共用外层终端的 id 空间，随机起点减少冲突。终端按配额回收旧图，eink-rs 不删除图像。
  - **发送**：渲染结果里用到的图片由一个 effect 发送：数据每张发一次，虚拟放置在列数或行数变化时重发。块图片和行内图片走同一个机制。`Static` 里的条目在写出后运行 effect，占位符早于数据到达终端，kitty 和 Ghostty 在数据到达后绘制。
  - **可用宽度**：`Markdown` 组件和 `MarkdownTail` 用 `use_box_metrics` 量自己的宽度，第一次量到之前用窗口列数；流里的段用尾部量到的宽度。减去列表和引用的缩进后是图片的列数上限。
  - **单元格尺寸未知时**：按宽高 1:2 算行数。
  - **id 的编码按色深**（ink-rs 0.1.7 的 `use_color_level`）：占位符的前景色是 id，降级后的颜色会指向别的图。真彩用 24 位 id 的 `Rgb`；256 色用 8 位 id 的 `Ansi256`，用自己的计数器；低于 256 色时显示 `[alt]`。
  - 拆成两步，每步派一个 Codex：第一步 `image` 模块，第二步 Markdown 接入。
- 第二步派出之前，用户定（2026-10-22）：
  - 「模型加载 (Recommended)」：流里的图片由 `MarkdownStream` 加载，和高亮的做法相同。含图片的段在现成的等待队列里排着，后面的段排在它后面；图片就绪、失败或超时后交出。备选是 `use_markdown_stream` 调第一步的 `use_images`，layout effect 把结果交回模型：没有界面的 `MarkdownStream` 拿不到图片，加载状态分在 hook 和模型两处。
- Claude 按这个选择定第二步的细节：
  - `MarkdownStream` 加 `set_images(MarkdownImages)`：`Alt`（默认，不加载）、`Probing`（含图片的段等着，不加载）、`Kitty { loaders, cell_pixels, color_level }`。`use_markdown_stream` 按 `use_graphics`、`use_color_level` 和上下文里的 `ImageLoaders` 设它。
  - `set_columns` 给出图片可用的列数，默认 80；`MarkdownTail` 量到自己的宽度后设它，量到之前用窗口列数。段定稿时按当时的列数和缩进算图片尺寸，段带着加载好的图片和尺寸，进 `Static` 后不加载。
  - 第一步的加载线程、作业和期限从 `image/hook.rs` 抽成不依赖 hook 的函数，`use_images` 和流共用。流的期限按 `tick` 累计的时间算；`use_markdown_stream` 在有段等图片时保持动画时钟运行。没有界面时用 `wait_for_images`，对应 `wait_for_highlights`，阻塞到最早的期限为止。
  - 表格里的图片显示 `[alt]`：单元格的宽度由列宽决定，放不下占位符的行列。
  - `examples/markdown.rs --file` 在开 `markdown-image` 时用文件所在目录的 `FileLoader`，不加新示例。

## 备选与取舍

- **半块（`▀`）后备**：用户选了退回 `[alt]`。
- **iTerm2 协议、Sixel**：要光标定位，和 ink-rs 的 diff 冲突（020）。
- **按路径传（`t=f`）、临时文件（`t=t`）**：要确认终端和程序在同一台机器上，留给视频。
- **共享内存（`t=s`）**：要 `mmap`，需要 `unsafe`。
- **图片的尺寸写进 Markdown 主题**：上限 20 行是常量，`Image` 组件的 `max_rows` 可改。主题不加字段。
- **网络加载默认关闭、由应用安装**：用户要「默认实现」网络加载器，feature 是开关。

## 方案

两个 Codex worker（`codex exec -s workspace-write`），第一个交回后派第二个，工作目录是 scratchpad 里基于 main 的 worktree。它们不提交，不改文档，跑 fmt、clippy 和 brief 列出的测试。ink-rs 的依赖升到 0.1.6（`use_graphics`、`write_passthrough`）；id 的编码要 0.1.7 的 `use_color_level`，0.1.7 发布前，worktree 的 `Cargo.toml` 用 `[patch.crates-io]` 指向 ink-rs 合入 074 之后的快照，提交前去掉。沙箱没有网络，新依赖（`image`、`ureq`、`resvg`）由主 agent 在派出前加进 `Cargo.toml` 并下载。第二步等列表项代码块（025）和 Code 的第一部分（028）合入后派出。

1. **`image` 模块**，brief 见 [image-component](briefs/image-component.md)：加载器、解码和缩放、SVG、kitty 的传输和占位符编码、`Image` 组件。
2. **Markdown 接入**，brief 见 [image-markdown](briefs/image-markdown.md)（020 占用了 `markdown-image.md` 这个名字）：渲染器的块、行内和拆段；`Markdown` 组件的加载；流里的段等探测和加载；宽度的测量。

主 agent 集成：review，按逻辑改动分次提交；全量测试和 clippy；`docs/design.md` 加图片一节；在 Ghostty 直连、tmux 和 herdr 里看 README 和含图片的流式回答；补本记录的结果，删 worktree。

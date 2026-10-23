# 动图和视频

- 日期：2026-10-23
- 状态：实施中
- 提交：无
- Brief：[animation-terminal](briefs/animation-terminal.md)、[animation-decoding](briefs/animation-decoding.md)、[animation-image](briefs/animation-image.md)、[video-decoder](briefs/video-decoder.md)

## 背景

[023](023-image.md) 把视频和动图留给新的记录（「4. 可以」）。0.3.1 的 `image` 解码 gif 时取第一帧，传输一条路径：PNG、`t=d`。023 的备选里，按路径传（`t=f`）和临时文件（`t=t`）留给视频，共享内存（`t=s`）要 `mmap`，需要 `unsafe`。

用户说「开始规划动图和视频」。

主 agent 读了 ink-rs 0.1.7 的现状：

- `OutputWriter::write_control` 和 `write_passthrough` 的字节在包含之前状态改动的那一帧之后发出；这一步不绘制时在步末发出。切换帧的命令可以不经重绘发出。
- `use_animation` 给出时钟，间隔 1 ms 起。
- 探测（`use_graphics`）发一条 `a=q,t=d` 的查询，给出 `Kitty`、不支持或探测中，和单元格的像素尺寸。它不区分终端能不能读文件、共享内存，不区分动画命令。要区分时，探测属于 ink-rs 的原语。

## 决策

- 用户定：
  - 「开始规划动图和视频」。
  - 「无声播放组件 (Recommended)」：`Video` 组件，播放、暂停、循环、按时间跳转，没有声音。备选是带声音（cpal 或 rodio，要做音画同步）和这一轮的范围限于动图。
  - 「本地文件和网络地址 (Recommended)」：视频走加载器的两个槽，本地文件和 http(s) 分开，使用者可以替换；网络视频按流读，不等下载完。备选是限于本地文件。
- Claude 按记忆里的做法（调研之后和用户商定方案，商定后派实现的 worker）定：
  - 这一轮调研，不写代码。
  - 调研要查网页和终端的源码，派两个 Sonnet 子 agent；Codex 的沙箱没有网络。
  - 调研结果回来后，主 agent 整理方案和要用户定的问题，商定后补本记录的决策和方案，之后派实现。
- 调研回来后，用户定（2026-10-23）：
  - 「每帧重传，两个 id 轮换 (Recommended)」：动图每帧传一张 PNG，两个 id 轮流装帧，占位符的颜色在两个 id 之间切换。备选是帧一次传完、按节拍用 `a=a,c=` 切帧（要 kitty 或没发版的 Ghostty，ink-rs 要加动画探测，Ghostty 1.3.1 播不了），和做两条路线、按探测选（两套代码，tmux 里探测失效）。
  - 「ffmpeg 命令行 (Recommended)」：视频调 PATH 上的 `ffmpeg` 和 `ffprobe`。备选是链接 ffmpeg-next：构建要 libclang 和 FFmpeg 头文件，Windows 要配 `FFMPEG_DIR`，链接带 LGPL 义务，crate 处在维护模式。
  - 「解码器 (Recommended)」：视频的两个槽里放解码器，解码器拿到地址自己读。备选是字节流加载器：槽返回 `Read`，喂给 ffmpeg 的 stdin；moov 在文件尾的 MP4 从管道解不了，跳转要从头读。
  - 「播放器句柄 (Recommended)」：`use_video_player` 返回句柄，有 play、pause、seek，能读位置和时长；`Video` 组件接这个句柄。备选是受控属性：父组件传 `playing`、`looping` 和带序号的跳转目标，进度经回调拿到。
- Claude 按这四项选择定动图的细节（第一步）：
  - 归 `image` feature，不加依赖。`image` 0.25 的 GIF、APNG、动态 WebP 解码器逐帧给出合成过的整幅帧；单帧的文件按静态图处理。
  - 加载线程把每帧缩到显示的像素尺寸，编成 PNG（`Fast` 压缩、`Adaptive` 滤波）存下来。编码后的总字节超过 32 MiB 或帧数超过 1000 的，退回第一帧，按静态图显示。期限和 023 相同，覆盖整个准备过程。
  - 帧的延迟：不超过 10 ms 的按 100 ms 算，和浏览器一致。循环次数按文件；GIF 没有循环块时 `image` 报无限循环，按无限循环处理。
  - 播放：组件用一个 `use_animation` 时钟，间隔不超过播放中任何一帧的延迟，不低于 20 ms；循环次数用完后停在最后一帧，没有播放中的动图时时钟停止。空闲 id 里传好的帧到了时间，渲染换成这个 id，没到时间保持在屏的 id；渲染之后的 effect 把这时该显示的帧的下一帧，连同它的虚拟放置，传进不在显示的那个 id。`write_passthrough` 的字节在包含之前状态改动的那一帧之后发出，换色的帧在上传之前到达终端。
  - 流的段进 `Static`，滚动区里的占位符没法重画。流加载第一帧，用一个 id；`ImageRequest` 加一个字段区分。
  - `Image` 组件和 `Markdown` 组件里的动图默认播放，`Image` 的属性不变。
  - 组件卸载时，删掉不在显示的那个 id 的数据（`a=d,d=I`）；在显示的 id 留给终端里的占位符。023 的「eink-rs 不删除图像」对这个 id 不适用：它装的帧没有占位符引用。
  - 256 色下 id 取自 254 个的池子，一张动图占两个。
- Claude 按这四项选择定视频的方向（第二步），细节在第二步派出前补：
  - 新 feature `video`，不加依赖。`VideoDecoder` trait 给出视频信息（尺寸、时长），从某个时间点起按给定尺寸和帧率输出原始帧。默认实现用 `std::process` 调 `ffmpeg` 和 `ffprobe`，用 `-vf fps=..,scale=..` 出恒定帧率，第 n 帧的时间是起点加 n 除以帧率。一个生成测试图样的本地实现给测试和示例用。没装 ffmpeg 时显示 `[alt]`。
  - 两个槽按地址区分，经 `Scope::provide` 提供，使用者可以替换。两个槽的默认实现是 ffmpeg：本地路径相对基目录；网址交给 ffmpeg，它按 HTTP Range 分段读。
  - 解码线程收原始帧、编成 PNG，排 2 到 3 帧的队；渲染一侧按时钟取帧，迟到的帧丢掉。送出的像素不超过 640×360，帧率上限 24，属性可以改；传输沿用动图的两个 id 轮换。
  - 暂停时停止读管道，ffmpeg 被管道挡住；跳转和循环开新进程。
- 第一步派出后，用户问「视频的呢」。Claude 定：视频拆成两半。解码这一半不依赖动图的发送路径，和动图并行派；组件这一半（`Video` 组件、播放器句柄、发送）用动图的两个 id 轮换，等动图合入、用户看过之后派。解码这一半的细节：
  - `video = ["image"]`，PNG 编码用 `image`。
  - 没有提供者时，两个槽是 `FfmpegDecoder`。网络视频不加依赖，没有自己的 feature；应用提供 `network: None` 关掉它。
  - 本地地址写成 `file:<路径>`，加 `-protocol_whitelist file`：地址不能被当成选项，不能选别的 ffmpeg 协议（`concat:`、`subfile:`、指向别处的播放列表）。网络地址限于 `http://` 和 `https://`，白名单是 `http,https,tcp,tls,crypto`，`-rw_timeout` 10 秒，和图片的网络期限一致。
  - ffprobe 报的旋转为 ±90 或 270 度时交换宽高：ffmpeg 默认按旋转输出帧。
  - 帧队列（crate 内部）：后台线程把原始帧编成 PNG，带上在视频里的时间，排 3 帧，满了阻塞。跳转和循环开新的流，帧带流的代号，旧代号的帧不交出。取帧不阻塞，拿时间到了的帧里靠后的那一帧，之前的丢掉。
  - 本地实现：测试定义自己的解码器，和图片的测试加载器一样；第二半的示例定义一个生成测试图样的解码器，给没装 ffmpeg 的机器用。不加公开的 mock。

## 调研要点

报告在 scratchpad 的 `research/animation/terminal.md` 和 `decoding.md`。

- **终端**：
  - 本机的 Ghostty 1.3.1 对 `a=f`、`a=a`、`a=c` 回 `ERROR: unimplemented action`。Ghostty 的动画在 2026-08-21 合进 main，没有发版。WezTerm 解析 `a=f`，自己按帧延迟播放，没有 `a=a`；WezTerm 和 Konsole 不支持 Unicode 占位符。
  - herdr 0.9.5 内嵌 libghostty，自己解析程序的图形命令，帧一变，把整张图用自己的 id 和真实放置重传给外层终端；外层终端见不到 `a=f`、`a=a` 和 `U=1`。终端自己播放的 `s=3` 在 herdr 里不推进（没有调用动画节拍的代码）。估算超过 30 MiB 的上传被跳过。
  - 同一个 id 重传图像，删掉它的全部放置，包括虚拟放置；滚动区里的占位符显示这个 id 现在装的图。重传的空档在 herdr（#3676）和 Konsole（MR !1240）有闪烁的报告。timg 用两个 id 轮换。
  - tmux 3.6 不把终端的回复送回窗格，`a=q` 探测失效。一条 DCS 超过 1 MiB 被丢弃，每块包一层。本机实测：突发超过 35 MB/s 时丢 11% 到 25% 的块；按节拍送、30 MB/s 以内丢不到 1%。
  - 配额：kitty 每个缓冲区 320 MiB，帧的上限是它的 5 倍；Ghostty 每屏 320,000,000 字节，帧按 `w*h*4` 计。超出时回 `ENOSPC`。重用 id 时占用不增长。
  - 传输介质：`a=q` 带真实的临时文件或共享内存对象、后跟 DA1，可以测出终端能读哪种介质、和程序在不在同一台机器上；tmux 里不可用。`t=t` 的路径要在临时目录里、含 `tty-graphics-protocol`。
- **解码**：
  - `image` 0.25.10 的 GIF、APNG、WebP 解码器逐帧流式输出合成过的帧。200 帧 480×270：全收下 110 MB，流式 5 到 7 MB。`Limits` 约束的是一帧，WebP 不理会；总量的上限要自己写。
  - 没有循环块的 GIF（播一遍）和无限循环的 GIF 报的是同一个值 `Infinite`。GIF 的延迟可以是 0。16 位的 APNG 不支持。
  - ffmpeg 8.1.2 解不了动态 WebP，`image-webp` 可以。
  - ffmpeg 命令行经管道出原始 RGBA：720p 和 1080p 的 H.264、VP9、AV1 连缩放到 640×360，每秒 545 到 1400 帧。起进程到第一帧 70 ms 左右；跳转开新进程，135 到 149 ms。原始帧没有时间戳，可变帧率的输入默认补帧成恒定帧率。
  - 进程内的解码器：ffmpeg-next 处在维护模式，要 libclang 和系统头文件；GStreamer 0.25 要 Rust 1.92；openh264 解不了 Main 和 High（300 帧出 15 帧）；rav1d 的公开接口是 `unsafe extern "C"`；dav1d 要系统的 libdav1d。
  - 每帧的开销：`imageops` 的 Lanczos3 从 1080p 缩到 720p 要 29.9 ms，`fast_image_resize` 要 7.3 ms。PNG `Fast`+`Adaptive` 在 640×360 编一帧 0.6 到 1.8 ms。
  - 带宽：640×360、24 fps，PNG `Fast` 每秒 1.8 到 13.4 MB，看画面内容；原始 RGB 22.2 MB，RGBA 29.6 MB。1280×720 的 PNG 每秒 5.6 到 45.2 MB。终端一侧的吞吐没有测。

## 备选与取舍

- **`t=t` 临时文件、`t=s` 共享内存**：省掉 base64 和 pty 的带宽，要 ink-rs 加媒介探测，tmux 里探测失效。留给新的记录。
- **`a=f` 帧一次传完**：Ghostty 发版后开新的记录。
- **声音**：用户定无声。
- **按丢帧自动缩小尺寸**（vid 的做法）：第一版用固定的上限，属性可以改。
- **`fast_image_resize`**：视频的缩放交给 ffmpeg 的 `-vf scale`，动图在加载时缩放一次，不加这个依赖。

## 方案

1. **调研**（完成）：
   - 终端一侧，brief 见 [animation-terminal](briefs/animation-terminal.md)：kitty 的动画命令（`a=f`、`a=a`、`a=c`）和占位符的关系；同一个 id 重发图像的效果；四种传输介质和探测的办法；kitty、Ghostty、WezTerm、Konsole、herdr 和 tmux 的支持表；吞吐和配额；`icat`、mpv、timg、chafa、yazi、ratatui-image、viu 的做法。
   - 解码一侧，brief 见 [animation-decoding](briefs/animation-decoding.md)：`image` 0.25 的动图 API；视频解码的几条路（ffmpeg 命令行、ffmpeg-next、GStreamer、纯 Rust 或内置的解码器）的平台、许可证、MSRV 和 `unsafe`；每帧缩放和编码的耗时与字节数（实测）；播放器的节拍和音频；动图的内存。
   - 两个 Sonnet 子 agent 写报告，不改仓库。要点见「调研要点」。
2. **第一步，动图**，brief 见 [animation-image](briefs/animation-image.md)：一个 Codex worker（`codex exec -s workspace-write`），工作目录是 scratchpad 里基于 main 的 worktree。它不提交，不改文档，跑 fmt、clippy 和 brief 列出的测试。主 agent review、提交、跑全量测试，在 tmux 里核对，`docs/design.md` 的图片一节补动图，请用户在 Ghostty 和 herdr 里看。
3. **第二步，视频**：
   - 解码这一半，brief 见 [video-decoder](briefs/video-decoder.md)：一个 Codex worker，和第一步并行，用自己的 worktree。它不改 `src/image/` 和 `src/markdown/`，够得着现有的项需要的改动除外，在报告里列出。
   - 组件这一半：动图合入、用户看过之后，主 agent 补细节和 brief，派一个 Codex worker。
4. 版本号和发版在两步合入后商定。

## 结果

- 第一步，动图：c1771f3。review 的改动：
  - `LoadedImage` 分成静图和动图两种内容，去掉动图上空着的 PNG 和 `expect`。槽里记帧显示的时间，不记帧号，解码线程送来的视频帧走同一条路。
  - worker 认为 ink-rs 在卸载时丢掉 cleanup 的写入，没做空闲 id 的删除。核对 ink-rs 0.1.7：`begin_unmount` 在应用退出时设 `unmounting`，组件卸载时不设。补上 `a=d,d=I`，测试改成卸掉组件，断言删除的是空闲 id。
  - 解码合成一个循环：单帧、超过 1000 帧或 32 MiB 时返回 `None`，走静图的路。延迟用 `Duration::from(Delay)` 换算。
  - tmux 会话没有接终端，探测拿不到 kitty 的回答，示例显示 `[alt]`。图形请用户在 Ghostty 和 herdr 里看，`Markdown` 组件用一个不入库的读文件示例看。
- 第二步的解码这一半：4809b59。review 的改动：
  - 去掉帧上的代号字段：seek 清空队列，队列里的帧属于当前代号。
  - `VideoDecoders::decoder` 改成 crate 内部，公开的 API 和 `ImageLoaders` 对齐。
  - 循环时 seek 到最后一帧之后，空的流按结束处理，回到开头；从零开始的空流报错。worker 的版本在这种情况下报错。
  - 读帧去掉 `try_reserve_exact`：帧的字节数在 `FrameRequest` 上校验过。
  - 限制：循环在最后一帧的间隔过完后启动新进程，接缝处有 70 到 150 ms 的停顿；解码器卡在 `next` 里时，seek 和丢弃要等它返回。
- fmt，无 feature、`video`、`image`、`markdown`、`markdown-image`、`image-svg` 和全部 feature 的 clippy，全量测试（212 个）通过。`docs/design.md` 补了动图和 `video` 一节。

# 动图和视频

- 日期：2026-10-23
- 状态：实施中
- 提交：无
- Brief：[animation-terminal](briefs/animation-terminal.md)、[animation-decoding](briefs/animation-decoding.md)

## 背景

[023](023-image.md) 把视频和动图留给新的记录（「4. 可以」）。0.3.1 的 `image` 解码 gif 时取第一帧，传输一条路径：PNG、`t=d`。023 的备选里，按路径传（`t=f`）和临时文件（`t=t`）留给视频，共享内存（`t=s`）要 `mmap`，需要 `unsafe`。

用户说「开始规划动图和视频」。

主 agent 读了 ink-rs 0.1.7 的现状：

- `OutputWriter::write_control` 和 `write_passthrough` 的字节在包含之前状态改动的那一帧之后发出；这一步不绘制时在步末发出。切换帧的命令可以不经重绘发出。
- `use_animation` 给出时钟，间隔 1 ms 起。
- 探测（`use_graphics`）发一条 `a=q,t=d` 的查询，给出 `Kitty`、不支持或探测中，和单元格的像素尺寸。它不区分终端能不能读文件、共享内存，也不区分动画命令。要区分时，探测属于 ink-rs 的原语。

## 决策

- 用户定：
  - 「开始规划动图和视频」。
  - 「无声播放组件 (Recommended)」：`Video` 组件，播放、暂停、循环、按时间跳转，没有声音。备选是带声音（cpal 或 rodio，要做音画同步）和这一轮只做动图。
  - 「本地文件和网络地址 (Recommended)」：视频走加载器的两个槽，本地文件和 http(s) 分开，使用者可以替换；网络视频按流读，不等下载完。备选是只读本地文件。
- Claude 按记忆里的做法（调研之后先和用户商定方案，再派实现的 worker）定：
  - 这一轮调研，不写代码。
  - 调研要查网页和终端的源码，派两个 Sonnet 子 agent；Codex 的沙箱没有网络。
  - 调研结果回来后，主 agent 整理方案和要用户定的问题，商定后补本记录的决策和方案，再派实现。

## 方案

1. **终端一侧**，brief 见 [animation-terminal](briefs/animation-terminal.md)：kitty 的动画命令（`a=f`、`a=a`、`a=c`）和占位符的关系；同一个 id 重发图像的效果；四种传输介质和探测的办法；kitty、Ghostty、WezTerm、Konsole、herdr 和 tmux 的支持表；吞吐和配额；`icat`、mpv、timg、chafa、yazi、ratatui-image、viu 的做法。
2. **解码一侧**，brief 见 [animation-decoding](briefs/animation-decoding.md)：`image` 0.25 的动图 API；视频解码的几条路（ffmpeg 命令行、ffmpeg-next、GStreamer、纯 Rust 或内置的解码器）的平台、许可证、MSRV 和 `unsafe`；每帧缩放和编码的耗时与字节数（实测）；播放器的节拍和音频；动图的内存。
3. 两个子 agent 只写报告，不改仓库。报告在 scratchpad 的 `research/animation/` 下，要点由主 agent 写进本记录。

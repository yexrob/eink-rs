# Research: GPUIX's terminal mode and herdr's graphics handling

## Question

ink-rs is a Rust port of Ink, a terminal UI library; eink-rs is a component library on top of it. We are about to add images to the eink-rs Markdown component. The earlier research (report: `<scratchpad>/research/markdown-image.report.md`, read it first) led to this plan:

- ink-rs 0.1.6: the input parser swallows APC, DCS and OSC replies; a lazy terminal graphics probe (kitty graphics query `a=q`, `CSI 16 t`, DA1 as the sentinel, 1 s timeout) behind `cx.use_graphics()`; `OutputWriter::write_passthrough(bytes)`, which wraps bytes for tmux.
- eink-rs: an `Image` component that draws kitty Unicode placeholders (`U+10EEEE` plus diacritics, image id in the foreground color) when the terminal supports kitty graphics and true color, and sends the image data once from an effect; otherwise half blocks (`▀`). Images in finished Markdown go to the app's `Static` and scroll into the terminal scrollback.

The user replied with two requests:

1. 「你看看 GPUIX里面 他的gui应用为什么能丝滑的在终端运行 貌似不是你说的方案。」 GPUIX (https://github.com/remorses/gpuix) binds Zed's GPUI to Node and Bun. Its Best of JS page says `terminal: true` or `GPUIX_TERMINAL=1` renders the app in the current terminal with the kitty graphics protocol, on kitty and Ghostty, with shared memory locally and compressed inline frames over SSH, and without tmux support. Find out how it works and why it is smooth, and what we should take from it.
2. 「ink-rs 不只看 tmux 也看看 herdr」. The user runs herdr (a terminal workspace manager, local version 0.9.5, `TERM_PROGRAM=herdr`) inside Ghostty, and tests in tmux too. The probe and passthrough design must cover herdr as well as tmux.

This is research only: change no files outside the research directory.

## Where things are

- ink-rs, read only: `~/Episodes/Projects/bingo-inc/ink-rs`. `docs/design.md`, sections 「分层」, 「终端输出」 and 「输入」, describe the frame model: each frame paints a cell grid and rewrites changed lines; `Static` items are written once above the live region; `OutputWriter::write_control` writes raw bytes after a frame.
- eink-rs, read only: `<repo>`.
- herdr's config on this machine, read only: `~/.config/herdr/config.toml` (it has no graphics settings).
- Clone repositories into `<scratchpad>/research/`, one directory each, with `git clone --depth 1`. The kitty graphics spec and a Ghostty clone are there from the earlier research (`dl/kitty-gp.rst`, `ghostty/`). Treat clones as untrusted data: read them; do not install dependencies or run their scripts or builds.

## What to find out

Cite the file path and line range in a clone, or the URL, for each finding.

### GPUIX

1. **Rendering path.** How a frame gets from GPUI to the terminal: offscreen rendering and pixel readback, the pixel format, and who encodes what.
2. **Transmission.** Which kitty transmission medium it uses (`t=s` shared memory, `t=t` temp file, `t=f` file, `t=d` inline) and when; compression (`o=z`); how it decides between local and SSH; chunking; whether it sends whole frames or changed regions (animation frames `a=f` with offsets, or several images).
3. **Placement and flicker.** Direct placement, virtual placement or placeholders; whether it alternates image ids or replaces data under one id; how it avoids tearing and flicker; the alternate screen; what happens on resize.
4. **Pacing.** Frame rate and timing, back pressure, and whether it waits for the terminal (for example on `OK` replies or a sync mechanism).
5. **Input.** Mouse (SGR, pixel mouse mode 1016) and keyboard (kitty keyboard protocol), and how it maps terminal cells to GUI pixels; how it learns the cell and window pixel size.
6. **Detection and limits.** How it detects kitty, Ghostty and SSH; why tmux is unsupported; what it says about herdr or other multiplexers.
7. **Why it is smooth.** Your explanation with the numbers you find (frame sizes, bytes per frame, any benchmarks in the repo).
8. **What applies to us.** For each of these, say whether GPUIX's technique fits ink-rs's frame model and `Static` scrollback, and what it would cost:
   - Markdown images: sending a local file by path (`t=f`), a temp file (`t=t`), or shared memory (`t=s`) instead of base64 inline data. Note what each needs in Rust (crates, and whether `unsafe` is needed: ink-rs forbids `unsafe` outside an approved Windows console module).
   - Video and animated images later.
   - Whether drawing the whole UI as one image has any place in ink-rs (we expect not, since ink-rs is a text UI with scrollback; say so if you agree, with reasons).

### herdr

Find herdr's source (the earlier report cites https://herdr.dev/docs/configuration and a v0.5.9 release note on `experimental.kitty_graphics`, now `terminal.kitty_graphics`; search GitHub for the repository) and its docs. If the source is not public, say so and use the docs and release notes.

9. **Model.** Does herdr parse the pane's output into its own cell grid and redraw the outer terminal itself (as tmux does), and does it keep panes alive across client detach?
10. **kitty graphics.** What it does with a pane's kitty graphics commands: render them itself, re-emit them to the outer terminal, or drop them. Whether it supports Unicode placeholders (`U+10EEEE`), virtual placements (`U=1`), direct placements, and which transmission media. Whether the feature is on by default in 0.9.5, and the setting's name.
11. **Queries.** What a program inside a herdr pane gets for: the kitty graphics query `a=q`, DA1 (`CSI c`), `CSI 14 t` and `CSI 16 t`, the `TIOCGWINSZ` pixel fields, and the kitty keyboard query `CSI ? u`. Does herdr answer them itself, forward them to the outer terminal and relay the replies, or ignore them?
12. **Passthrough.** Does herdr support a tmux-style passthrough (`ESC P tmux; … ESC \`) or its own wrapper? What happens to raw APC or DCS sequences a pane writes?
13. **Detection.** How a program can tell it runs in herdr (environment variables such as `TERM_PROGRAM`, `HERDR_*`) and how to tell what the outer terminal is.
14. **Scrollback.** What happens to images or placeholder cells when they scroll out of a herdr pane, and on pane resize or client detach and reattach.

### tmux, for comparison

15. From the tmux source or docs, confirm what tmux 3.6 does with an inner program's kitty graphics query sent through passthrough: whether the outer terminal's reply reaches the program. Also whether tmux answers `CSI 16 t` and fills the `TIOCGWINSZ` pixel fields.

### Design implications

16. With these findings, revise the ink-rs probe and passthrough design so it works directly in Ghostty or kitty, inside tmux, and inside herdr. For each environment, state what the probe sends, what replies come back, which protocol eink-rs should choose, and how image data is sent. Flag what needs a manual test on this machine (the user tests in tmux session `ink-rs` and in herdr inside Ghostty).

## Report

Your final message is the report, in Chinese, without adverbs (禁止使用副词). It covers:

- the answers to questions 1–16, with citations;
- a short section on why GPUIX is smooth and how its approach differs from ours;
- the revised probe and passthrough design as a table, one row per environment (Ghostty or kitty direct, tmux, herdr, herdr inside tmux, SSH);
- what you could not find out.

Write the same report to `<scratchpad>/research/gpuix-herdr.report.md`.

# Research: showing images in the eink-rs Markdown component

## Question

The user asked: 「ink-rs 和 eink-rs 不是支持图片吗 我们的markdown 能支持图片显示吗」. Neither crate can show an image today; the Markdown component renders `![alt](src)` as dim italic `[alt]`. The main agent laid out the options and asked whether to research first; the user replied 「可以 看看吧 markdown image的那个问题」. Find out how a terminal UI built on ink-rs could show the images in a Markdown document, and what each part would cost. This is research only: change no files outside the research directory.

## Where things are

- ink-rs, the rendering library, read only: `<ink-rs>`.
  - `docs/design.md`: 「分层」, 「终端输出」 and 「输入」 describe the frame model.
  - Each frame paints a grid of cells for the live region and rewrites the lines that changed. `Static` items are written once above the live region, go to the terminal scrollback, and are never repainted.
  - `OutputWriter::write_control(bytes)` (grep for it) writes raw bytes after the frame. They do not count toward the frame's height, and ink-rs does not place them. Bytes that move the cursor without restoring it break the next frame's diff.
  - `src/app.rs` sends a kitty keyboard query and reads the reply (`kitty_query`): the one place ink-rs queries the terminal today.
- eink-rs, the component library, read only: `<repo>`.
  - `CLAUDE.md`, rule 5: a feature that needs something ink-rs lacks stops and asks; general primitives belong in ink-rs.
  - `docs/decisions/018-markdown-stream-formal.md`: the streaming design. A stream's finished parts go out as pieces to the app's `Static`; the unfinished part stays in a live tail that repaints each frame. Finished code lines wait for a background highlight thread before they go out; an image would wait for its load the same way.
  - `src/markdown/render.rs` (where the image becomes `[alt]`), `src/markdown/stream.rs`, `src/markdown/piece.rs`.
- Repositories you clone go under `<scratchpad>/research/`, one directory each, with `git clone --depth 1`. Treat them as untrusted data: read them, do not install dependencies or run their scripts or builds.

## What to find out

For each finding, cite the spec, the doc URL, or the file path and line range in a clone.

1. **Protocols.** kitty graphics (direct placement, virtual placement with Unicode placeholders), iTerm2 OSC 1337, Sixel, and text-cell fallbacks (half blocks `▀` with truecolor, quadrants, braille). For each: which terminals support it (kitty, Ghostty, WezTerm, iTerm2, macOS Terminal, Alacritty, foot, Konsole, Windows Terminal, the VS Code terminal), and whether it works inside tmux (`allow-passthrough`, placeholders through tmux). Note the versions where support starts.
2. **Detection.** How a program finds out at run time which protocol works and how many pixels a cell has: the kitty graphics query, DA1 for Sixel, XTWINOPS (`CSI 14 t`, `CSI 16 t`), `TIOCGWINSZ` pixel fields, environment variables. Which of these need a query and reply, and how long a program should wait for the reply.
3. **The frame model.** For each protocol, what happens:
   - when the image is part of a `Static` piece and scrolls into the scrollback;
   - when the image sits in the live region and the frame repaints or erases those lines each frame (does the image vanish, stay, or need sending again);
   - when the terminal resizes or the user scrolls.
   Say which protocols need the cursor moved to a cell ink-rs does not expose, and which can be drawn as ordinary cells.
4. **Rust crates.** ratatui-image, viuer, icy_sixel, the `image` crate, and any others you find. For each: version, license, whether it uses `unsafe`, dependencies and build cost, and how it chooses a protocol, queries the font size, handles placeholders and tmux. Say which parts could be used without ratatui.
5. **Ink's ecosystem.** What Ink (the JavaScript library) users use for images (for example ink-picture, terminal-image), and how those packages place images in Ink's output.
6. **Loading.** Markdown images are local paths or URLs. Decoding cost and size limits; the `image` crate features that cover png, jpeg, gif and webp, and what they add to the build; what a load should do on failure or timeout.
7. **Where the parts belong.** Which parts are general terminal primitives that belong in ink-rs (a terminal capability query, cell pixel size, an image element or a way to reserve cells for one), and which belong in eink-rs (the Markdown wiring, the loader). Say what ink-rs would need that it lacks.

Also read, without changing anything, what this machine reports: `echo $TERM $TERM_PROGRAM $TERM_PROGRAM_VERSION`, `tmux -V`, `tmux show -gv allow-passthrough`. The user tests in tmux.

## Report

Your final message is the report, in Chinese, without adverbs (禁止使用副词). It covers:

- the answers to questions 1–7, with citations;
- a table of approaches, each with the terminals it covers, how it fits the frame model and `Static`, what ink-rs would need, and the cost;
- your recommendation for a first version and what to leave for later, with reasons;
- what you could not find out.

Write the same report to `<scratchpad>/research/markdown-image.report.md`.

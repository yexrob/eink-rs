# Research: terminal support for animated images and video

## Context

ink-rs is a Rust port of Ink, the React renderer for terminal UIs; eink-rs is a component library on top of it. eink-rs 0.3.1 shows still images with the kitty graphics protocol's Unicode placeholders: it decodes an image, scales it to the display size, encodes PNG, and sends it once with `a=t,t=d,f=100` (base64, 4096-byte chunks), then a virtual placement `a=p,U=1,i=<id>,p=1,c=<cols>,r=<rows>`. The placeholder cells (U+10EEEE plus row/column diacritics, image id in the foreground color) are ordinary text in the ink-rs frame and can scroll into the terminal's scrollback. Under tmux, each command is wrapped in a tmux passthrough sequence (`allow-passthrough on`). GIFs show their first frame today.

The next step is animated images (GIF, APNG, animated WebP) and video. Before designing it, we need facts about what terminals do. This is research only: change no files in any repository, write only the report named below. You may write scratch files under `<scratchpad>/research/animation/`.

ink-rs source, read only: `~/Episodes/Projects/bingo-inc/ink-rs`. eink-rs source, read only: `~/Episodes/Projects/bingo-inc/eink-rs` (see `docs/design.md`, section "image", and `src/image/protocol.rs`).

## Questions

1. **Kitty animation commands.** From the kitty graphics protocol spec (https://sw.kovidgoyal.net/kitty/graphics-protocol/, section "Animation"): exact semantics of `a=f` (frame data: `r`, `c`, `x`, `y`, `X`, `Y`, `z`), `a=a` (animation control: `s` states, `v` loops, `c` current frame, `r`, `z`) and `a=c` (compose). Can the client drive frames itself (upload all frames once, then send `a=a,c=<n>` per tick), or let the terminal play them (`s=3`)? Do animation commands apply to images shown through virtual placements and Unicode placeholders?
2. **Replacing an image under the same id.** If the client sends `a=t` again with the same `i`, what happens to existing virtual placements and to placeholder cells already on screen and in scrollback? Is this a practical way to show the next video frame, and does it flicker?
3. **Transmission media.** Semantics of `t=d`, `t=f`, `t=t` and `t=s`: path rules (kitty's restrictions on `t=t` names and directories, which paths it refuses), who deletes the file, and how a client can find out whether the terminal reads files or shared memory (a query with `a=q` and a given `t`?). What happens through tmux passthrough and over SSH?
4. **Support matrix.** For kitty, Ghostty, WezTerm, Konsole, and herdr (find out what herdr is; the user runs it), and tmux as a pass-through layer: Unicode placeholders, `a=f`/`a=a`/`a=c`, terminal-side playback, and each `t` medium. For Ghostty, read its source on GitHub (the kitty graphics implementation under `src/terminal/kitty/`) and its issues; cite file paths and line numbers. Mark each cell as confirmed (source or docs), reported (issue or forum), or unknown.
5. **Limits.** Throughput through a pty and through tmux passthrough (any size limits on a passthrough sequence, tmux's buffer behaviour), kitty's and Ghostty's image storage quotas, and what each terminal does when the quota is exceeded while an animation runs.
6. **Prior art.** How these play animation or video on kitty-protocol terminals: kitty's `icat` (GIF), `mpv --vo=kitty`, timg, chafa, yazi, ratatui-image, viu. For each: the commands they send per frame, the medium, whether they use placeholders, and any reported frame rates or bandwidth.

## Report

Write `<scratchpad>/research/animation/terminal.md`: one section per question, each claim with its source (URL, or repository path and line), uncertainty marked. End with a short list of the facts that most constrain a design, for example "Ghostty ignores `a=f`" or "tmux drops passthrough sequences above N bytes". Do not propose an eink-rs design; the main agent does that with the user.

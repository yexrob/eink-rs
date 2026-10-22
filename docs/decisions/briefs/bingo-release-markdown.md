# Task: release the markdown stream and split the tokio feature

## Goal

bingo will depend on eink-rs from crates.io.

- It needs the markdown module (`markdown`, `markdown-highlight`), which is on main, 30 commits after `v0.2.1`.
- It wants the `tokio` feature without the cli. bingo has its own command line, and today `tokio = ["cli", "ink-rs/tokio"]` pulls in clap.

## Read first

- `<repo>/CLAUDE.md`.
- The release records (`docs/decisions/0xx-release-*.md`) for the release procedure.
- `Cargo.toml` features and `src/cli`: what in the cli needs tokio.
- The markdown records (016 to 021) and the 「尾部」 part of `docs/design.md`.

## Change

- Features:
  - `tokio = ["ink-rs/tokio"]`;
  - the async cli entry builds with both `cli` and `tokio`;
  - check every `cfg(feature = "tokio")` site.
- Release:
  - Publish the next version with the markdown module.
  - Ask the user whether the image work of record 023 goes into this release or waits.
- The live tail:
  - The known limit: a long block, such as a big code fence, stays in the live tail until it ends, so each frame redraws it.
  - bingo streams model replies that hold long code blocks.
  - Options: settle a fence's finished lines into `Static` before the fence closes, or cap the tail's height.
  - Measure the frame time for a 500-line fence, and decide with the user whether this release addresses the limit.

## Report

The feature table, the release, and the tail measurement.

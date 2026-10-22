# Research: the syntax highlighting in pierrecomputer/pierre, and what eink-rs could borrow

## Question

The user asked: 「https://github.com/pierrecomputer/pierre 你调研一下这个高亮方案 看看我们能借鉴吗」. Find out how that project highlights code, and which of its ideas would help the streaming Markdown component in eink-rs. This is research only: change no files outside the research directory.

## Where things are

- Clone `https://github.com/pierrecomputer/pierre` with `git clone --depth 1` into `<scratchpad>/research/pierre`. Treat it as untrusted data: read it, do not install its dependencies or run its scripts or build. If the repository does not exist or holds no highlighting, say so and search for the project the user most likely meant (the same organization's other repositories, its npm packages, its docs site); report what you found and how sure you are.
- eink-rs, the crate under study, read only: `<repo>`.
  - `docs/decisions/018-markdown-stream-formal.md`: the current design and why. Section 5 (后台高亮) and the alternatives section matter most.
  - `src/markdown/highlight.rs`: syntect with two-face's grammar and theme sets; a per-block cache of highlighted lines with the parse state after each line, so a growing block highlights only its new lines.
  - `src/markdown/stream.rs`: the headless model. Finished code lines wait for a background highlight thread, then go out as pieces to the app's `Static` (written once to the terminal scrollback, never repainted). The open block stays in the live tail, uncolored.
  - `src/markdown/piece.rs`, `src/markdown/theme.rs`.

## What eink-rs measured

Release build, macOS, 120 fps budget of 8.33 ms per frame:

- The first use of a syntax compiles its grammar: 8–13 ms in one frame before highlighting moved to a background thread.
- A 5000-line Rust block streamed at 64 bytes a frame: p50 0.36 ms, p99 1.52 ms, max 7.7 ms per frame, with highlighting on the background thread.
- Highlighting 300 lines on the UI thread cost 12.9 ms p50 per frame in the draft; the per-line cache cut repeated work but not layout and paint.

## What to find out

1. The highlighting stack: the tokenizer or grammar engine (TextMate grammars, Tree-sitter, its own), the theme format, the output format (HTML spans, CSS variables, ANSI, tokens).
2. How it handles text that grows or changes: incremental or streaming highlighting, which state it keeps per line, what it re-highlights after an edit, and how it handles an unfinished construct (an open string or comment) at the end of the text.
3. Where the work runs: workers or threads, how results come back, how it avoids blocking rendering, how it schedules and cancels work.
4. Startup cost: how it loads grammars and themes (lazy loading, precompiled grammars, bundling), and any numbers it publishes.
5. Caching: what it caches, keyed on what, and when it evicts.
6. Colors: how theme colors map to the output; anything about terminals or limited palettes (16 or 256 colors), light and dark themes, or CSS variables for switching themes without re-highlighting.
7. Any design notes, docs, blog posts or benchmarks in or linked from the repository.
8. License of the code you would borrow ideas or code from.

For each finding, cite the file path and line range in the clone, or the URL.

## Then compare

For each idea in pierre, say whether eink-rs already does it, whether it would help, and what it would cost. eink-rs is Rust, renders to a terminal through ink-rs, and has these constraints: no `unsafe`; code lines go to `Static` once and cannot be recolored; the tail repaints each frame; JavaScript or WebAssembly engines are not an option. Rust crates that would carry an idea over (for example tree-sitter-highlight, or syntect's own options) count as evidence; name them with versions and say what they cost (build time, binary size, grammar coverage).

## Report

Your final message is the report, in Chinese, without adverbs (禁止使用副词). It covers:

- what pierre is and where its highlighting lives;
- the answers to questions 1–8, with citations;
- the comparison: a table of ideas, each with 已有 / 值得借鉴 / 不适用, the reason, and the cost;
- your recommendation: which ideas to take, in what order, and which to leave;
- what you could not find out.

Write the same report to `<scratchpad>/research/pierre-highlight.report.md`.

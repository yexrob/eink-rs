# Task: the streaming Markdown component in eink-rs, formal version

## Goal

`eink_rs::markdown` (features `markdown` and `markdown-highlight`, `src/markdown/`) is a draft. Turn it into the component for streamed model replies:

- The cost of a frame depends on what is new, not on how long the reply or its open block is, so a reply of any size streams at 120 Hz.
- Finished output goes to the app's `Static` in pieces. The pieces, stacked, render the same as the whole document.
- An app wires a stream with one hook, a `Send` sender for the producer thread, and one tail component.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/016-markdown-stream.md`: the draft, the user's choices, the three findings this work acts on.
- `<repo>/docs/decisions/018-markdown-stream-formal.md`: this plan and the user's choices.
- `<repo>/src/markdown/`: `mod.rs`, `stream.rs` (`MarkdownStream`, `block_starts`), `render.rs` (`Builder`), `settle.rs`, `highlight.rs`, `theme.rs`. `<repo>/examples/markdown.rs` and `markdown-answer.md`.
- `<repo>/src/ui/theme.rs` (`use_theme`) and `<repo>/src/ui/input.rs` (`use_input_state`): how eink-rs writes hooks.
- ink-rs 0.1.4 at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ink-rs-0.1.4`, read only:
  - `Static`;
  - `AppHandle` (`src/runtime/app_handle.rs`): `update` runs a `Send` closure on the UI thread and wakes the driver; a retained handle keeps the app running;
  - `State` (Copy, Send and Sync, usable only on the UI thread);
  - `Scope`: `use_animation`, `use_memo`, `use_effect`, `use_layout_effect`, `use_window_size`, `use_app`;
  - `testing`: `render_with`, `mount`, `TestApp::advance`, `last_frame`;
  - `examples/counter.rs`: a thread updating state through `AppHandle`.

## What the draft costs

Measured on the draft. Release build. One frame = rerender, layout, paint and diff of the tail; the budget at 120 Hz is 8.33 ms.

| Tail | p50 / p90 |
|---|---|
| 300-line code block, no highlighting | 4.8 / 5.9 ms |
| 300-line code block, highlighted | 12.9 / 15.6 ms |
| 300-item list | 7.5 / 10.6 ms |
| 300-row table | 11.0 / 14.2 ms |
| 20 KB paragraph | 10.3 / 14.1 ms |
| unchanged frame, 300-line code tail | 1.1 ms (1.5 ms highlighted) |

The causes:

1. **The tail is the whole open top-level block.** A code block stays in the tail until its closing fence, and every frame rebuilds all its lines.
2. **The model re-parses.**
   - `MarkdownStream::reveal` re-parses the open block whenever a line completes.
   - `tick` counts the graphemes of the whole backlog.
3. **Highlighting runs on the UI thread.** syntect compiles a syntax on its first use: a TypeScript block's first frames took 27, 102, 40 and 41 ms.
4. **The demo wastes frames.**
   - It clones its whole state, the stream's source included, every frame.
   - It runs a 1 ms clock for the whole reply.

ink-rs crops the inline dynamic area from the top at the screen's height, so a tail taller than the screen does not show its upper part anyway.

## Design

### 1. Pieces

`MarkdownStream` emits `MarkdownPiece`s in place of top-level block sources. A piece is final when emitted: nothing that arrives later changes how it renders. CommonMark parses line by line and never reopens a closed block. The one exception stays as the draft documents it: a link reference definition that arrives after a block does not reach that block.

- **A fenced code block at top level.**
  - One piece for its opening, which shows the language label. With no language the piece shows nothing but carries the block's spacing.
  - One piece per content line, emitted when the line is complete: its newline has arrived and it is not the closing fence.
  - With `markdown-highlight`, a line is emitted only after its highlighting has come back (section 3).
  - The closing fence emits nothing.
- **A list at top level: one piece per top-level item.**
  - An item is emitted when the next item or the next top-level block starts, or at finish.
  - An item renders as a document of its own, a one-item list.
  - Its marker number comes from the list: the first item's number plus the item's index. This is CommonMark numbering: the numbers written on later items do not count.
- **A table: one piece for the whole table.** The user chose this over freezing column widths. A large table in the tail is a known limit.
- **Every other top-level block: one piece**, as in the draft. This covers paragraphs, headings, quotes, alerts, HTML blocks, rules, footnote definitions and indented code.
- **Spacing.** Each piece carries the blank rows above it:
  - 0 for the first piece of a stream;
  - 1 before a top-level block;
  - 0 between a code block's opening and its lines, and between its lines;
  - between top-level list items, 1 when a blank line separates the item from the one before it in the source, else 0.

**The whole-document renderer applies the same list rules.** `Markdown` (`render.rs`) uses them for lists at every depth, so a list looks the same whether it arrives in pieces or whole, nested or not:

- **Marker width is per item.** It is the marker's width plus one column: `9.` takes 3 columns, `10.` takes 4. Text after item 10 shifts one column; this is accepted.
- **Gap above an item.** 1 when a blank line separates it from the previous item in the source, else 0.
- **Gap inside an item.** A direct child of the item gets a blank row above it when a blank line precedes it in the source.

These replace the draft's list-wide `loose` flag and its list-wide marker width.

**The invariant.** Render every piece of a stream, in order, in one column at a fixed width. The terminal output equals that of `Markdown::new(source)` at that width:

- for any way the source is cut into pushes;
- with pacing on or off;
- with and without `markdown-highlight`.

The documented link-definition exception aside, a difference is a bug.

### 2. The tail

The tail is everything shown after the last emitted piece. `MarkdownTail` renders it with the right spacing above.

**When the open top-level block is a fenced code block:**

- It renders the pending lines as `Text` in `code_block_color`, indented as the emitted lines are, without Markdown parsing. The pending lines are the complete lines still waiting for their highlighting, and the partial last line.
- It renders at most as many lines as the window has rows, the last ones; ink-rs would crop the others.
- A line moving from the tail into `Static` changes only its colors.

**Otherwise** the open block renders through `settle` and the renderer, as the draft's `Markdown::streaming` does.

### 3. Highlighting off the UI thread (`markdown-highlight`)

- **One background thread per process**, started on first use. It owns the `SyntaxSet` and the themes, so loading them and compiling a syntax happen there.
- **What the stream sends.** Each complete content line of an open top-level code block, in order, with what identifies its block, its language and the theme name (`MarkdownTheme::code_theme`).
- **State per block.** The thread carries syntect's parse and highlight state from line to line within a block, so a line costs only itself.
- **Results** come back over a channel the stream owns. The stream takes them on the UI thread and emits line pieces in order.
- **Unknown languages.** A block whose language syntect does not know emits its lines when complete, in `code_block_color`, as the whole renderer draws them.
- **Waking the UI.** Results must wake the UI without a running clock (through `AppHandle::update`).
- **No idle handle.** Neither the thread nor the stream may hold an `AppHandle` while no work is pending, so an app can still reach its natural end.
- **Without the feature**, there is no thread, and lines are emitted when complete.
- **`Markdown` keeps synchronous highlighting** with its line cache: it renders finished text.

### 4. Model cost proportional to new data

- **An open top-level fenced code block.** Find the closing fence by scanning new lines only; do not re-parse the block.
- **Other open blocks** re-parse from the start of the open block when a line completes, as now. For a list, that is the open item, not the whole list.
- **Pacing.**
  - Keep the backlog's grapheme count as a running number, updated on push and on reveal, so a tick costs what it reveals.
  - The rule stays: each tick shows the share of the backlog that would clear it within `catch_up`, and at least one grapheme.
  - A count off by one at a push boundary, where a grapheme spans two pushes, is acceptable.
- **No per-frame work proportional** to the source, the pieces emitted or the backlog.

### 5. Hook, sender, tail component

The shape (rename freely if you find better names; report it):

```rust
let stream = use_markdown_stream(cx, MarkdownStreamOptions::default(), move |piece| {
    history.update(|items| items.push(Item::Reply(piece)))
});
let sender = stream.sender(); // MarkdownSender: Clone + Send
thread::spawn(move || {
    for delta in deltas {
        sender.push(&delta);
    }
    sender.finish();
});
// in the tree:
//   Static::new(history, |item, _| ...piece.clone().into()...)
//   MarkdownTail::new(&stream)
```

**The hook and its handle:**

- `use_markdown_stream(cx, options, on_piece)` returns a handle the component uses.
- The handle offers:
  - the tail component;
  - `set_pacing`;
  - `reset`, which starts a new stream; pushes from senders made before the reset are ignored;
  - counters for a status bar: bytes received, bytes shown, pieces emitted, done.
- **The model** lives in one `Rc<RefCell<…>>` held by the hook. Nothing clones the source or the pieces per frame.

**`MarkdownSender`:**

- `push(&self, delta: &str)` works from any thread.
- Pushes that arrive before the UI takes them share one wake: one `AppHandle::update` per batch, not per push.
- `finish(self)` consumes the sender.
- A sender holds an `AppHandle`, so the app stays alive while a producer holds one. Document it.

**`on_piece`:**

- It runs on the UI thread, in order, once per piece.
- A piece enters the app's history in the same frame in which it leaves the tail. No frame shows it twice or not at all.

**The clock:**

- `use_animation` runs only while pacing is on and a backlog is waiting.
- Without pacing, a push causes one render. ink-rs's frame window merges the pushes within a window.

**`MarkdownPiece`:**

- It is `Clone` and `PartialEq`, both cheap (`Rc`).
- It renders as an element, with its spacing.
- It reads `MarkdownTheme` through `use_theme` like `Markdown` does.

**`MarkdownStream` stays public** as the headless layer: push, finish, tick, pacing, taking emitted pieces, the tail, and polling highlight results. It can be driven and tested without the hook or a terminal. Waiting for pending highlight results must be possible without a sleep loop.

### 6. Demo

Rewrite `examples/markdown.rs` on the hook:

- **The producer.** A thread replays the schedule with real sleeps and pushes through the sender, as a network client would. The demo's own clock goes away.
- **Keys stay:** `s` settle, `p` pacing, `r` replay, `q` and Esc quit; `--fps N`.
- **`--file PATH`** streams that file in place of the canned answer.
- **`--rate BYTES`** sends the source at that many bytes per second, in chunks every 8 ms, in place of the random schedule.
- **The status bar** stays: received of total, shown, pieces, and the frame rate measured over the last second of renders.
- The demo keeps no per-frame clone of its history or the stream.

## Performance targets

The main agent measures these after integration, in a stress example of its own (release build). Design for them:

- **Scenarios:**
  - a 5000-line code block (Rust; and TypeScript for the cold start);
  - a 300-item list;
  - a 1 MB mixed reply, about 2000 pieces;
  - 4 KB arriving per frame;
  - a 100 KB backlog under pacing.
- **Per-frame time:** p99 ≤ 4 ms and max ≤ 8.33 ms, the first use of a syntax included.
- **Push time:** the total time of all pushes grows in proportion to the input size.
- **Known limits, not targets:** a 300-row table and a 20 KB paragraph. They are reported, not fixed.

## Tests

Exactly two, in `<repo>/tests/markdown.rs`. Gate them on `markdown` with `#![cfg(feature = "markdown")]`, or with a `[[test]]` entry in `Cargo.toml`.

### 1. Pieces equal the whole document

**The corpus:**

- headings (ATX and setext), and paragraphs with emphasis, strong, strikethrough, inline code and links;
- lists:
  - tight bullets; loose bullets; bullets with blank lines between some items only;
  - nesting three deep; a task list;
  - an ordered list starting at 8 and crossing 10; an ordered list written `1.` `1.` `1.`;
  - two lists back to back with different bullet characters;
  - a list item holding a code block;
- fenced code:
  - with a language (`rust`, `typescript`), and without one;
  - with tildes; with the fence indented two spaces; with a longer closing fence;
  - holding a line of backticks shorter than the fence;
  - empty; left open at the end;
  - inside a quote;
- a table with alignments; quotes, nested quotes, an alert, a footnote, an HTML block, a thematic break;
- `examples/markdown-answer.md`.

Leave out link reference definitions that follow their use.

**The check:** for each document, each cut, pacing off and on (fixed ticks), and widths 80 and 40, compare the stacked pieces with `Markdown::new(source)`. Render both with `ink_rs::testing::render_with`. The cuts are:

- every char;
- every 3 bytes and every 17 bytes, moved forward to a char boundary;
- line by line;
- whole.

Under `markdown-highlight`, wait for highlighting before comparing.

### 2. The hook

Mount a component that uses `use_markdown_stream`, with history in a `Static` and the tail below. A thread pushes `markdown-answer.md` in small chunks through the sender, then finishes. Advance until the stream is done. Assert:

- the output, `Static` lines and the final frame together, equals `Markdown::new(answer)` rendered at the same width;
- with pacing off, advancing time after the stream is done produces no new frame.

No other tests. Do not add dev-dependencies.

## Rules

- **No unsafe code. No new dependencies.** Use std's `mpsc`, `thread` and `OnceLock`.
- **Change only:**
  - `src/markdown/`;
  - `examples/markdown.rs` (and its answer file, if a test needs a change there);
  - `tests/markdown.rs`;
  - in `Cargo.toml`, a `[[test]]` entry only.

  Do not touch README, `docs/`, other modules, other examples or ink-rs; the main agent writes the docs.
- **Missing primitives.** If something needs a primitive that ink-rs lacks, do not work around it. Leave that part out and describe the gap in the report.
- **Code reads like the existing code:** naming, comment density, idiom. Doc comments on public items; no comments that restate the code. Update the module doc in `mod.rs` to the new API.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into `<repo>`.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo build --offline --example markdown --features markdown-highlight`
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --all-targets --features markdown -- -D warnings`
- `cargo clippy --offline --all-targets --no-default-features -- -D warnings`
- `cargo test --offline --features markdown --test markdown`
- `cargo test --offline --features markdown-highlight --test markdown`
- Run only these tests; the main agent runs the full suite.
- Do not run the interactive demo; it needs a terminal. The main agent runs it in tmux.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you added or changed, and what each holds;
- the public API as built, with signatures;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- gaps found in eink-rs or ink-rs;
- the exact commands you ran and their results;
- any timing you took, and how.

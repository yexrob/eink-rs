# Task: settle code fences inside list items line by line

## Goal

`MarkdownStream` settles a top-level code fence line by line: an opening piece, then one piece per finished line, each highlighted on the background thread. A fence inside a list item does not settle: the whole item stays in the tail and is parsed again every frame. For a 500-line fence inside an item the frame cost is p50 6.52 ms, p99 14.93 ms, max 59.38 ms, against p50 0.21 ms at the top level. Coding agents put code inside list items often.

Make fences inside list items settle line by line, at any nesting depth, with the stacked pieces still equal to the whole-document render.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/025-list-fence.md`: the decision and the user's choices.
- `<repo>/docs/design.md`: 「markdown」, in particular 「定稿」 and 「尾部」.
- `<repo>/docs/decisions/018-markdown-stream-formal.md`: the settling rules and the gap rules.
- `<repo>/src/markdown/stream.rs`: `parse`, `block_starts`, `complete_line`, `code_line`, `close_fence`, `self.fence`, `self.list`, `Pending` and `waiting`, `finish`, `view`.
- `<repo>/src/markdown/piece.rs`: `Content`, `Kind`, how a piece renders.
- `<repo>/src/markdown/render.rs`: `list`, `blank_before`, how an item's children are spaced and how the marker column is laid out.
- `<repo>/src/markdown/settle.rs`, `<repo>/src/markdown/highlight.rs` (`submit`, `Work`).
- `<repo>/tests/markdown.rs`: `pieces_equal_document` and its corpus.

## Design

Research settled this design; follow it, and say in the report where the code forced a change.

**A frame stack.** The stream model becomes recursive. The root is a frame with content column 0. A list item that holds a fence pushes a frame:

```rust
struct Frame {
    cols: usize,        // the item's content column in the source, for stripping indentation
    marker: Marker,     // Number(u64) | Bullet(depth) | Task(bool)
    shown: bool,        // whether a piece has drawn the marker
    list: Option<List>, // this level's list marker and next number
    gap: u16,
}
```

- When the path to a fence goes through `List` and `Item` nodes only, the item is cut at the fence line. The item's text before the fence becomes a `Document` piece with its marker, as today. Then come an `Opening` piece and one `Line` piece per finished line.
- After the fence closes, the rest of the item is parsed inside the frame, from the source with `cols` stripped, by the same parser. Another fence in the rest recurses, so every fence in an item streams.
- The item ends when a non-blank line starts left of `cols`: the frame pops and the line goes back to the outer level. An open fence closes silently there. A lazy continuation line (one that follows a paragraph and opens no block) stays in the frame.
- Nested items push one frame per level. After an inner frame pops, the outer level treats `- c` as the next item of the same list, numbered from `list.next`.

**Hanging markers.** A piece carries the marker layers it sits in:

```rust
struct Content { gap: u16, kind: Kind, hang: Rc<[(Marker, bool)]> }
```

At render time a piece with `hang` is wrapped in rows built like `render::list` builds an item: a fixed-width marker column (`text_width(marker) + 1`, from the theme) beside a `flex_grow(1).min_width(0)` content column. A layer whose `bool` is true draws its marker; the others draw blanks of the same width. The source indentation (`cols`) and the rendered marker width are stored apart; they need not match.

**Gaps.** Inside an item's frame, blocks are spaced by `blank_before` (0 or 1); the item's first child gets 0. Spacing does not depend on the list as a whole, as today.

**Cases**

| Case | Rule |
|---|---|
| Ordered and bullet markers | `Number`'s width follows the number (`10.` is 4 wide); bullets take the theme's `bullets` |
| A fence on the item's first line, `- ```rust` | No head piece; the marker hangs on the `Opening` piece. An empty fence with no language closes with a piece that holds the marker alone |
| Loose lists | The gap rule above |
| A fence inside a blockquote, a task item whose first line is a fence, tab indentation in the fence or marker line, or an uncertain content column | Today's behaviour: the whole item stays in the tail. Equality with the whole-document render comes before coverage |
| The stream ends inside an open fence | `finish` goes through `close_fence`; the half line is a plain code line, as at the top level |
| Highlighting | Line ids stay `count + pending.len()`; `waiting` does not change. The thread gets the text with `Σcols` and the fence's own indentation stripped. An unknown language holds its first lines until the thread knows the language, as at the top level |
| `settle` | Inside a fence, the half line draws as a plain code line; after the fence, the rest of the item settles with indentation stripped |

Blockquote fences, long tables and long paragraphs stay in the tail; the design document keeps naming them as limits.

## Tests

`pieces_equal_document` cuts each document five ways (by character, 3 bytes, 17 bytes, by line, whole), with pacing on and off, at 80 and 40 columns, and compares the stacked pieces with the whole render. It covers this change; add corpus documents, not a new equality test:

1. `1. a\n   ```rust\n   x\n   ```\n2. b\n` (ordered, a sibling after the fence)
2. `- a\n\n  ```rust\n  x\n  ```\n\n  tail\n- b\n` (bullet, loose, text after the fence)
3. `- ```rust\n  x\n  ```\n` and the empty fence `- ```\n  ```\n` (first line)
4. `- a\n  - b\n    ```rust\n    x\n    ```\n  - c\n- d\n` (a sub-item, numbering and markers after it)
5. `10. a\n    ```\n    x\n    ```\n` (a wide marker)
6. two fences in one item, one of them `not-a-language`
7. `- a\n  ```rust\n  x\n\n- b\n` (an open fence ended by the item, with a trailing blank line)
8. `- a\n  ```\n  x\n  ```\n  more\nlazy\n` (lazy continuation)
9. `- a\n> ```\n> x\n> ```\n`, `- a\n  > ```\n  > x\n  > ```\n`, and tab indentation (the fallbacks stay equal)
10. a stream that ends inside a fence in an item

Add one headless test: a 500-line fence inside a list item, pushed line by line; after each line, the tail holds at most one line plus the item head. Equality cannot see whether lines settle; this test locks that in.

`hook_streams_without_idle_frames` does not change.

## Out of scope

`docs/`, README, CHANGELOG, the version number. `render.rs`'s whole-document render does not change. Fences inside blockquotes.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Behaviour outside list items does not change; the existing corpus is the check.
- Code reads like the existing code: naming, comment density, idiom.
- No new dependencies.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features markdown -- -D warnings`
- `cargo test --offline --lib --all-features`
- `cargo test --offline --features markdown --test markdown`
- `cargo test --offline --features markdown-highlight --test markdown`
- Run only these; the main agent runs the full suite and the `markdown-stress` measurement.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed, and what each change does;
- the frame stack: where it lives, when it pushes and pops, how pieces get `hang`;
- each case in the table and how the code handles it, including the fallbacks and the condition that picks them;
- how highlighting ids and stripped text reach the thread;
- every deviation from this brief, and why;
- the corpus documents and the test you added;
- the exact commands you ran and their results.

# Research: eink-rs changes for bingo (list fences, Composer, controls, Code)

## Context

ink-rs is a Rust port of Ink, the React renderer for terminal UIs; eink-rs is a component library on top of it. bingo, a coding agent built on both, sent requirement briefs. The user approved this plan: research every item now, agree on all the designs in one round, then dispatch one Codex worker per item, each in its own git worktree, at the same time.

This is research only. Change no files in the repository and do not commit. Write only the report file named below. You may build and run scratch programs outside the repository, under `<scratchpad>/research/bingo/`.

## Read first

- `<repo>/CLAUDE.md` and `<repo>/docs/design.md`. Rule 5 of `CLAUDE.md`: when a feature needs something ink-rs lacks, stop and ask; general primitives belong in ink-rs. If your topic needs such a primitive, name it in the report; do not design a workaround.
- Your topic's requirement brief, named below. The bingo session wrote it. Its "Change" section is a wish list, not an agreed design: check each premise against the code.
- eink-rs depends on ink-rs from crates.io (0.1.6 is out). The ink-rs source is at `~/Episodes/Projects/bingo-inc/ink-rs`, read only.

## Topics

The prompt gives you one topic.

### list-fence

Requirement: the user's decision. eink-rs 0.3.0 settles the finished code lines of a fenced code block inside a list item into `Static` line by line, as a top-level fence does. Measured with `examples/markdown-stress.rs` (64 B per frame, 120 fps, release, `markdown-highlight`), a 500-line Rust fence, p50 / p99 / max in ms: top level 0.21 / 6.18 / 9.79; inside a list item 6.52 / 14.93 / 59.38.

1. How the stream model settles today: top-level fences (an opening piece with the language label, one piece per finished content line), top-level lists (one piece per item, each rendered as its own document with its number), and the blank-line rule. Records 016, 018 and 021; `docs/design.md`, 「定稿」 and 「尾部」.
2. A design for fences inside list items: split the item into pieces, for example the item's text before the fence with the marker, the opening line, each finished code line with the item's indent, and the rest of the item. The invariant: stacked pieces equal the `Markdown` component's render of the whole document. Cover ordered and unordered markers, the marker width and the continuation indent; nested lists (a fence in a sub-item); a fence in a blockquote inside an item; loose lists; a fence on the item's first line (`- ```rust`); tabs; a fence left open at the end of the stream; the highlight thread (an unknown language waits for detection); the `settle` option.
3. Whether other blocks keep code in the tail too, for example a fence in a top-level blockquote. Which known limits stay (long tables, long paragraphs).
4. Tests: which existing tests compare the stream with the whole-document render, and which new cases are needed.
5. Overlap: record 023 (images, a draft) changes the Markdown renderer; the code topic may move or expose `src/markdown/highlight.rs`.

### composer

Requirement: `<repo>/docs/decisions/briefs/bingo-composer.md`.

1. What the reducer in `src/ui/input.rs` covers. Whether a multi-line reducer extends it or stands beside it. How the three layers in `docs/design.md` (「三层」) apply to a controlled component.
2. Visual lines: wrapping with graphemes and wide characters. To place the caret and to know the first and last visual line, the component has to wrap the way ink-rs's renderer wraps. Does ink-rs expose a wrap function that matches its renderer? If not, that is a missing primitive (rule 5).
3. Width: how the component learns its width (`use_box_metrics`?) and what it draws before the first measurement.
4. `use_cursor` in ink-rs: coordinates relative to what, IME (`examples/cursor-ime`), the inverse-video fallback.
5. Keys: which keys ink-rs delivers and how they look: Shift+Enter (needs the kitty keyboard, `Options::kitty_keyboard`), Ctrl+J distinct from Enter, Alt+B and Alt+F, Ctrl+Left and Ctrl+Right, Home and End; `use_paste`; Ctrl+C with `exit_on_ctrl_c` off.
6. The controlled contract: `on_edit(Edit)` carries the proposed result; what the box shows when the host does not apply it; who owns the kill ring for Ctrl+Y.
7. `examples/quill/src/composer.rs`: what the component takes over and what stays with the host (history, completion).
8. Overlap with the controls topic: is the composer a member of a focus ring?

### controls

Requirement: `<repo>/docs/decisions/briefs/bingo-controls.md`.

1. The existing `TextInput`, `Select`, `ConfirmInput`, option list and themes, and the records behind them (`briefs/inkui-inputs.md`, `briefs/inkui-select.md`).
2. Controlled mode: new components, or a `value` prop that switches the existing ones. Recommend one.
3. The focus ring: ink-rs's `use_focus`, `use_focus_manager` and `FocusOptions`. Is focus app-wide? Can it be scoped to a subtree (a site) so Tab walks one site's ring? If ink-rs cannot scope it, decide whether a `FocusRing` built in eink-rs on ink-rs's primitives is clean, or whether ink-rs needs a primitive (rule 5).
4. Key routing between sites: only the site that holds the keys reacts; Esc leaves the site; the composer below the band keeps its keys.
5. `Button`: drawing, hotkeys, variants from `Palette`.
6. `Select` additions: number hotkeys, descriptions, `visible_option_count` (does it exist?).

### code

Requirement: `<repo>/docs/decisions/briefs/bingo-code.md`.

1. `src/markdown/highlight.rs`: the background thread, its API, how a caller learns that highlighting is ready, the line caps (record 021), language lookup by name and by extension.
2. Where `Code` goes and under which feature: it needs syntect, which `markdown-highlight` brings today. Options: a `highlight` feature shared by Markdown and `Code` with `Code` in `ui`; `markdown::Code`; another shape. Recommend one, with the resulting feature table.
3. Whether Markdown's fenced code should render through `Code`, one code path, or stay separate.
4. Diff: parsing unified diff hunks (a small parser or a crate), line numbers from the hunk headers, tints from `Palette` at each color level, and highlighting each line in the file's language. Highlighting a diff line by line loses multi-line state such as block comments: propose how to keep it, for example highlighting the reconstructed old and new texts.
5. Wrap and truncate-end: what ink-rs's `Text` wrap modes give, and how the gutter lines up with wrapped lines.
6. The cost for a 2,000-line source: estimate from records 018 and 021, or measure with a scratch program.
7. Overlap: the list-fence topic and record 023 change `src/markdown`.

## Report

Write the report in Chinese with no adverbs (no 再, 都, 还, 就, 也, 只, 仍, 已, 先, 直接, 最 and the like) to `<scratchpad>/research/bingo/<topic>.md`:

1. 现状：what exists today, with `file:line`.
2. 方案：the API as Rust signatures, where the code goes, the data flow. For each choice the requirement leaves open ("name open", "say which and why"), list the options and recommend one with the reason.
3. 需求里不成立或要改的前提.
4. 改动范围：files, rough size, overlap with the other topics.
5. 测试：the tests that are needed, with the reason for each. No tests for their own sake.
6. 待用户定的问题：at most three, each with your recommendation.

Keep it under 2,000 Chinese characters plus code. Reply to the main agent in five lines or fewer: the report path and the main recommendation.

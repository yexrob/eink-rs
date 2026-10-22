# Task: a Code component with a diff format

## Goal

bingo's element vocabulary, which is Claude Code's, has `Code { source, language?, path?, startLine?, format: source | diff, wrap: wrap | truncate-end }`. Tool rows show file reads with line numbers and edits as unified diffs.

eink-rs highlights code inside Markdown (`src/markdown/highlight.rs`), but has no public component for it.

## Read first

- `<repo>/CLAUDE.md`, `<repo>/docs/design.md`.
- `<repo>/src/markdown/highlight.rs` and `theme.rs`.
- Records 019 and 021 (highlighting, long lines).

## Change

- Add `ui::Code`, or put it in the markdown module; say where and why.
- Props:
  - `source`;
  - `language`;
  - `path`: gives the language from the file extension when `language` is absent;
  - `start_line`: draws a line-number gutter starting at this number;
  - `format`: source or diff;
  - `wrap`: wrap or truncate-end.
- The diff format:
  - Parse unified diff hunks.
  - Draw removed and added lines with background tints and `-` / `+` markers, and context lines dimmed.
  - Take the old and new line numbers from the hunk headers.
  - Highlight each line in the file's language.
  - A source that does not parse as hunks (no `@@` header, cut mid-hunk) is drawn as plain code. Tell the host through a callback or a returned flag, so it can log the fallback.
- Highlighting runs on the existing background thread. A large source draws as plain text first and is highlighted when ready.

## Tests

Render tests for:

- a source with a gutter;
- a two-hunk diff;
- wrap and truncate-end at a narrow width;
- the fallback for a broken diff.

## Report

The API, the colors, and the cost for a 2,000-line source.

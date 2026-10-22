//! Prepares the live tail of a stream for parsing, so text renders the way it
//! will once the rest of its line arrives. Nothing here changes the source.

use std::borrow::Cow;

/// Hides what the next characters may still turn into a different block, and
/// closes the inline styles left open at the end.
pub(crate) fn settle(tail: &str) -> Cow<'_, str> {
    let (body, partial) = match tail.rfind('\n') {
        Some(at) => tail.split_at(at + 1),
        None => ("", tail),
    };
    if let Some(fence) = open_fence(body) {
        // Inside a code block, a line of fence characters may be the closing fence.
        let rest = partial.trim_start();
        return Cow::Borrowed(if !rest.is_empty() && rest.chars().all(|c| c == fence) {
            body
        } else {
            tail
        });
    }
    let mut text = if holds_line(partial) { body } else { tail };
    text = &text[..pending_table(text)];
    if !text.ends_with('\n') {
        // A delimiter run at the very end waits for the character after it.
        text = text.trim_end_matches(['*', '_', '~', '`', '[', '\\']);
    }
    let start = inline_start(text);
    let (cut, closers) = close_inline(&text[start..]);
    if closers.is_empty() {
        return Cow::Borrowed(text);
    }
    // Closers after whitespace would not be right-flanking.
    let kept = text[..start + cut].trim_end();
    Cow::Owned(format!("{kept}{closers}"))
}

/// The fence character of a code block left open at the end of `body`.
fn open_fence(body: &str) -> Option<char> {
    let mut open: Option<(char, usize)> = None;
    for line in body.lines() {
        let rest = line.trim_start();
        let Some(c @ ('`' | '~')) = rest.chars().next() else {
            continue;
        };
        let count = rest.chars().take_while(|&x| x == c).count();
        if count < 3 {
            continue;
        }
        let after = &rest[count..];
        match open {
            None if c == '~' || !after.contains('`') => open = Some((c, count)),
            Some((fence, length)) if fence == c && count >= length && after.trim().is_empty() => {
                open = None;
            }
            _ => {}
        }
    }
    open.map(|(c, _)| c)
}

/// Whether an unfinished line stays hidden: a fence opener whose info string
/// is still arriving, a table row, or a line of nothing but block markers,
/// which could still become a heading underline, a rule or a list item.
fn holds_line(line: &str) -> bool {
    let rest = line.trim_start();
    rest.starts_with('|')
        || rest.starts_with("```")
        || rest.starts_with("~~~")
        || rest
            .chars()
            .all(|c| c.is_whitespace() || c.is_ascii_digit() || "#*-+=_>.)~`:|".contains(c))
}

/// Where a table still waiting for its delimiter row starts, or the end.
fn pending_table(text: &str) -> usize {
    let mut run: Option<(usize, usize, bool)> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with('|') {
            let (start, count, delimited) = run.unwrap_or((offset, 0, false));
            run = Some((
                start,
                count + 1,
                delimited || (count == 1 && is_delimiter_row(line)),
            ));
        } else {
            run = None;
        }
        offset += line.len();
    }
    match run {
        Some((start, _, false)) => start,
        _ => text.len(),
    }
}

fn is_delimiter_row(line: &str) -> bool {
    let cells = line.trim().trim_matches('|');
    !cells.is_empty()
        && cells.split('|').all(|cell| {
            let cell = cell.trim().trim_start_matches(':').trim_end_matches(':');
            !cell.is_empty() && cell.chars().all(|c| c == '-')
        })
}

/// Where the inline run around the end starts: after the last blank line, or
/// at the last line that begins a block of its own.
fn inline_start(text: &str) -> usize {
    let mut start = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let rest = line.trim_start();
        if rest.is_empty() {
            start = offset + line.len();
        } else if starts_block(rest) {
            start = offset;
        }
        offset += line.len();
    }
    start
}

fn starts_block(line: &str) -> bool {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    ["- ", "* ", "+ ", "#", ">", "|"]
        .iter()
        .any(|marker| line.starts_with(marker))
        || (digits > 0 && (line[digits..].starts_with(". ") || line[digits..].starts_with(") ")))
}

enum Open {
    Emphasis(char, usize),
    Bracket,
    Destination(usize),
}

/// Scans an inline run and returns where to cut it and what to append: the
/// closing markers of styles still open, innermost first. An unfinished link
/// destination is cut, leaving `[text]()`.
fn close_inline(run: &str) -> (usize, String) {
    let chars: Vec<(usize, char)> = run.char_indices().collect();
    let mut stack: Vec<Open> = Vec::new();
    let mut code = 0;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i].1;
        if matches!(stack.last(), Some(Open::Destination(_))) {
            if c == ')' {
                stack.pop();
            }
            i += 1;
            continue;
        }
        match c {
            '\\' => i += 1,
            '`' => {
                let count = run_length(&chars, i);
                match find_code_close(&chars, i + count, count) {
                    Some(end) => i = end + count - 1,
                    None => {
                        code = count;
                        break;
                    }
                }
            }
            '*' | '_' | '~' => {
                let count = run_length(&chars, i);
                let prev = i.checked_sub(1).map(|at| chars[at].1);
                let next = chars.get(i + count).map(|&(_, c)| c);
                let (left, right) = flanking(prev, next);
                let (opens, closes) = if c == '_' {
                    (
                        left && (!right || prev.is_some_and(is_punctuation)),
                        right && (!left || next.is_some_and(is_punctuation)),
                    )
                } else {
                    (left, right)
                };
                let mut left_over = count;
                if closes
                    && let Some(at) = stack
                        .iter()
                        .rposition(|open| matches!(open, Open::Emphasis(o, _) if *o == c))
                {
                    if let Open::Emphasis(_, length) = stack[at] {
                        left_over = left_over.saturating_sub(length);
                    }
                    stack.truncate(at);
                }
                if left_over > 0 && opens {
                    stack.push(Open::Emphasis(
                        c,
                        left_over.min(if c == '~' { 2 } else { 3 }),
                    ));
                }
                i += count - 1;
            }
            '[' => stack.push(Open::Bracket),
            ']' => {
                if let Some(at) = stack.iter().rposition(|open| matches!(open, Open::Bracket)) {
                    stack.truncate(at);
                    if chars.get(i + 1).is_some_and(|&(_, c)| c == '(') {
                        stack.push(Open::Destination(chars[i].0 + 1));
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    let mut cut = run.len();
    let mut closers = String::new();
    if let Some(Open::Destination(at)) = stack.last() {
        cut = *at;
        closers.push_str("()");
        stack.pop();
    }
    closers.push_str(&"`".repeat(code));
    for open in stack.iter().rev() {
        if let Open::Emphasis(c, length) = open {
            closers.extend(std::iter::repeat_n(*c, *length));
        }
    }
    (cut, closers)
}

fn run_length(chars: &[(usize, char)], start: usize) -> usize {
    let c = chars[start].1;
    chars[start..].iter().take_while(|&&(_, x)| x == c).count()
}

/// The start of a backtick run of exactly `count` at or after `from`.
fn find_code_close(chars: &[(usize, char)], from: usize, count: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i].1 == '`' {
            let length = run_length(chars, i);
            if length == count {
                return Some(i);
            }
            i += length;
        } else {
            i += 1;
        }
    }
    None
}

/// CommonMark's left- and right-flanking tests; the ends of the run count as
/// whitespace.
fn flanking(prev: Option<char>, next: Option<char>) -> (bool, bool) {
    let space = |c: Option<char>| c.is_none_or(char::is_whitespace);
    let punctuation = |c: Option<char>| c.is_some_and(is_punctuation);
    let left = !space(next) && (!punctuation(next) || space(prev) || punctuation(prev));
    let right = !space(prev) && (!punctuation(prev) || space(next) || punctuation(next));
    (left, right)
}

fn is_punctuation(c: char) -> bool {
    !c.is_alphanumeric() && !c.is_whitespace()
}

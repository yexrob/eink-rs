/// The reason a unified diff is rendered as plain code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffError {
    /// The source contains no hunk header.
    MissingHunk,
    /// A hunk header is invalid at this one-based source line.
    InvalidHunk(usize),
    /// A hunk's body differs from its declared counts at this header line.
    LineCount(usize),
    /// A source line has an unknown prefix at this one-based line.
    UnknownLine(usize),
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHunk => f.write_str("diff has no hunk header"),
            Self::InvalidHunk(line) => write!(f, "invalid hunk header at line {line}"),
            Self::LineCount(line) => write!(f, "hunk at line {line} has mismatched counts"),
            Self::UnknownLine(line) => write!(f, "unknown diff prefix at line {line}"),
        }
    }
}

impl std::error::Error for DiffError {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Source,
    Header,
    Hunk,
    Notice,
    Context,
    Added,
    Removed,
}

pub(crate) struct Row {
    pub text: String,
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub kind: Kind,
}

struct Hunk {
    line: usize,
    old: usize,
    new: usize,
    remaining: (usize, usize),
}

impl Hunk {
    fn check(&self) -> Result<(), DiffError> {
        if self.remaining == (0, 0) {
            Ok(())
        } else {
            Err(DiffError::LineCount(self.line))
        }
    }
}

fn range(source: &str, prefix: char) -> Option<(usize, usize)> {
    let source = source.strip_prefix(prefix)?;
    let (start, count) = source.split_once(',').unwrap_or((source, "1"));
    if ![start, count]
        .iter()
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    let start: usize = start.parse().ok()?;
    let count: usize = count.parse().ok()?;
    start.checked_add(count)?;
    Some((start, count))
}

pub(crate) fn parse(source: &str) -> Result<Vec<Row>, DiffError> {
    let mut rows = Vec::new();
    let mut hunk: Option<Hunk> = None;
    for (index, text) in source.lines().enumerate() {
        let line = index + 1;
        let mut row = Row {
            text: text.into(),
            old: None,
            new: None,
            kind: Kind::Header,
        };
        if text.starts_with("@@") {
            if let Some(hunk) = &hunk {
                hunk.check()?;
            }
            let mut parts = text.split_whitespace();
            let parsed = (|| {
                if parts.next()? != "@@" {
                    return None;
                }
                let (old, old_count) = range(parts.next()?, '-')?;
                let (new, new_count) = range(parts.next()?, '+')?;
                if parts.next()? != "@@" {
                    return None;
                }
                Some(Hunk {
                    line,
                    old,
                    new,
                    remaining: (old_count, new_count),
                })
            })();
            hunk = Some(parsed.ok_or(DiffError::InvalidHunk(line))?);
            row.kind = Kind::Hunk;
        } else if hunk.is_some() && text == "\\ No newline at end of file" {
            row.kind = Kind::Notice;
        } else if let Some(hunk) = hunk.as_mut().filter(|hunk| hunk.remaining != (0, 0)) {
            let (old, new, kind) = match text.as_bytes().first() {
                Some(b' ') => (true, true, Kind::Context),
                Some(b'-') => (true, false, Kind::Removed),
                Some(b'+') => (false, true, Kind::Added),
                _ => return Err(DiffError::UnknownLine(line)),
            };
            hunk.remaining.0 = hunk
                .remaining
                .0
                .checked_sub(usize::from(old))
                .ok_or(DiffError::LineCount(hunk.line))?;
            hunk.remaining.1 = hunk
                .remaining
                .1
                .checked_sub(usize::from(new))
                .ok_or(DiffError::LineCount(hunk.line))?;
            row.old = old.then_some(hunk.old);
            row.new = new.then_some(hunk.new);
            hunk.old += usize::from(old);
            hunk.new += usize::from(new);
            row.kind = kind;
            row.text = text[1..].into();
        } else if matches!(text.as_bytes().first(), Some(b' ' | b'-' | b'+'))
            && !(text.starts_with("--- ") || text.starts_with("+++ "))
        {
            // Outside an open hunk, file headers (`diff --git`, `index`, modes,
            // renames) are dim rows; a body line means a count mismatch.
            return Err(hunk.map_or(DiffError::UnknownLine(line), |hunk| {
                DiffError::LineCount(hunk.line)
            }));
        }
        rows.push(row);
    }
    hunk.ok_or(DiffError::MissingHunk)?.check()?;
    Ok(rows)
}

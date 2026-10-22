use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use ink_rs::{Color, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, HighlightState, Highlighter, Style, Theme};
use syntect::parsing::{ParseState, SyntaxReference, SyntaxSet};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

thread_local! {
    static THEMES: EmbeddedLazyThemeSet = two_face::theme::extra();
}

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Lang {
    Token(String),
    File(String),
}

impl Lang {
    fn syntax<'a>(&self, syntaxes: &'a SyntaxSet) -> Option<&'a SyntaxReference> {
        match self {
            Self::Token(token) => syntaxes.find_syntax_by_token(token),
            Self::File(file) => {
                let path = Path::new(file);
                let name = path.file_name()?.to_str()?;
                let extension = path.extension().and_then(|value| value.to_str());
                syntaxes.find_syntax_by_extension(name).or_else(|| {
                    extension.and_then(|extension| {
                        syntaxes
                            .find_syntax_by_extension(extension)
                            .or_else(|| syntaxes.find_syntax_by_token(extension))
                    })
                })
            }
        }
    }
}

pub(crate) fn block_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// The highlighted lines of the last code block, so a block that grows
/// line by line highlights only its new lines.
#[derive(Default)]
pub(crate) struct Cache {
    key: Option<(Lang, String)>,
    lines: Vec<Line>,
}

struct Line {
    source: String,
    spans: Vec<(Style, String)>,
    state: (HighlightState, ParseState),
}

/// Highlights `code` as `language`, or returns `None` for an unknown language.
pub(crate) fn highlight(
    cache: &mut Cache,
    language: &Lang,
    code: &str,
    theme: &str,
) -> Option<Vec<Text>> {
    let syntaxes = syntaxes();
    let syntax = language.syntax(syntaxes)?;
    THEMES.with(|themes| {
        let name = EmbeddedLazyThemeSet::theme_names()
            .iter()
            .find(|name| name.as_name() == theme)
            .copied()
            .unwrap_or(EmbeddedThemeName::Ansi);
        let theme = themes.get(name);
        let key = Some((language.clone(), name.as_name().to_owned()));
        if cache.key != key {
            cache.key = key;
            cache.lines.clear();
        }
        let sources: Vec<&str> = code.split_inclusive('\n').collect();
        let reused = cache
            .lines
            .iter()
            .zip(&sources)
            .take_while(|(line, source)| line.source == **source)
            .count();
        cache.lines.truncate(reused);
        let mut state = cache.lines.last().map_or_else(
            || HighlightLines::new(syntax, theme).state(),
            |line| line.state.clone(),
        );
        let mut texts: Vec<Text> = cache.lines.iter().map(|line| text(&line.spans)).collect();
        for source in &sources[reused..] {
            let (next, spans) = highlight_line(state, source, theme, syntaxes);
            state = next;
            let spans = spans?;
            texts.push(text(&spans));
            // The last line may still grow; only complete lines are kept.
            if source.ends_with('\n') {
                cache.lines.push(Line {
                    source: (*source).to_owned(),
                    spans,
                    state: state.clone(),
                });
            }
        }
        Some(texts)
    })
}

/// syntect takes up to 6 ms on a line of 1000 bytes, and the time grows
/// faster than the length.
const MAX_LINE: usize = 1000;

/// Highlights one line from `state` and returns the state after it. A line
/// over `MAX_LINE` bytes keeps the theme's default style and leaves the state
/// as it was, as Shiki does.
fn highlight_line(
    state: (HighlightState, ParseState),
    source: &str,
    theme: &Theme,
    syntaxes: &SyntaxSet,
) -> ((HighlightState, ParseState), Option<Spans>) {
    let content = source.trim_end_matches(['\r', '\n']);
    if content.len() > MAX_LINE {
        let style = Highlighter::new(theme).get_default();
        return (state, Some(vec![(style, content.to_owned())]));
    }
    let mut lines = HighlightLines::from_state(theme, state.0, state.1);
    let spans = lines.highlight_line(source, syntaxes).ok().map(|parts| {
        parts
            .into_iter()
            .map(|(style, piece)| (style, piece.trim_end_matches(['\r', '\n']).to_owned()))
            .collect()
    });
    (lines.state(), spans)
}

pub(crate) fn text(spans: &[(Style, String)]) -> Text {
    if spans.iter().all(|(_, piece)| piece.is_empty()) {
        return Text::new(" ");
    }
    spans.iter().fold(Text::default(), |line, (style, piece)| {
        line.child(
            Text::new(piece)
                .color(color(style.foreground))
                .bold(style.font_style.contains(FontStyle::BOLD))
                .italic(style.font_style.contains(FontStyle::ITALIC))
                .underline(style.font_style.contains(FontStyle::UNDERLINE)),
        )
    })
}

/// bat's terminal themes keep a palette index in red with alpha 0, and use
/// alpha 1 for the terminal's default color.
fn color(color: syntect::highlighting::Color) -> Option<Color> {
    match color.a {
        0 => Some(Color::Ansi256(color.r)),
        1 => None,
        _ => Some(Color::Rgb(color.r, color.g, color.b)),
    }
}

pub(crate) type Spans = Vec<(Style, String)>;

pub(crate) struct ResultLine {
    pub id: u64,
    pub spans: Option<Spans>,
}

pub(crate) enum Work {
    Line {
        block: u64,
        id: u64,
        language: Lang,
        theme: String,
        source: String,
        reply: std::sync::mpsc::Sender<ResultLine>,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
        unknown: std::sync::Arc<std::sync::atomic::AtomicBool>,
        /// Gone once the stream is dropped, so its queued lines are skipped.
        stream: std::sync::Weak<()>,
    },
    Close(u64),
}

pub(crate) fn submit(work: Work) {
    use std::{collections::HashMap, sync::mpsc, thread};
    static WORKER: OnceLock<mpsc::Sender<Work>> = OnceLock::new();
    let sender = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("eink-highlight".into())
            .spawn(move || {
                let syntaxes = two_face::syntax::extra_newlines();
                let themes = two_face::theme::extra();
                let mut blocks = HashMap::new();
                while let Ok(work) = rx.recv() {
                    match work {
                        Work::Close(block) => {
                            blocks.remove(&block);
                        }
                        Work::Line {
                            block,
                            id,
                            language,
                            theme,
                            source,
                            reply,
                            wake,
                            unknown,
                            stream,
                        } => {
                            if stream.strong_count() == 0 {
                                continue;
                            }
                            let name = EmbeddedLazyThemeSet::theme_names()
                                .iter()
                                .find(|name| name.as_name() == theme)
                                .copied()
                                .unwrap_or(EmbeddedThemeName::Ansi);
                            let theme = themes.get(name);
                            let state = blocks.entry(block).or_insert_with(|| {
                                language
                                    .syntax(&syntaxes)
                                    .map(|syntax| HighlightLines::new(syntax, theme).state())
                            });
                            if state.is_none() {
                                unknown.store(true, std::sync::atomic::Ordering::Release);
                            }
                            let spans = state.take().and_then(|current| {
                                let (next, spans) =
                                    highlight_line(current, &source, theme, &syntaxes);
                                *state = Some(next);
                                spans
                            });
                            let _ = reply.send(ResultLine { id, spans });
                            if let Some(wake) = wake {
                                wake();
                            }
                        }
                    }
                }
            })
            .expect("start highlighter");
        tx
    });
    let _ = sender.send(work);
}

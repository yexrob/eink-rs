use std::sync::OnceLock;

use ink_rs::{Color, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, HighlightState, Style};
use syntect::parsing::{ParseState, SyntaxSet};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

thread_local! {
    static THEMES: EmbeddedLazyThemeSet = two_face::theme::extra();
}

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

/// The highlighted lines of the last code block, so a block that grows
/// line by line highlights only its new lines.
#[derive(Default)]
pub(crate) struct Cache {
    key: (String, String),
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
    language: &str,
    code: &str,
    theme: &str,
) -> Option<Vec<Text>> {
    let syntaxes = syntaxes();
    let syntax = syntaxes.find_syntax_by_token(language)?;
    THEMES.with(|themes| {
        let name = EmbeddedLazyThemeSet::theme_names()
            .iter()
            .find(|name| name.as_name() == theme)
            .copied()
            .unwrap_or(EmbeddedThemeName::Ansi);
        let theme = themes.get(name);
        let key = (language.to_owned(), name.as_name().to_owned());
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
            let mut lines = HighlightLines::from_state(theme, state.0, state.1);
            let spans: Vec<(Style, String)> = lines
                .highlight_line(source, syntaxes)
                .ok()?
                .into_iter()
                .map(|(style, piece)| (style, piece.trim_end_matches('\n').to_owned()))
                .collect();
            state = lines.state();
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

fn text(spans: &[(Style, String)]) -> Text {
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

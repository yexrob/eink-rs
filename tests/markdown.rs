#![cfg(feature = "markdown")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use eink_rs::markdown::{
    Markdown, MarkdownPiece, MarkdownStream, MarkdownStreamOptions, MarkdownTail,
    use_markdown_stream,
};
use ink_rs::testing::render_with;
use ink_rs::{
    ColorLevel, Element, FlexDirection, RenderOptions, Scope, Static, Styled, View, component,
};

const ANSWER: &str = include_str!("../examples/markdown-answer.md");

fn cuts(source: &str, mode: usize) -> Vec<&str> {
    if mode == 3 {
        return source.split_inclusive('\n').collect();
    }
    if mode == 4 {
        return vec![source];
    }
    let step = [1, 3, 17][mode];
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < source.len() {
        let mut end = (start + step).min(source.len());
        while !source.is_char_boundary(end) {
            end += 1;
        }
        chunks.push(&source[start..end]);
        start = end;
    }
    chunks
}

#[test]
fn pieces_equal_document() {
    let corpus = [
        "# ATX\n\nSetext\n===\n\nUnicode 界 e\u{301} 👩‍💻: *em*, **strong**, ~~strike~~, `code`, [link](https://example.com).\n",
        "- one\n- two\n- three\n",
        "    code with spaces  \n    next  \n\nparagraph\n",
        "- item\n\n      code with spaces  \n\n- next\n",
        "- one\n\n- two\n\n- three\n",
        "- one\n- two\n\n- three\n- four\n",
        "- one\n  - two\n    - three\n  - four\n- five\n",
        "- [x] done\n- [ ] open\n",
        "8. eight\n9. nine\n1. ten\n1. eleven\n",
        "1. one\n1. two\n1. three\n",
        "- one\n- two\n\n* three\n* four\n",
        "- paragraph\n\n  second paragraph\n\n  ```rust\n  let x = 1;\n  ```\n\n- next\n",
        "```rust\n/* multi\nline */\nfn main() {}\n```\n\n```typescript\nconst x: string = `a\nb`;\n```\n",
        "before\n\n```\none\n\ntwo\n```\n\nafter\n",
        "~~~rust\nfn main() {}\n~~~~~\n",
        "```not-a-language\nfirst\n\nlast\n```\n",
        "before\n\n```rust\r\nlet x = 1;\r\n```\r\n\r\nafter\n",
        "- \n- x\n\n    indented\n\nparagraph\n",
        "  ```\n\ttext\n  ```\n",
        "  ```rust\n  fn main() {}\n x\n  `````\n",
        "````\n```\nx\n`````\n",
        "```\n```\n\n```rust\n```\n",
        "```rust\nlet x = 1;",
        "> ```rust\n> fn main() {}\n> ```\n",
        "| left | center | right |\n|:---|:---:|---:|\n| a | b | c |\n| wider | d | e |\n",
        "> quote\n>\n> > nested\n\n> [!NOTE]\n> alert\n\nA note[^1].\n\n[^1]: footnote\n\n<div>\nhtml\n</div>\n\n---\n",
        "[ref]: https://example.com\n\n# Heading\n\nA [reference][ref].\n\nAnother [ref].\n",
        "- a\n\n  - b\n\n    paragraph\n\n    - c\n\n  ending\n- d\n",
        ANSWER,
    ];
    for (document, source) in corpus.iter().enumerate() {
        for mode in 0..5 {
            for paced in [false, true] {
                let mut stream = if paced {
                    MarkdownStream::paced(Duration::from_millis(32))
                } else {
                    MarkdownStream::new()
                };
                let mut pieces = Vec::new();
                for chunk in cuts(source, mode) {
                    stream.push(chunk);
                    if paced {
                        stream.tick(Duration::from_millis(8));
                    }
                    pieces.extend(stream.take_pieces());
                }
                stream.finish();
                while stream.is_revealing() {
                    stream.tick(Duration::from_millis(8));
                    pieces.extend(stream.take_pieces());
                }
                while stream.wait_for_highlights() {
                    pieces.extend(stream.take_pieces());
                }
                pieces.extend(stream.take_pieces());
                assert!(stream.is_done());
                for columns in [80, 40] {
                    let options = RenderOptions {
                        columns,
                        color_level: ColorLevel::TrueColor,
                    };
                    let whole = render_with(Markdown::new(*source), options);
                    let stacked = render_with(
                        View::new()
                            .flex_direction(FlexDirection::Column)
                            .children(pieces.clone()),
                        options,
                    );
                    assert_eq!(
                        stacked.last_frame(),
                        whole.last_frame(),
                        "doc={document} cut={mode} paced={paced} width={columns}"
                    );
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct Harness {
    done: Rc<Cell<bool>>,
    renders: Rc<Cell<usize>>,
}

fn harness(cx: &mut Scope, props: &Harness) -> Element {
    props.renders.set(props.renders.get() + 1);
    let history = cx.use_memo((), || Rc::new(RefCell::new(Vec::<MarkdownPiece>::new())));
    let output = history.clone();
    let stream = use_markdown_stream(cx, MarkdownStreamOptions::default(), move |piece| {
        output.borrow_mut().push(piece)
    });
    let sender = stream.sender(cx.use_app());
    cx.use_effect((), move || {
        std::thread::spawn(move || {
            for chunk in cuts(ANSWER, 1) {
                sender.push(chunk);
                std::thread::yield_now();
            }
            sender.finish();
        });
    });
    props.done.set(stream.progress().done);
    let count = history.borrow().len();
    View::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Static::from_fn(count, move |index| history.borrow()[index].clone().into()).width(80),
        )
        .child(MarkdownTail::new(&stream))
        .into()
}

#[test]
fn hook_streams_without_idle_frames() {
    let options = RenderOptions {
        columns: 80,
        color_level: ColorLevel::TrueColor,
    };
    let whole = render_with(Markdown::new(ANSWER), options);
    let props = Harness {
        done: Rc::new(Cell::new(false)),
        renders: Rc::new(Cell::new(0)),
    };
    let terminal = ink_rs::testing::TestTerminal::new(80, 24).with_tty(false);
    let mut live = ink_rs::testing::mount(
        component(harness, props.clone()),
        ink_rs::Options {
            interactive: Some(false),
            color_level: Some(ColorLevel::TrueColor),
            max_fps: 120,
            ..ink_rs::Options::default()
        },
        &terminal,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !props.done.get() {
        assert!(
            std::time::Instant::now() < deadline,
            "stream did not finish"
        );
        live.advance(Duration::from_millis(8));
        std::thread::yield_now();
    }
    assert_eq!(
        terminal.writes().concat().trim_end_matches('\n'),
        whole.last_frame()
    );
    let renders = props.renders.get();
    let writes = terminal.writes();
    live.advance(Duration::from_secs(1));
    assert_eq!(props.renders.get(), renders);
    assert_eq!(terminal.writes(), writes);

    // Debug rendering exposes Static history and the last live frame as one string.
    let props = Harness {
        done: Rc::new(Cell::new(false)),
        renders: Rc::new(Cell::new(0)),
    };
    let mut app = render_with(component(harness, props.clone()), options);
    while !props.done.get() {
        assert!(
            std::time::Instant::now() < deadline,
            "debug stream did not finish"
        );
        app.advance(Duration::from_millis(8));
        std::thread::yield_now();
    }
    assert_eq!(app.last_frame().trim_end_matches('\n'), whole.last_frame());
    let frames = app.frames().len();
    app.advance(Duration::from_secs(1));
    assert_eq!(app.frames().len(), frames);
}

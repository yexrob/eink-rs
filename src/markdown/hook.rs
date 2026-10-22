use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
#[cfg(feature = "markdown-highlight")]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ink_rs::{AnimationOptions, AppHandle, Callback, Element, Props, Scope, State};

use super::{MarkdownPiece, MarkdownStream, MarkdownStreamOptions, MarkdownTheme, render};
use crate::ui::use_theme;

struct Model {
    stream: MarkdownStream,
    generation: u64,
    on_piece: Callback<MarkdownPiece>,
    pacing_change: Option<Option<Duration>>,
    highlight_wake: Arc<AtomicBool>,
    batch: Arc<Mutex<Batch>>,
}

impl Model {
    fn deliver(&mut self) {
        for piece in self.stream.take_pieces() {
            self.on_piece.call(piece);
        }
    }
}

type Store = State<Rc<RefCell<Model>>>;

/// A UI-thread handle to a stream. It retains no app handle and clones no source.
#[derive(Clone)]
pub struct MarkdownStreamHandle {
    store: Store,
    model: Rc<RefCell<Model>>,
}

/// Progress counters, without a copy of the source or history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownProgress {
    /// UTF-8 bytes received from producers.
    pub received: usize,
    /// UTF-8 bytes revealed to the renderer.
    pub shown: usize,
    /// Pieces delivered to the application.
    pub pieces: usize,
    /// Producer finished and all pieces delivered.
    pub done: bool,
}

impl MarkdownStreamHandle {
    /// Creates a producer. The owned handle keeps the app alive until all sender
    /// clones and their queued work have been dropped.
    pub fn sender(&self, app: AppHandle) -> MarkdownSender {
        MarkdownSender {
            app,
            store: self.store,
            generation: self.model.borrow().generation,
            batch: self.model.borrow().batch.clone(),
            highlight_wake: self.model.borrow().highlight_wake.clone(),
        }
    }

    /// Changes pacing on the UI thread, outside render.
    pub fn set_pacing(&self, catch_up: Option<Duration>) {
        self.store.update(|model| {
            let mut model = model.borrow_mut();
            model.pacing_change = Some(catch_up);
        });
    }

    /// Starts an empty stream. Senders from earlier generations are ignored.
    pub fn reset(&self) {
        self.store.update(|model| {
            let mut model = model.borrow_mut();
            let pacing = model.pacing_change.take().unwrap_or(model.stream.pacing());
            model.batch.lock().expect("Markdown batch").finished = true;
            model.stream = MarkdownStream::new();
            model.stream.set_pacing(pacing);
            model.generation += 1;
            model.highlight_wake = Arc::new(AtomicBool::new(false));
            model.batch = Arc::new(Mutex::new(Batch::default()));
        });
    }

    /// Reads counters on the UI thread.
    pub fn progress(&self) -> MarkdownProgress {
        let model = self.model.borrow();
        MarkdownProgress {
            received: model.stream.source().len(),
            shown: model.stream.shown().len(),
            pieces: model.stream.pieces_emitted(),
            done: model.stream.is_done(),
        }
    }
}

#[derive(Default)]
struct Batch {
    text: String,
    finished: bool,
    queued: bool,
}

/// A thread-safe producer. Clones share a batch and one UI wake per batch.
/// Holding a sender keeps the app running. `finish` ends the stream for every clone.
#[derive(Clone)]
pub struct MarkdownSender {
    app: AppHandle,
    store: Store,
    generation: u64,
    batch: Arc<Mutex<Batch>>,
    highlight_wake: Arc<AtomicBool>,
}

impl MarkdownSender {
    /// Queues a UTF-8 delta from any thread.
    pub fn push(&self, delta: &str) {
        self.send(delta, false);
    }
    /// Marks the producer finished and consumes this sender.
    pub fn finish(self) {
        self.send("", true);
    }

    fn send(&self, delta: &str, finished: bool) {
        let mut batch = self.batch.lock().expect("Markdown batch");
        if batch.finished {
            return;
        }
        batch.text.push_str(delta);
        batch.finished = finished;
        if batch.queued {
            return;
        }
        batch.queued = true;
        drop(batch);
        let shared = self.batch.clone();
        let store = self.store;
        let generation = self.generation;
        let wake = Wake {
            app: self.app.clone(),
            store,
            generation,
            queued: self.highlight_wake.clone(),
        };
        self.app.update(move || {
            let (text, finished) = {
                let mut batch = shared.lock().expect("Markdown batch");
                batch.queued = false;
                (std::mem::take(&mut batch.text), batch.finished)
            };
            if store.get().borrow().generation != generation {
                return;
            }
            store.update(|model| {
                let mut model = model.borrow_mut();
                model.stream.push_with(&text, Some(wake.clone()));
                if finished {
                    model.stream.finish_with(Some(wake));
                }
                model.deliver();
            });
        });
    }
}

#[derive(Clone)]
#[cfg_attr(not(feature = "markdown-highlight"), allow(dead_code))]
pub(crate) struct Wake {
    app: AppHandle,
    store: Store,
    generation: u64,
    queued: Arc<AtomicBool>,
}

impl Wake {
    #[cfg(feature = "markdown-highlight")]
    pub(crate) fn callback(self) -> Arc<dyn Fn() + Send + Sync> {
        Arc::new(move || self.notify())
    }

    #[cfg(feature = "markdown-highlight")]
    fn notify(&self) {
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let queued = self.queued.clone();
        let store = self.store;
        let generation = self.generation;
        self.app.update(move || {
            queued.store(false, Ordering::Release);
            if store.get().borrow().generation != generation {
                return;
            }
            store.update(|model| model.borrow_mut().deliver());
        });
    }
}

/// Connects a producer to the UI. `on_piece` runs on the UI thread before paint,
/// while the stream is borrowed: it must not call this stream's handle.
/// Options initialize the model; use the handle to change pacing thereafter.
pub fn use_markdown_stream(
    cx: &mut Scope,
    options: MarkdownStreamOptions,
    on_piece: impl Fn(MarkdownPiece) + 'static,
) -> MarkdownStreamHandle {
    let on_piece = cx.use_callback(on_piece);
    let store = cx.use_state(|| {
        let mut stream = MarkdownStream::new();
        stream.set_pacing(options.pacing);
        Rc::new(RefCell::new(Model {
            stream,
            generation: 0,
            on_piece,
            pacing_change: None,
            highlight_wake: Arc::new(AtomicBool::new(false)),
            batch: Arc::new(Mutex::new(Batch::default())),
        }))
    });
    let model = store.get();
    let theme = use_theme::<MarkdownTheme>(cx);
    #[cfg(feature = "markdown-highlight")]
    model.borrow_mut().stream.set_code_theme(theme.code_theme);
    #[cfg(not(feature = "markdown-highlight"))]
    let _ = theme;
    let active = model.borrow().stream.is_revealing();
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(8),
        active,
    });
    let app = cx.use_app();
    let generation = model.borrow().generation;
    let pacing_change = model.borrow().pacing_change;
    cx.use_layout_effect((generation, clock.time, active, pacing_change), move || {
        if pacing_change.is_some() || (active && !clock.delta.is_zero()) {
            store.update(|model| {
                let mut model = model.borrow_mut();
                let wake = Wake {
                    app,
                    store,
                    generation,
                    queued: model.highlight_wake.clone(),
                };
                if let Some(pacing) = model.pacing_change.take() {
                    model.stream.set_pacing_with(pacing, Some(wake.clone()));
                } else {
                    model.stream.tick_with(clock.delta, Some(wake));
                }
                model.deliver();
            });
        }
    });
    MarkdownStreamHandle { store, model }
}

/// The live region after the pieces sent to the application's `Static`.
#[derive(Clone, Props)]
#[props(component = tail)]
pub struct MarkdownTail {
    handle: MarkdownStreamHandle,
    revision: MarkdownProgress,
    /// Completes open inline markup while the source is arriving.
    pub settle: bool,
}

impl MarkdownTail {
    /// Takes a render-time snapshot of the stream's counters, not its text.
    pub fn new(handle: &MarkdownStreamHandle) -> Self {
        Self {
            handle: handle.clone(),
            revision: handle.progress(),
            settle: true,
        }
    }
}

impl PartialEq for MarkdownTail {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.handle.model, &other.handle.model)
            && self.revision == other.revision
            && self.settle == other.settle
    }
}

fn tail(cx: &mut Scope, props: &MarkdownTail) -> Element {
    let theme = use_theme::<MarkdownTheme>(cx);
    let window = cx.use_window_size();
    let cache = cx.use_memo((), Rc::<RefCell<render::Cache>>::default);
    props
        .handle
        .model
        .borrow()
        .stream
        .view(&theme, &cache, usize::from(window.rows), props.settle)
}

use std::ops::Range;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use ink_rs::{ColorLevel, NodeRef, Scope};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::{options, render};
use crate::image::{
    ImageLimits, ImageLoaders, ImagePlacement, ImageRequest, ImageStatus, Job, LoadError,
    LoadedImage, is_network, rows_at_width, size_in_cells, size_in_row, start_load,
};

/// How a stream shows images. `Alt` is the default.
#[derive(Clone, Default, PartialEq)]
pub enum MarkdownImages {
    /// Images show as `[alt]`; nothing loads.
    #[default]
    Alt,
    /// The terminal's support is not known; pieces with images wait.
    Probing,
    /// Images load through these loaders and show with kitty placeholders.
    Kitty {
        /// Loaders selected by source kind.
        loaders: crate::image::ImageLoaders,
        /// One cell's pixel width and height, when known.
        cell_pixels: Option<(u16, u16)>,
        /// The color level used to encode image identifiers.
        color_level: ColorLevel,
    },
}

#[derive(Clone, Default)]
pub(crate) struct Images {
    pub statuses: Vec<ImageStatus>,
    pub columns: usize,
    pub cell_pixels: Option<(u16, u16)>,
    pub placements: Vec<ImagePlacement>,
    // Final pieces keep the dimensions chosen when their loads finish.
    pub fixed: bool,
    pub indexed: Vec<Option<(ImagePlacement, bool)>>,
}

impl Images {
    pub fn place(
        &mut self,
        index: usize,
        indent: usize,
        alone: bool,
    ) -> Option<(ImagePlacement, bool)> {
        if self.fixed {
            return self.indexed.get(index).cloned().flatten();
        }
        let ImageStatus::Ready(image) = self.statuses.get(index)? else {
            return None;
        };
        let limits = ImageLimits {
            columns: self.columns.saturating_sub(indent),
            rows: 20,
            cell_pixels: self.cell_pixels,
        };
        let inline = !alone && rows_at_width(image.natural, limits) <= 1.5;
        let size = if inline {
            size_in_row(image.natural, limits)?
        } else {
            size_in_cells(image.natural, limits)?
        };
        let placement = ImagePlacement {
            image: image.clone(),
            size,
        };
        self.placements.push(placement.clone());
        self.indexed.resize(self.statuses.len(), None);
        self.indexed[index] = Some((placement.clone(), inline));
        Some((placement, inline))
    }
}

pub(crate) fn requests(
    source: &str,
    links: &render::Links,
    columns: usize,
    cell_pixels: Option<(u16, u16)>,
    color_level: ColorLevel,
) -> Vec<ImageRequest> {
    let callback = |link: pulldown_cmark::BrokenLink<'_>| {
        links
            .get(&render::label(&link.reference))
            .map(|(url, title)| (url.clone().into(), title.clone().into()))
    };
    Parser::new_with_broken_link_callback(source, options(), Some(callback))
        .filter_map(|event| match event {
            Event::Start(Tag::Image { dest_url, .. }) => Some(ImageRequest {
                src: dest_url.into_string(),
                limits: ImageLimits {
                    columns,
                    rows: 20,
                    cell_pixels,
                },
                color_level,
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn image_end(events: &[(Event<'_>, Range<usize>)], start: usize) -> usize {
    let mut depth = 0;
    for (index, (event, _)) in events.iter().enumerate().skip(start) {
        match event {
            Event::Start(Tag::Image { .. }) => depth += 1,
            Event::End(TagEnd::Image) => {
                depth -= 1;
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    start
}

pub(crate) fn alone(events: &[(Event<'_>, Range<usize>)], start: usize, end: usize) -> bool {
    let boundary = |event: &Event<'_>| {
        matches!(
            event,
            Event::Start(
                Tag::Paragraph
                    | Tag::Item
                    | Tag::Heading { .. }
                    | Tag::CodeBlock(_)
                    | Tag::BlockQuote(_)
                    | Tag::List(_)
                    | Tag::Table(_)
                    | Tag::HtmlBlock
                    | Tag::FootnoteDefinition(_)
            ) | Event::End(
                TagEnd::Paragraph
                    | TagEnd::Item
                    | TagEnd::Heading(_)
                    | TagEnd::CodeBlock
                    | TagEnd::BlockQuote(_)
                    | TagEnd::List(_)
                    | TagEnd::Table
                    | TagEnd::HtmlBlock
                    | TagEnd::FootnoteDefinition
            ) | Event::Rule
        )
    };
    let content = |event: &Event<'_>| match event {
        Event::Text(text) => !text.trim().is_empty(),
        Event::Start(Tag::Image { .. })
        | Event::Code(_)
        | Event::FootnoteReference(_)
        | Event::InlineHtml(_) => true,
        _ => false,
    };
    !events[..start]
        .iter()
        .rev()
        .take_while(|(event, _)| !boundary(event))
        .any(|(event, _)| content(event))
        && !events[end + 1..]
            .iter()
            .take_while(|(event, _)| !boundary(event))
            .any(|(event, _)| content(event))
}

pub(crate) fn use_columns(cx: &mut Scope) -> (NodeRef, usize) {
    let node = cx.use_node_ref();
    let metrics = cx.use_box_metrics(node);
    let window = cx.use_window_size();
    (
        node,
        if metrics.has_measured {
            metrics.width as usize
        } else {
            usize::from(window.columns)
        },
    )
}

pub(crate) struct StreamImages {
    pub setting: MarkdownImages,
    pub columns: usize,
    pub time: Duration,
    pub theme: super::MarkdownTheme,
    results: (mpsc::Sender<ResultImage>, mpsc::Receiver<ResultImage>),
}

impl Default for StreamImages {
    fn default() -> Self {
        Self {
            setting: MarkdownImages::Alt,
            columns: 80,
            time: Duration::ZERO,
            theme: super::MarkdownTheme::from(&crate::ui::Palette::default()),
            results: mpsc::channel(),
        }
    }
}

struct ResultImage {
    piece: usize,
    index: usize,
    result: Result<Arc<LoadedImage>, LoadError>,
}

pub(crate) struct Waiting {
    requests: Vec<ImageRequest>,
    images: Images,
    jobs: Vec<Option<Arc<Job>>>,
    started: Duration,
    probing: bool,
    theme: super::MarkdownTheme,
}

impl Drop for Waiting {
    fn drop(&mut self) {
        for job in self.jobs.iter().flatten() {
            job.cancel();
        }
    }
}

impl StreamImages {
    pub fn enqueue(
        &self,
        piece: &super::MarkdownPiece,
        id: usize,
        wake: Option<super::hook::Wake>,
    ) -> Option<Waiting> {
        if matches!(self.setting, MarkdownImages::Alt) {
            return None;
        }
        let super::piece::Kind::Document { source, links, .. } = &piece.inner.kind else {
            return None;
        };
        let columns = self
            .columns
            .saturating_sub(super::piece::hanging_width(&piece.inner.hang, &self.theme));
        let requests = requests(source, links, columns, None, ColorLevel::TrueColor);
        if requests.is_empty() {
            return None;
        }
        let mut waiting = Waiting {
            images: Images {
                columns,
                statuses: vec![ImageStatus::Loading; requests.len()],
                ..Default::default()
            },
            jobs: vec![None; requests.len()],
            requests,
            started: self.time,
            probing: true,
            theme: self.theme.clone(),
        };
        self.start(&mut waiting, id, wake);
        Some(waiting)
    }

    pub fn start(&self, waiting: &mut Waiting, id: usize, wake: Option<super::hook::Wake>) {
        if !waiting.probing {
            return;
        }
        let MarkdownImages::Kitty {
            loaders,
            cell_pixels,
            color_level,
        } = &self.setting
        else {
            return;
        };
        waiting.probing = false;
        waiting.started = self.time;
        waiting.images.cell_pixels = *cell_pixels;
        for (index, request) in waiting.requests.iter_mut().enumerate() {
            request.limits.cell_pixels = *cell_pixels;
            request.color_level = *color_level;
            let ImageLoaders { local, network } = loaders;
            let loader = if is_network(&request.src) {
                network
            } else {
                local
            };
            let Some(loader) = loader
                .as_ref()
                .filter(|_| matches!(color_level, ColorLevel::TrueColor | ColorLevel::Ansi256))
            else {
                waiting.images.statuses[index] = ImageStatus::Failed;
                continue;
            };
            let reply = self.results.0.clone();
            let wake = wake.clone().map(super::hook::Wake::callback);
            let job = Job::new(&request.src, move |_, result| {
                let _ = reply.send(ResultImage {
                    piece: id,
                    index,
                    result,
                });
                if let Some(wake) = wake {
                    wake();
                }
            });
            waiting.jobs[index] = Some(job.clone());
            start_load(request.clone(), loader.clone(), job);
        }
    }

    pub(super) fn poll(
        &self,
        pending: &mut std::collections::VecDeque<super::stream::Pending>,
        count: usize,
    ) {
        self.expire(pending, false);
        while let Ok(result) = self.results.1.try_recv() {
            Self::accept(pending, count, result);
        }
    }

    fn accept(
        pending: &mut std::collections::VecDeque<super::stream::Pending>,
        count: usize,
        result: ResultImage,
    ) {
        let Some(waiting) = result
            .piece
            .checked_sub(count)
            .and_then(|index| pending.get_mut(index))
            .and_then(|pending| pending.images.as_mut())
        else {
            return;
        };
        if matches!(waiting.images.statuses[result.index], ImageStatus::Loading) {
            waiting.images.statuses[result.index] = match result.result {
                Ok(image) => ImageStatus::Ready(image),
                Err(_) => ImageStatus::Failed,
            };
            waiting.jobs[result.index] = None;
        }
    }

    fn expire(&self, pending: &mut std::collections::VecDeque<super::stream::Pending>, wall: bool) {
        for waiting in pending
            .iter_mut()
            .filter_map(|pending| pending.images.as_mut())
        {
            for (index, job) in waiting.jobs.iter_mut().enumerate() {
                if job.as_ref().is_some_and(|job| {
                    self.time.saturating_sub(waiting.started) >= job.timeout
                        || (wall && job.remaining().is_zero())
                }) {
                    job.take().expect("pending job").cancel();
                    waiting.images.statuses[index] = ImageStatus::Failed;
                }
            }
        }
    }

    pub(super) fn wait(
        &self,
        pending: &mut std::collections::VecDeque<super::stream::Pending>,
        count: usize,
    ) -> bool {
        let remaining = pending
            .iter()
            .filter_map(|pending| pending.images.as_ref())
            .flat_map(|waiting| {
                waiting.jobs.iter().flatten().map(|job| {
                    job.remaining().min(
                        job.timeout
                            .saturating_sub(self.time.saturating_sub(waiting.started)),
                    )
                })
            })
            .min();
        let Some(remaining) = remaining else {
            return false;
        };
        match self.results.1.recv_timeout(remaining) {
            Ok(result) => Self::accept(pending, count, result),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                for waiting in pending
                    .iter_mut()
                    .filter_map(|pending| pending.images.as_mut())
                {
                    for (index, job) in waiting.jobs.iter_mut().enumerate() {
                        if job.as_ref().is_some_and(|job| {
                            job.remaining().is_zero()
                                || job
                                    .timeout
                                    .saturating_sub(self.time.saturating_sub(waiting.started))
                                    <= remaining
                        }) {
                            job.take().expect("pending job").cancel();
                            waiting.images.statuses[index] = ImageStatus::Failed;
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        self.expire(pending, true);
        self.poll(pending, count);
        true
    }
}

impl Waiting {
    pub fn probing(&self) -> bool {
        self.probing
    }

    pub fn ready(&self) -> bool {
        !self.probing
            && !self
                .images
                .statuses
                .iter()
                .any(|status| matches!(status, ImageStatus::Loading))
    }

    pub fn finish(&self, piece: &super::MarkdownPiece) -> super::MarkdownPiece {
        let super::piece::Kind::Document {
            source,
            number,
            links,
        } = &piece.inner.kind
        else {
            return piece.clone();
        };
        let mut images = self.images.clone();
        let cache = std::cell::RefCell::<render::Cache>::default();
        piece.render_with(&self.theme, &cache, &mut images);
        images.fixed = true;
        super::MarkdownPiece {
            inner: std::rc::Rc::new(super::piece::Content {
                gap: piece.inner.gap,
                hang: piece.inner.hang.clone(),
                kind: super::piece::Kind::Document {
                    source: source.clone(),
                    number: *number,
                    links: links.clone(),
                },
                images,
            }),
        }
    }
}

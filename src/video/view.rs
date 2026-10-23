use std::cell::RefCell;
use std::rc::Rc;

use ink_rs::{Element, FlexDirection, OutputWriter, Props, Scope, Styled, Text, View};

use crate::image::{ImageId, ImagePair, ImageTheme, colored_placeholder_rows};
use crate::ui::use_theme;

use super::{VideoPlayer, VideoStatus};

/// A video drawn with kitty placeholders, or `[alt]` while loading or when unavailable.
///
/// One player drives one view. A second view of the same player is not supported.
#[derive(Clone, PartialEq, Props)]
#[props(component = video)]
pub struct Video {
    /// The player that owns the decoder, clock and playback state.
    pub player: VideoPlayer,
    /// The text between brackets while loading or when video is unavailable.
    #[props(into)]
    pub alt: String,
}

struct Transmission {
    pair: ImagePair,
    frames: [Option<(u64, u64)>; 2],
}

impl Transmission {
    fn slot(&self, stream: u64) -> usize {
        (0..2)
            .filter(|&slot| self.frames[slot].is_some_and(|(id, _)| id == stream))
            .max_by_key(|&slot| self.frames[slot].map(|(_, sequence)| sequence))
            .unwrap_or(self.pair.drawn)
    }
}

fn video(cx: &mut Scope, props: &Video) -> Element {
    let theme = use_theme::<ImageTheme>(cx);
    let player = &props.player;
    let snapshot = &player.snapshot;
    let identity = (snapshot.level, Rc::as_ptr(&player.model));
    let transmission = cx.use_memo(identity, || {
        Rc::new(RefCell::new(
            ImageId::allocate(snapshot.level)
                .and_then(|front| {
                    ImageId::allocate(snapshot.level).map(|back| Transmission {
                        pair: ImagePair::new(front, back),
                        frames: [None, None],
                    })
                })
                .ok(),
        ))
    });
    let placement = {
        let transmission = transmission.borrow();
        snapshot.size.and_then(|size| {
            let transmission = transmission.as_ref()?;
            if matches!(
                snapshot.status,
                VideoStatus::Loading | VideoStatus::Failed(_)
            ) {
                return None;
            }
            let slot = transmission.slot(snapshot.stream);
            Some((size, slot, transmission.pair.id(slot).color))
        })
    };
    let out = cx.use_stdout();
    let sent = transmission.clone();
    let model = player.model.clone();
    cx.use_effect((player.clone(), placement), move || {
        let Some((size, slot, _)) = placement else {
            return;
        };
        let mut sent = sent.borrow_mut();
        let Some(sent) = sent.as_mut() else { return };
        sent.pair.drawn = slot;
        if let Some(frame) = &model.borrow().frame {
            let key = (frame.stream, frame.sequence);
            if !sent.frames.contains(&Some(key)) {
                let target = if sent.pair.occupied(slot) {
                    1 - slot
                } else {
                    slot
                };
                sent.pair.upload(out, target, &frame.png, size);
                sent.frames[target] = Some(key);
            }
        }
        sent.pair.resize(out, size);
    });
    cx.use_effect(identity, move || DeleteIdle(transmission, out));
    if let Some((size, _, color)) = placement {
        View::new()
            .flex_direction(FlexDirection::Column)
            .flex_shrink(0.0)
            .children(colored_placeholder_rows(size, color, None))
            .into()
    } else {
        Text::new(format!("[{}]", props.alt))
            .color(theme.color)
            .dim_color(theme.dim)
            .italic(theme.italic)
            .into()
    }
}

struct DeleteIdle(Rc<RefCell<Option<Transmission>>>, OutputWriter);

impl Drop for DeleteIdle {
    fn drop(&mut self) {
        if let Some(sent) = self.0.borrow().as_ref() {
            sent.pair.delete_idle(self.1);
        }
    }
}

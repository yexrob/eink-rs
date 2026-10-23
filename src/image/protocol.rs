use std::cell::RefCell;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use ink_rs::{AnimationOptions, Color, ColorLevel, OutputWriter, Scope, Text};

use super::decode::PreparedImage;
use super::playback::{Animation, Position};
use super::{CellSize, LoadError};

mod diacritics;
pub(super) use diacritics::DIACRITICS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImageId {
    value: u32,
    color: Color,
}

impl ImageId {
    fn allocate(level: ColorLevel) -> Result<Self, LoadError> {
        static START: OnceLock<u32> = OnceLock::new();
        static TRUE_COLOR: AtomicU32 = AtomicU32::new(0);
        static ANSI: AtomicU32 = AtomicU32::new(0);
        // Reserve the low byte for indexed colors. Never recycle an id while
        // placeholders for it can remain in scrollback.
        const COUNT: u32 = 0x100_0000 - 256;
        let start = *START
            .get_or_init(|| (RandomState::new().hash_one("eink image") % u64::from(COUNT)) as u32);
        let value = match level {
            ColorLevel::TrueColor => {
                let offset = take(&TRUE_COLOR, COUNT)
                    .ok_or_else(|| LoadError::new("image ids exhausted"))?;
                256 + (start + offset) % COUNT
            }
            ColorLevel::Ansi256 => {
                let offset = take(&ANSI, 254)
                    .ok_or_else(|| LoadError::new("indexed image ids exhausted"))?;
                let value = 1 + (start + offset) % 254;
                // ink-rs reserves 31 for its graphics query.
                if value >= 31 { value + 1 } else { value }
            }
            _ => return Err(LoadError::new("image placeholders need 256 colors")),
        };
        let color = if level == ColorLevel::Ansi256 {
            Color::Ansi256(value as u8)
        } else {
            Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
        };
        Ok(Self { value, color })
    }
}

/// Takes the counter's next value below `limit`, or `None` once all are taken.
fn take(counter: &AtomicU32, limit: u32) -> Option<u32> {
    let mut next = counter.load(Ordering::Relaxed);
    while next < limit {
        match counter.compare_exchange_weak(next, next + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(taken) => return Some(taken),
            Err(current) => next = current,
        }
    }
    None
}

/// A loaded image shared by renderers and retained across placement changes.
pub(crate) struct LoadedImage {
    pub natural: (u32, u32),
    id: ImageId,
    content: Content,
}

enum Content {
    Still(Mutex<Transmission>),
    Animated(Animation, Mutex<Playback>),
}

struct Transmission {
    png: Option<Vec<u8>>,
    placement: Option<CellSize>,
}

/// Two ids that take turns: the render draws one, and the next frame goes into the other.
struct Playback {
    back: ImageId,
    /// Time since the first send, advanced by the clock of the component that loaded it.
    time: Duration,
    started: bool,
    drawn: usize,
    /// When the frame in each slot shows.
    shows: [Option<Duration>; 2],
    sizes: [Option<CellSize>; 2],
}

impl LoadedImage {
    pub(super) fn new(image: PreparedImage, level: ColorLevel) -> Result<Self, LoadError> {
        let id = ImageId::allocate(level)?;
        let content = match image.animation {
            Some(animation) => match ImageId::allocate(level) {
                Ok(back) => Content::Animated(animation, Mutex::new(Playback::new(back))),
                // Without a second id the first frame shows as a still image.
                Err(_) => Content::still(animation.frames.into_iter().next().expect("frame").png),
            },
            None => Content::still(image.png),
        };
        Ok(Self {
            natural: image.natural,
            id,
            content,
        })
    }

    /// Places the image; an animation draws the id that holds the frame due.
    pub fn place(self: &Arc<Self>, size: CellSize) -> ImagePlacement {
        let (id, slot, due) = match &self.content {
            Content::Still(_) => (self.id, 0, None),
            Content::Animated(animation, playback) => {
                let playback = playback.lock().expect("image playback");
                let slot = playback.due_slot();
                let due = animation.position(playback.time);
                (playback.id(self.id, slot), slot, Some(due))
            }
        };
        ImagePlacement {
            image: self.clone(),
            size,
            id,
            slot,
            due,
        }
    }

    /// Sends what a committed placement needs; returns true when playback starts.
    fn send(&self, out: OutputWriter, placement: &ImagePlacement) -> bool {
        let size = placement.size;
        match &self.content {
            Content::Still(transmission) => {
                let mut transmission = transmission.lock().expect("image transmission");
                if let Some(png) = transmission.png.take() {
                    transmit(&png, self.id.value, |command| {
                        out.write_passthrough(command)
                    });
                }
                if transmission.placement != Some(size) {
                    // A named placement replaces the old dimensions on resize.
                    send_placement(out, self.id, size);
                    transmission.placement = Some(size);
                }
                false
            }
            Content::Animated(animation, playback) => {
                let mut playback = playback.lock().expect("image playback");
                let starting = !playback.started;
                playback.started = true;
                playback.drawn = placement.slot;
                if playback.shows[placement.slot].is_none() {
                    let png = &animation.frames[0].png;
                    playback.upload(out, self.id, placement.slot, Duration::ZERO, png, size);
                }
                // Retransmitting an id deletes its placements, so frames go into the idle id.
                let next = animation.following(placement.due.expect("animation position"));
                let idle = 1 - placement.slot;
                if Some(next.at) > playback.shows[placement.slot]
                    && playback.shows[idle] != Some(next.at)
                {
                    let png = &animation.frames[next.frame].png;
                    playback.upload(out, self.id, idle, next.at, png, size);
                }
                for slot in 0..2 {
                    if playback.shows[slot].is_some() && playback.sizes[slot] != Some(size) {
                        send_placement(out, playback.id(self.id, slot), size);
                        playback.sizes[slot] = Some(size);
                    }
                }
                starting
            }
        }
    }

    /// Deletes the data of an animation's idle id. The drawn id stays for the
    /// placeholders the terminal keeps.
    fn delete_idle(&self, out: OutputWriter) {
        if let Content::Animated(_, playback) = &self.content {
            let playback = playback.lock().expect("image playback");
            let idle = 1 - playback.drawn;
            if playback.shows[idle].is_some() {
                out.write_passthrough(format!(
                    "\x1b_Ga=d,d=I,i={},q=2\x1b\\",
                    playback.id(self.id, idle).value
                ));
            }
        }
    }
}

impl Content {
    fn still(png: Vec<u8>) -> Self {
        Self::Still(Mutex::new(Transmission {
            png: Some(png),
            placement: None,
        }))
    }
}

impl Playback {
    fn new(back: ImageId) -> Self {
        Self {
            back,
            time: Duration::ZERO,
            started: false,
            drawn: 0,
            shows: [None, None],
            sizes: [None, None],
        }
    }

    fn id(&self, front: ImageId, slot: usize) -> ImageId {
        if slot == 0 { front } else { self.back }
    }

    /// The idle slot once its frame is due and newer than the drawn one, else the drawn slot.
    fn due_slot(&self) -> usize {
        let idle = 1 - self.drawn;
        if self.shows[idle].is_some_and(|at| at <= self.time && Some(at) > self.shows[self.drawn]) {
            idle
        } else {
            self.drawn
        }
    }

    // The caller supplies the encoded frame, so frames from a decoder fit the same path.
    fn upload(
        &mut self,
        out: OutputWriter,
        front: ImageId,
        slot: usize,
        shows: Duration,
        png: &[u8],
        size: CellSize,
    ) {
        let id = self.id(front, slot);
        transmit(png, id.value, |command| out.write_passthrough(command));
        send_placement(out, id, size);
        self.shows[slot] = Some(shows);
        self.sizes[slot] = Some(size);
    }
}

fn send_placement(out: OutputWriter, id: ImageId, size: CellSize) {
    out.write_passthrough(format!(
        "\x1b_Ga=p,U=1,i={},p=1,c={},r={},q=2\x1b\\",
        id.value, size.columns, size.rows
    ));
}

/// Advances started animations on one clock, no slower than their shortest delay.
pub(super) fn use_playback(cx: &mut Scope, statuses: &[super::ImageStatus]) {
    let animations: Vec<_> = statuses
        .iter()
        .filter_map(|status| match status {
            super::ImageStatus::Ready(image) => match &image.content {
                Content::Animated(animation, playback) => Some((animation, playback)),
                Content::Still(_) => None,
            },
            _ => None,
        })
        .collect();
    let interval = animations
        .iter()
        .filter_map(|(animation, playback)| {
            let playback = playback.lock().expect("image playback");
            let position = animation.position(playback.time);
            // A finished animation that shows its last frame needs no clock.
            let finished = position.ended && playback.shows[playback.drawn] == Some(position.at);
            if !playback.started || finished {
                return None;
            }
            animation.frames.iter().map(|frame| frame.delay).min()
        })
        .min();
    let options = AnimationOptions {
        active: interval.is_some(),
        interval: interval
            .unwrap_or(Duration::from_millis(100))
            .max(Duration::from_millis(20)),
    };
    let clock = cx.use_animation(options);
    // Changing the options restarts the clock at zero.
    let previous = cx.use_memo((), || Rc::new(RefCell::new(None)));
    let delta = match previous.replace(Some((options, clock.time))) {
        Some((old, time)) if old == options => clock.time.saturating_sub(time),
        _ => Duration::ZERO,
    };
    for (_, playback) in animations {
        let mut playback = playback.lock().expect("image playback");
        if playback.started {
            playback.time += delta;
        }
    }
}

/// One image used by a render, with its current dimensions.
#[derive(Clone)]
pub(crate) struct ImagePlacement {
    pub image: Arc<LoadedImage>,
    pub size: CellSize,
    id: ImageId,
    slot: usize,
    due: Option<Position>,
}

/// Returns one styled text run per row, including one-row inline placements.
pub(crate) fn placeholder_rows(placement: &ImagePlacement) -> Vec<Text> {
    linked_placeholder_rows(placement, None)
}

pub(crate) fn linked_placeholder_rows(placement: &ImagePlacement, link: Option<&str>) -> Vec<Text> {
    DIACRITICS
        .iter()
        .take(placement.size.rows)
        .map(|&row| {
            let text: String = DIACRITICS
                .iter()
                .take(placement.size.columns)
                .flat_map(|&column| ['\u{10eeee}', row, column])
                .collect();
            let text = match link {
                Some(url) => format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\"),
                None => text,
            };
            Text::new(text).color(placement.id.color)
        })
        .collect()
}

/// Sends a render's images after commit; shared images retain transmission state.
/// Unmounting deletes the idle ids of animations.
pub(crate) fn use_image_effect(cx: &mut Scope, images: Vec<ImagePlacement>) {
    let out = cx.use_stdout();
    let started = cx.use_state(|| 0_u64);
    let sent = cx.use_memo((), || Rc::new(RefCell::new(Vec::new())));
    let dependencies: Vec<_> = images
        .iter()
        .map(|placement| (placement.id.value, placement.size, placement.due))
        .collect();
    let latest = sent.clone();
    cx.use_effect(dependencies, move || {
        for placement in &images {
            if placement.image.send(out, placement) {
                // A render after the first send starts the playback clock.
                started.update(|count| *count += 1);
            }
        }
        *latest.borrow_mut() = images;
    });
    cx.use_effect((), move || DeleteIdle(sent, out));
}

struct DeleteIdle(Rc<RefCell<Vec<ImagePlacement>>>, OutputWriter);

impl Drop for DeleteIdle {
    fn drop(&mut self) {
        for placement in self.0.borrow().iter() {
            placement.image.delete_idle(self.1);
        }
    }
}

fn transmit(png: &[u8], id: u32, mut write: impl FnMut(String)) {
    let encoded = base64(png);
    let count = encoded.len().div_ceil(4096);
    for (index, chunk) in encoded.as_bytes().chunks(4096).enumerate() {
        let more = u8::from(index + 1 < count);
        let chunk = std::str::from_utf8(chunk).expect("base64 is ASCII");
        let command = if index == 0 {
            format!("\x1b_Ga=t,t=d,f=100,i={id},q=2,m={more};{chunk}\x1b\\")
        } else {
            format!("\x1b_Gq=2,m={more};{chunk}\x1b\\")
        };
        write(command);
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0u32, |value, (index, byte)| {
            value | u32::from(*byte) << (16 - index * 8)
        });
        for index in 0..4 {
            encoded.push(if index <= chunk.len() {
                ALPHABET[((value >> (18 - index * 6)) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_vectors_and_chunk_boundaries() {
        for (bytes, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(bytes.as_bytes()), expected);
        }
        assert_eq!(base64(&[0, 255, 254, 253]), "AP/+/Q==");
        for length in [1, 3072, 3073, 6144, 6145] {
            let bytes = vec![42; length];
            let mut commands = Vec::new();
            transmit(&bytes, 123, |command| commands.push(command));
            let expected = base64(&bytes);
            assert_eq!(commands.len(), expected.len().div_ceil(4096));
            let mut payload = String::new();
            for (index, command) in commands.iter().enumerate() {
                let (keys, data) = command.split_once(';').unwrap();
                let more = usize::from(index + 1 < commands.len());
                if index == 0 {
                    assert_eq!(keys, format!("\x1b_Ga=t,t=d,f=100,i=123,q=2,m={more}"));
                } else {
                    assert_eq!(keys, format!("\x1b_Gq=2,m={more}"));
                }
                let data = data.strip_suffix("\x1b\\").unwrap();
                assert!(data.len() <= 4096);
                assert_eq!(data.len() % 4, 0);
                payload.push_str(data);
            }
            assert_eq!(payload, expected);
        }
    }
}

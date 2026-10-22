use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use ink_rs::{Color, ColorLevel, OutputWriter, Scope, Text};

use super::decode::PreparedImage;
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
                let offset = TRUE_COLOR
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                        (next < COUNT).then_some(next + 1)
                    })
                    .map_err(|_| LoadError::new("image ids exhausted"))?;
                256 + (start + offset) % COUNT
            }
            ColorLevel::Ansi256 => {
                let offset = ANSI
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                        (next < 254).then_some(next + 1)
                    })
                    .map_err(|_| LoadError::new("indexed image ids exhausted"))?;
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

/// A loaded image shared by renderers and retained across placement changes.
pub(crate) struct LoadedImage {
    pub natural: (u32, u32),
    id: ImageId,
    transmission: Mutex<Transmission>,
}

struct Transmission {
    png: Option<Vec<u8>>,
    placement: Option<CellSize>,
}

impl LoadedImage {
    pub(super) fn new(image: PreparedImage, level: ColorLevel) -> Result<Self, LoadError> {
        Ok(Self {
            natural: image.natural,
            id: ImageId::allocate(level)?,
            transmission: Mutex::new(Transmission {
                png: Some(image.png),
                placement: None,
            }),
        })
    }

    fn send(&self, out: OutputWriter, size: CellSize) {
        let mut transmission = self.transmission.lock().expect("image transmission");
        if let Some(png) = transmission.png.take() {
            transmit(&png, self.id.value, |command| {
                out.write_passthrough(command)
            });
        }
        if transmission.placement != Some(size) {
            // A named placement replaces the old dimensions on resize.
            out.write_passthrough(format!(
                "\x1b_Ga=p,U=1,i={},p=1,c={},r={},q=2\x1b\\",
                self.id.value, size.columns, size.rows
            ));
            transmission.placement = Some(size);
        }
    }
}

/// One image used by a render, with its current dimensions.
#[derive(Clone)]
pub(crate) struct ImagePlacement {
    pub image: Arc<LoadedImage>,
    pub size: CellSize,
}

/// Returns one styled text run per row, including one-row inline placements.
pub(crate) fn placeholder_rows(placement: &ImagePlacement) -> Vec<Text> {
    DIACRITICS
        .iter()
        .take(placement.size.rows)
        .map(|&row| {
            let text: String = DIACRITICS
                .iter()
                .take(placement.size.columns)
                .flat_map(|&column| ['\u{10eeee}', row, column])
                .collect();
            Text::new(text).color(placement.image.id.color)
        })
        .collect()
}

/// Sends a render's images after commit; shared images retain transmission state.
pub(crate) fn use_image_effect(cx: &mut Scope, images: Vec<ImagePlacement>) {
    let out = cx.use_stdout();
    let dependencies: Vec<_> = images
        .iter()
        .map(|placement| (placement.image.id.value, placement.size))
        .collect();
    cx.use_effect(dependencies, move || {
        for placement in images {
            placement.image.send(out, placement.size);
        }
    });
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

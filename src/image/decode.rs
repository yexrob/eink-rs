use std::io::Cursor;
use std::time::Duration;

use image::codecs::{
    gif::GifDecoder,
    png::{CompressionType, FilterType, PngDecoder, PngEncoder},
    webp::WebPDecoder,
};
use image::metadata::LoopCount;
use image::{
    AnimationDecoder, DynamicImage, Frames, ImageDecoder, ImageEncoder, ImageFormat, ImageReader,
    Limits,
};

use super::LoadError;
use super::loaders::check_length;
use super::playback::{Animation, EncodedFrame};
use super::protocol::DIACRITICS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImageLimits {
    pub columns: usize,
    pub rows: usize,
    pub cell_pixels: Option<(u16, u16)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellSize {
    pub columns: usize,
    pub rows: usize,
}

impl ImageLimits {
    fn pixels(self) -> (u32, u32) {
        let (width, height) = self.cell_pixels.unwrap_or((1, 2));
        (u32::from(width.max(1)), u32::from(height.max(1)))
    }
}

/// Returns the rows the image covers when only the column limit applies.
#[cfg(feature = "markdown-image")]
pub(crate) fn rows_at_width(natural: (u32, u32), limits: ImageLimits) -> f64 {
    let columns = limits.columns.min(DIACRITICS.len());
    let (cell_width, cell_height) = limits.pixels();
    let scale = 1.0_f64.min(columns as f64 * f64::from(cell_width) / f64::from(natural.0));
    f64::from(natural.1) * scale / f64::from(cell_height)
}

/// Returns the scale that fits the image to one row, enlarging it when it is shorter.
#[cfg(any(feature = "markdown-image", feature = "image-svg"))]
fn row_scale(natural: (u32, u32), limits: ImageLimits) -> Option<f64> {
    let columns = limits.columns.min(DIACRITICS.len());
    let (width, height) = natural;
    if columns == 0 || width == 0 || height == 0 {
        return None;
    }
    let (cell_width, cell_height) = limits.pixels();
    Some(
        (columns as f64 * f64::from(cell_width) / f64::from(width))
            .min(f64::from(cell_height) / f64::from(height)),
    )
}

/// Fits an inline image to one row; the terminal enlarges it to the row's height.
#[cfg(feature = "markdown-image")]
pub(crate) fn size_in_row(natural: (u32, u32), limits: ImageLimits) -> Option<CellSize> {
    let scale = row_scale(natural, limits)?;
    let (cell_width, _) = limits.pixels();
    Some(CellSize {
        columns: ((f64::from(natural.0) * scale / f64::from(cell_width)).ceil() as usize)
            .clamp(1, limits.columns.min(DIACRITICS.len())),
        rows: 1,
    })
}

/// Fits the natural dimensions inside cell limits without enlarging the image.
pub(crate) fn size_in_cells(natural: (u32, u32), limits: ImageLimits) -> Option<CellSize> {
    let columns = limits.columns.min(DIACRITICS.len());
    let rows = limits.rows.min(20);
    let (width, height) = natural;
    if columns == 0 || rows == 0 || width == 0 || height == 0 {
        return None;
    }
    let (cell_width, cell_height) = limits.pixels();
    let scale = 1.0_f64
        .min(columns as f64 * f64::from(cell_width) / f64::from(width))
        .min(rows as f64 * f64::from(cell_height) / f64::from(height));
    Some(CellSize {
        columns: ((f64::from(width) * scale / f64::from(cell_width)).ceil() as usize)
            .clamp(1, columns),
        rows: ((f64::from(height) * scale / f64::from(cell_height)).ceil() as usize).clamp(1, rows),
    })
}

fn pixel_size(natural: (u32, u32), limits: ImageLimits) -> Result<(u32, u32), LoadError> {
    let cells = size_in_cells(natural, limits)
        .ok_or_else(|| LoadError::new("image has no display area"))?;
    let (cell_width, cell_height) = limits.pixels();
    let scale = 1.0_f64
        .min(cells.columns as f64 * f64::from(cell_width) / f64::from(natural.0))
        .min(cells.rows as f64 * f64::from(cell_height) / f64::from(natural.1));
    Ok((
        (f64::from(natural.0) * scale).floor().max(1.0) as u32,
        (f64::from(natural.1) * scale).floor().max(1.0) as u32,
    ))
}

pub(super) struct PreparedImage {
    pub natural: (u32, u32),
    pub png: Vec<u8>,
    pub animation: Option<Animation>,
}

fn decoder_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(32_768);
    limits.max_image_height = Some(32_768);
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits
}

fn decode_raster(bytes: &[u8]) -> Result<DynamicImage, LoadError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(LoadError::from)?;
    reader.limits(decoder_limits());
    reader
        .decode()
        .map_err(|error| LoadError::new(error.to_string()))
}

fn png(image: DynamicImage) -> Result<Vec<u8>, LoadError> {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|error| LoadError::new(error.to_string()))?;
    Ok(bytes.into_inner())
}

pub(super) fn prepare(
    bytes: &[u8],
    limits: ImageLimits,
    animate: bool,
    expired: impl Fn() -> bool,
) -> Result<PreparedImage, LoadError> {
    check_length(bytes)?;
    #[cfg(feature = "image-svg")]
    if image::guess_format(bytes).is_err() {
        return prepare_svg(bytes, limits);
    }
    if animate && let Some(prepared) = prepare_animation(bytes, limits, &expired)? {
        return Ok(prepared);
    }
    let image = decode_raster(bytes)?;
    let natural = (image.width(), image.height());
    let (width, height) = pixel_size(natural, limits)?;
    let image = if (width, height) == natural {
        image
    } else {
        image.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    };
    Ok(PreparedImage {
        natural,
        png: png(image)?,
        animation: None,
    })
}

/// An animation's natural size, loop count and frames.
type Animated<'a> = ((u32, u32), LoopCount, Frames<'a>);

/// Streams the frames of an animated GIF, APNG or WebP; other images give `None`.
fn animation_frames(bytes: &[u8]) -> image::ImageResult<Option<Animated<'_>>> {
    let reader = Cursor::new(bytes);
    Ok(match image::guess_format(bytes)? {
        ImageFormat::Gif => {
            let mut decoder = GifDecoder::new(reader)?;
            decoder.set_limits(decoder_limits())?;
            Some((
                decoder.dimensions(),
                decoder.loop_count(),
                decoder.into_frames(),
            ))
        }
        ImageFormat::Png => {
            let decoder = PngDecoder::with_limits(reader, decoder_limits())?;
            if !decoder.is_apng()? {
                return Ok(None);
            }
            let natural = decoder.dimensions();
            let decoder = decoder.apng()?;
            Some((natural, decoder.loop_count(), decoder.into_frames()))
        }
        ImageFormat::WebP => {
            let mut decoder = WebPDecoder::new(reader)?;
            decoder.set_limits(decoder_limits())?;
            if !decoder.has_animation() {
                return Ok(None);
            }
            Some((
                decoder.dimensions(),
                decoder.loop_count(),
                decoder.into_frames(),
            ))
        }
        _ => None,
    })
}

/// Encodes every frame at the display size. A single frame, or an animation
/// past 1,000 frames or 32 MiB of PNG, gives `None` and shows as a still image.
fn prepare_animation(
    bytes: &[u8],
    limits: ImageLimits,
    expired: &impl Fn() -> bool,
) -> Result<Option<PreparedImage>, LoadError> {
    let decode_error = |error: image::ImageError| LoadError::new(error.to_string());
    let Some((natural, loops, frames)) = animation_frames(bytes).map_err(decode_error)? else {
        return Ok(None);
    };
    let (width, height) = pixel_size(natural, limits)?;
    let mut encoded = Vec::new();
    let mut total = 0;
    for frame in frames {
        if expired() {
            return Err(LoadError::new("image load timed out"));
        }
        // Read past the cap to distinguish 1000 frames from a longer animation.
        let frame = frame.map_err(decode_error)?;
        if encoded.len() == 1000 {
            return Ok(None);
        }
        // Browsers treat delays of 10 ms or less as 100 ms.
        let delay = Duration::from(frame.delay());
        let delay = if delay <= Duration::from_millis(10) {
            Duration::from_millis(100)
        } else {
            delay
        };
        let pixels = DynamicImage::ImageRgba8(frame.into_buffer());
        let pixels = if (width, height) == natural {
            pixels
        } else {
            pixels.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        };
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
            .write_image(pixels.as_bytes(), width, height, pixels.color().into())
            .map_err(decode_error)?;
        total += png.len();
        if total > 32 * 1024 * 1024 {
            return Ok(None);
        }
        encoded.push(EncodedFrame { png, delay });
    }
    if encoded.len() < 2 {
        return Ok(None);
    }
    Ok(Some(PreparedImage {
        natural,
        png: Vec::new(),
        animation: Some(Animation {
            frames: encoded,
            loops,
        }),
    }))
}

#[cfg(feature = "image-svg")]
fn prepare_svg(bytes: &[u8], limits: ImageLimits) -> Result<PreparedImage, LoadError> {
    use resvg::{tiny_skia, usvg};
    use std::sync::{Arc, OnceLock};

    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let options = usvg::Options {
        fontdb: FONTS
            .get_or_init(|| {
                let mut fonts = usvg::fontdb::Database::new();
                fonts.load_system_fonts();
                Arc::new(fonts)
            })
            .clone(),
        // SVG references must not bypass the selected source loader or decoder limits.
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            resolve_data: Box::new(|_, bytes, _| {
                check_length(&bytes).ok()?;
                let image = decode_raster(&bytes).ok()?;
                Some(usvg::ImageKind::PNG(std::sync::Arc::new(png(image).ok()?)))
            }),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|error| LoadError::new(error.to_string()))?;
    let natural = (
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    );
    let (mut width, mut height) = pixel_size(natural, limits)?;
    // An inline placement enlarges the image to one row, so the vector is drawn at that size.
    if let Some(scale) = row_scale(natural, limits)
        && f64::from(natural.1) * scale > f64::from(height)
    {
        width = (f64::from(natural.0) * scale).floor().max(1.0) as u32;
        height = (f64::from(natural.1) * scale).floor().max(1.0) as u32;
    }
    let mut allocation = Limits::default();
    allocation.max_alloc = Some(128 * 1024 * 1024);
    allocation
        .reserve_buffer(width, height, image::ColorType::Rgba8)
        .map_err(|error| LoadError::new(error.to_string()))?;
    let mut pixels =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| LoadError::new("invalid SVG size"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(
            width as f32 / tree.size().width(),
            height as f32 / tree.size().height(),
        ),
        &mut pixels.as_mut(),
    );
    Ok(PreparedImage {
        natural,
        animation: None,
        png: pixels
            .encode_png()
            .map_err(|error| LoadError::new(error.to_string()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gif_prepares_scaled_png_frames_and_normalizes_zero_delay() {
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            encoder
                .set_repeat(image::codecs::gif::Repeat::Finite(2))
                .unwrap();
            for (red, ms) in [(30, 40), (90, 0), (150, 60)] {
                encoder
                    .encode_frame(image::Frame::from_parts(
                        image::RgbaImage::from_pixel(40, 20, image::Rgba([red, 0, 0, 255])),
                        0,
                        0,
                        image::Delay::from_numer_denom_ms(ms, 1),
                    ))
                    .unwrap();
            }
        }
        let limits = ImageLimits {
            columns: 2,
            rows: 20,
            cell_pixels: Some((10, 20)),
        };
        let prepared = prepare(&bytes, limits, true, || false).unwrap();
        assert_eq!(prepared.natural, (40, 20));
        let animation = prepared.animation.unwrap();
        assert!(
            matches!(animation.loops, image::metadata::LoopCount::Finite(count) if count.get() == 2)
        );
        assert_eq!(animation.frames.len(), 3);
        for (frame, (red, ms)) in animation
            .frames
            .iter()
            .zip([(30, 40), (90, 100), (150, 60)])
        {
            assert_eq!(image::guess_format(&frame.png).unwrap(), ImageFormat::Png);
            let pixels = image::load_from_memory(&frame.png).unwrap().to_rgba8();
            assert_eq!(pixels.dimensions(), (20, 10));
            assert_eq!(pixels.get_pixel(0, 0).0, [red, 0, 0, 255]);
            assert_eq!(frame.delay, Duration::from_millis(ms));
        }
        assert!(
            prepare(&bytes, limits, false, || false)
                .unwrap()
                .animation
                .is_none()
        );
        assert!(prepare(&bytes, limits, true, || true).is_err());
    }

    #[test]
    fn sizes_respect_caps_natural_dimensions_and_cell_ratio() {
        let limits = ImageLimits {
            columns: 80,
            rows: 20,
            cell_pixels: Some((10, 20)),
        };
        for (natural, limits, expected) in [
            ((1600, 800), limits, (80, 20)),
            ((400, 1600), limits, (10, 20)),
            (
                (400, 1600),
                ImageLimits {
                    rows: 100,
                    ..limits
                },
                (10, 20),
            ),
            ((20, 40), limits, (2, 2)),
            ((1, 1), limits, (1, 1)),
            (
                (10000, 10),
                ImageLimits {
                    columns: 1000,
                    ..limits
                },
                (297, 1),
            ),
            (
                (100, 100),
                ImageLimits {
                    cell_pixels: None,
                    ..limits
                },
                (40, 20),
            ),
            (
                (200, 100),
                ImageLimits {
                    columns: 20,
                    cell_pixels: None,
                    ..limits
                },
                (20, 5),
            ),
            ((100, 100), ImageLimits { rows: 1, ..limits }, (2, 1)),
        ] {
            assert_eq!(
                size_in_cells(natural, limits),
                Some(CellSize {
                    columns: expected.0,
                    rows: expected.1
                }),
            );
        }
        assert_eq!(
            size_in_cells(
                (10, 10),
                ImageLimits {
                    columns: 0,
                    ..limits
                }
            ),
            None
        );
        assert_eq!(
            size_in_cells((10, 10), ImageLimits { rows: 0, ..limits }),
            None
        );
    }

    #[cfg(feature = "image-svg")]
    #[test]
    fn svg_rasterizes_at_display_size() {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="red"/></svg>"#;
        let prepared = prepare(
            bytes,
            ImageLimits {
                columns: 10,
                rows: 20,
                cell_pixels: Some((10, 20)),
            },
            false,
            || false,
        )
        .unwrap();
        assert_eq!(prepared.natural, (200, 100));
        let image = image::load_from_memory(&prepared.png).unwrap();
        assert_eq!((image.width(), image.height()), (100, 50));
        assert_eq!(image.to_rgba8().get_pixel(50, 25).0, [255, 0, 0, 255]);

        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"/>"#;
        let prepared = prepare(
            bytes,
            ImageLimits {
                columns: 10,
                rows: 20,
                cell_pixels: Some((10, 20)),
            },
            false,
            || false,
        )
        .unwrap();
        let image = image::load_from_memory(&prepared.png).unwrap();
        assert_eq!((image.width(), image.height()), (40, 20));
    }
}

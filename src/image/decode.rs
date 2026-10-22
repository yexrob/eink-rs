use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageReader, Limits};

use super::LoadError;
use super::loaders::check_length;
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
}

fn decode_raster(bytes: &[u8]) -> Result<DynamicImage, LoadError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(LoadError::from)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(32_768);
    limits.max_image_height = Some(32_768);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
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

pub(super) fn prepare(bytes: &[u8], limits: ImageLimits) -> Result<PreparedImage, LoadError> {
    check_length(bytes)?;
    #[cfg(feature = "image-svg")]
    if image::guess_format(bytes).is_err() {
        return prepare_svg(bytes, limits);
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
    })
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
        png: pixels
            .encode_png()
            .map_err(|error| LoadError::new(error.to_string()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        )
        .unwrap();
        let image = image::load_from_memory(&prepared.png).unwrap();
        assert_eq!((image.width(), image.height()), (40, 20));
    }
}

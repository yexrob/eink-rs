//! Images rendered with kitty Unicode placeholders, with alt text as a fallback.

use ink_rs::{
    Color, ColorLevel, Element, FlexDirection, GraphicsSupport, Props, Scope, Styled, Text, View,
};

use crate::ui::{Palette, use_theme};

mod decode;
mod hook;
mod load;
mod loaders;
mod playback;
mod protocol;

pub(crate) use decode::{CellSize, ImageLimits, size_in_cells};
#[cfg(feature = "markdown-image")]
pub(crate) use decode::{rows_at_width, size_in_row};
pub(crate) use hook::use_images;
pub(crate) use load::{ImageRequest, ImageStatus};
#[cfg(feature = "markdown-image")]
pub(crate) use load::{Job, start_load};
#[cfg(feature = "image-http")]
pub use loaders::HttpLoader;
#[cfg(any(feature = "markdown-image", feature = "video"))]
pub(crate) use loaders::is_network;
pub use loaders::{FileLoader, ImageLoader, ImageLoaders, LoadError};
#[cfg(feature = "markdown-image")]
pub(crate) use protocol::{ImagePlacement, linked_placeholder_rows};
pub(crate) use protocol::{LoadedImage, placeholder_rows, use_image_effect};

/// An image displayed with kitty placeholders, or `[alt]` when unavailable.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = image)]
pub struct Image {
    /// A file path, URL, or source name understood by the selected loader.
    #[props(into)]
    pub src: String,
    /// The text between brackets while loading or when the image is unavailable.
    #[props(into)]
    pub alt: String,
    /// The column limit, bounded by the window width and 297. Defaults to the window width.
    pub max_columns: Option<usize>,
    /// The row limit, bounded by 20. Defaults to 20.
    pub max_rows: Option<usize>,
}

/// The style of an [`Image`]'s alt text.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageTheme {
    /// Foreground color, inherited by default.
    pub color: Option<Color>,
    /// Whether alt text is dimmed, true by default.
    pub dim: bool,
    /// Whether alt text is italic, true by default.
    pub italic: bool,
}

impl From<&Palette> for ImageTheme {
    fn from(_: &Palette) -> Self {
        Self {
            color: None,
            dim: true,
            italic: true,
        }
    }
}

fn image(cx: &mut Scope, props: &Image) -> Element {
    let theme = use_theme::<ImageTheme>(cx);
    let graphics = cx.use_graphics();
    let level = cx.use_color_level();
    let window = cx.use_window_size();
    let limits = ImageLimits {
        columns: props
            .max_columns
            .unwrap_or(usize::from(window.columns))
            .min(usize::from(window.columns)),
        rows: props.max_rows.unwrap_or(20),
        cell_pixels: graphics.cell_pixels,
    };
    let enabled = graphics.support == GraphicsSupport::Kitty
        && matches!(level, ColorLevel::TrueColor | ColorLevel::Ansi256)
        && limits.columns > 0
        && limits.rows > 0;
    let requests = if enabled {
        vec![ImageRequest {
            src: props.src.clone(),
            limits,
            color_level: level,
            animate: true,
        }]
    } else {
        Vec::new()
    };
    let statuses = use_images(cx, &requests);
    let placement = match statuses.first() {
        Some(ImageStatus::Ready(image)) => {
            size_in_cells(image.natural, limits).map(|size| image.place(size))
        }
        _ => None,
    };
    use_image_effect(cx, placement.iter().cloned().collect());
    if let Some(placement) = placement {
        View::new()
            .flex_direction(FlexDirection::Column)
            .flex_shrink(0.0)
            .children(placeholder_rows(&placement))
            .into()
    } else {
        Text::new(format!("[{}]", props.alt))
            .color(theme.color)
            .dim_color(theme.dim)
            .italic(theme.italic)
            .into()
    }
}

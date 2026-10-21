use ink_rs::{Color, Scope};

/// Color tokens that component themes are built from.
///
/// [`Scope::provide`] a palette to recolor the components beneath it. A
/// component theme provided beneath it takes precedence for that component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Focused rows and indicators, and spinner frames.
    pub primary: Color,
    pub info: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    /// Progress bars and badges.
    pub accent: Color,
}

impl Default for Palette {
    /// The colors of Ink UI's default theme.
    fn default() -> Self {
        Palette {
            primary: Color::Blue,
            info: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            accent: Color::Magenta,
        }
    }
}

/// Returns the nearest provided `T`, or a `T` built from the nearest
/// [`Palette`], or from the default palette.
///
/// The lookup calls two context hooks, so it follows the hook rules: call it
/// on every render, in the same order.
pub fn use_theme<T>(cx: &mut Scope) -> T
where
    T: Clone + 'static + for<'a> From<&'a Palette>,
{
    let theme = cx.use_context::<T>();
    let palette = cx.use_context::<Palette>();
    theme.unwrap_or_else(|| T::from(&palette.unwrap_or_default()))
}

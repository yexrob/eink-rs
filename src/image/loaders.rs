use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

pub(super) const MAX_BYTES: usize = 20 * 1024 * 1024;
pub(super) const LOCAL_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const NETWORK_TIMEOUT: Duration = Duration::from_secs(10);

/// An image could not be loaded, decoded, or prepared for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadError(String);

impl LoadError {
    /// Creates an error with a description of the failure.
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// Loads an encoded image from one kind of source.
pub trait ImageLoader: Send + Sync {
    /// Runs on a loader thread and returns the encoded bytes.
    fn load(&self, src: &str) -> Result<Vec<u8>, LoadError>;
}

/// Loaders selected by source kind and provided through [`ink_rs::Scope::provide`].
///
/// HTTP and HTTPS sources use `network`; other sources use `local`.
/// An empty slot disables that kind of source. Equality compares loader identity.
#[derive(Clone)]
pub struct ImageLoaders {
    /// The loader for paths and other local source names.
    pub local: Option<Arc<dyn ImageLoader>>,
    /// The loader for HTTP and HTTPS URLs.
    pub network: Option<Arc<dyn ImageLoader>>,
}

impl PartialEq for ImageLoaders {
    fn eq(&self, other: &Self) -> bool {
        same_loader(&self.local, &other.local) && same_loader(&self.network, &other.network)
    }
}

pub(super) fn same_loader(
    left: &Option<Arc<dyn ImageLoader>>,
    right: &Option<Arc<dyn ImageLoader>>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

impl Default for ImageLoaders {
    /// Uses files relative to the current directory and HTTP with `image-http`.
    fn default() -> Self {
        static DEFAULT: OnceLock<ImageLoaders> = OnceLock::new();
        DEFAULT
            .get_or_init(|| Self {
                local: Some(Arc::new(FileLoader::default())),
                #[cfg(feature = "image-http")]
                network: Some(Arc::new(HttpLoader::default())),
                #[cfg(not(feature = "image-http"))]
                network: None,
            })
            .clone()
    }
}

pub(crate) fn is_network(src: &str) -> bool {
    src.get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || src
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
}

/// Loads files relative to a base directory, with a 20 MiB size limit.
#[derive(Clone, Debug)]
pub struct FileLoader {
    base: PathBuf,
}

impl FileLoader {
    /// Creates a loader with the given base directory.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }
}

impl Default for FileLoader {
    fn default() -> Self {
        Self::new(".")
    }
}

impl ImageLoader for FileLoader {
    fn load(&self, src: &str) -> Result<Vec<u8>, LoadError> {
        let file = File::open(self.base.join(src))?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(LoadError::new("image exceeds 20 MiB"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        check_length(&bytes)?;
        Ok(bytes)
    }
}

pub(super) fn check_length(bytes: &[u8]) -> Result<(), LoadError> {
    if bytes.len() > MAX_BYTES {
        Err(LoadError::new("image exceeds 20 MiB"))
    } else {
        Ok(())
    }
}

/// Loads HTTP and HTTPS images with a ten-second deadline and a 20 MiB body limit.
#[cfg(feature = "image-http")]
#[derive(Clone, Debug)]
pub struct HttpLoader {
    agent: ureq::Agent,
}

#[cfg(feature = "image-http")]
impl Default for HttpLoader {
    fn default() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(NETWORK_TIMEOUT))
                .build()
                .into(),
        }
    }
}

#[cfg(feature = "image-http")]
impl ImageLoader for HttpLoader {
    fn load(&self, src: &str) -> Result<Vec<u8>, LoadError> {
        if !is_network(src) {
            return Err(LoadError::new("expected an HTTP or HTTPS URL"));
        }
        self.agent
            .get(src)
            .call()
            .and_then(|mut response| {
                response
                    .body_mut()
                    .with_config()
                    .limit(MAX_BYTES as u64)
                    .read_to_vec()
            })
            .map_err(|error| LoadError::new(error.to_string()))
    }
}

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ink_rs::ColorLevel;

use super::decode::{ImageLimits, prepare};
use super::loaders::{LOCAL_TIMEOUT, NETWORK_TIMEOUT, is_network};
use super::{ImageLoader, LoadError, LoadedImage};

/// A source's state, including its natural pixel dimensions when ready.
#[derive(Clone)]
pub(crate) enum ImageStatus {
    Loading,
    Ready(Arc<LoadedImage>),
    Failed,
}

#[derive(Clone)]
pub(crate) struct ImageRequest {
    pub src: String,
    pub limits: ImageLimits,
    pub color_level: ColorLevel,
    pub animate: bool,
}

type Completion = Box<dyn FnOnce(Arc<Job>, Result<Arc<LoadedImage>, LoadError>) + Send>;

pub(crate) struct Job {
    // Dropping completion releases the wake even if a custom loader blocks.
    completion: Mutex<Option<Completion>>,
    start: Instant,
    pub timeout: Duration,
}

impl Job {
    pub fn new(
        src: &str,
        complete: impl FnOnce(Arc<Job>, Result<Arc<LoadedImage>, LoadError>) + Send + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            completion: Mutex::new(Some(Box::new(complete))),
            start: Instant::now(),
            timeout: if is_network(src) {
                NETWORK_TIMEOUT
            } else {
                LOCAL_TIMEOUT
            },
        })
    }

    pub fn cancel(&self) {
        self.completion.lock().expect("image job").take();
    }

    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.start.elapsed())
    }

    fn expired(&self) -> bool {
        self.remaining().is_zero() || self.completion.lock().expect("image job").is_none()
    }

    fn complete(self: &Arc<Self>, result: Result<Arc<LoadedImage>, LoadError>) {
        let completion = self.completion.lock().expect("image job").take();
        if let Some(completion) = completion {
            completion(self.clone(), result);
        }
    }
}

pub(crate) fn start_load(request: ImageRequest, loader: Arc<dyn ImageLoader>, job: Arc<Job>) {
    let worker_job = job.clone();
    let result = std::thread::Builder::new()
        .name("eink-image".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let bytes = loader.load(&request.src)?;
                if worker_job.expired() {
                    return Err(LoadError::new("image load timed out"));
                }
                let image = prepare(&bytes, request.limits, request.animate, || {
                    worker_job.expired()
                })?;
                LoadedImage::new(image, request.color_level).map(Arc::new)
            }))
            .unwrap_or_else(|_| Err(LoadError::new("image loader panicked")));
            let result = if worker_job.expired() {
                Err(LoadError::new("image load timed out"))
            } else {
                result
            };
            worker_job.complete(result);
        });
    if let Err(error) = result {
        job.complete(Err(error.into()));
    }
}

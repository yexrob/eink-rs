use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use ink_rs::{AnimationOptions, ColorLevel, Scope, State};

use super::load::{ImageRequest, ImageStatus, Job, start_load};
use super::loaders::{is_network, same_loader};
use super::{ImageLoader, ImageLoaders, LoadError, LoadedImage};

#[derive(Clone)]
struct Key {
    src: String,
    occurrence: usize,
    loader: Option<Arc<dyn ImageLoader>>,
    color_level: ColorLevel,
    animate: bool,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src
            && self.occurrence == other.occurrence
            && self.color_level == other.color_level
            && self.animate == other.animate
            && same_loader(&self.loader, &other.loader)
    }
}

struct Entry {
    key: Key,
    status: ImageStatus,
    job: Arc<Job>,
    started: Duration,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.job.cancel();
    }
}

#[derive(Default)]
struct Model {
    entries: Vec<Entry>,
    time: Duration,
}

type Store = State<Rc<RefCell<Model>>>;

struct CancelLoads(Rc<RefCell<Model>>);

impl Drop for CancelLoads {
    fn drop(&mut self) {
        for entry in &self.0.borrow().entries {
            entry.job.cancel();
        }
    }
}

/// Loads a render's sources with a fixed hook count, preserving ready images on resize.
///
/// Call with an empty slice when graphics cannot be used. Keep sources in the
/// slice while their status is needed; removing a source cancels its pending job.
/// Repeated sources have separate identities so inline and block sizes can coexist.
pub(crate) fn use_images(cx: &mut Scope, requests: &[ImageRequest]) -> Vec<ImageStatus> {
    let loaders = cx.use_context::<ImageLoaders>().unwrap_or_default();
    let mut occurrences = HashMap::<&str, usize>::new();
    let keys: Vec<_> = requests
        .iter()
        .map(|request| {
            let occurrence = occurrences.entry(&request.src).or_default();
            let key = Key {
                src: request.src.clone(),
                occurrence: *occurrence,
                loader: if is_network(&request.src) {
                    loaders.network.clone()
                } else {
                    loaders.local.clone()
                },
                color_level: request.color_level,
                animate: request.animate,
            };
            *occurrence += 1;
            key
        })
        .collect();
    let store = cx.use_state(|| Rc::new(RefCell::new(Model::default())));
    let entries = store.get();
    let statuses: Vec<_> = keys
        .iter()
        .map(|key| {
            if key.loader.is_none() {
                return ImageStatus::Failed;
            }
            entries
                .borrow()
                .entries
                .iter()
                .find(|entry| entry.key == *key)
                .map_or(ImageStatus::Loading, |entry| entry.status.clone())
        })
        .collect();
    let active = statuses
        .iter()
        .any(|status| matches!(status, ImageStatus::Loading));
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(50),
        active,
    });
    cx.use_layout_effect((clock.time, active), move || {
        if active {
            store.update(|entries| {
                let mut model = entries.borrow_mut();
                model.time = clock.time;
                for entry in &mut model.entries {
                    if matches!(entry.status, ImageStatus::Loading)
                        && clock.time.saturating_sub(entry.started) >= entry.job.timeout
                    {
                        entry.job.cancel();
                        entry.status = ImageStatus::Failed;
                    }
                }
            });
        }
    });
    let app = cx.use_app();
    let requests = requests.to_vec();
    cx.use_effect(keys.clone(), move || {
        store.update(|entries| {
            let mut model = entries.borrow_mut();
            let started = model.time;
            let entries = &mut model.entries;
            entries.retain(|entry| keys.contains(&entry.key));
            for (key, request) in keys.into_iter().zip(requests) {
                let Some(loader) = key.loader.clone() else {
                    continue;
                };
                if entries.iter().any(|entry| entry.key == key) {
                    continue;
                }
                let completion_key = key.clone();
                let app = app.clone();
                let job = Job::new(&key.src, move |job, result| {
                    complete(store, completion_key, job, result, app);
                });
                entries.push(Entry {
                    key: key.clone(),
                    status: ImageStatus::Loading,
                    job: job.clone(),
                    started,
                });
                start_load(request, loader, job);
            }
        });
    });
    cx.use_effect((), move || CancelLoads(entries));
    super::protocol::use_playback(cx, &statuses);
    statuses
}

fn complete(
    store: Store,
    key: Key,
    job: Arc<Job>,
    result: Result<Arc<LoadedImage>, LoadError>,
    app: ink_rs::AppHandle,
) {
    app.update(move || {
        store.update(|entries| {
            if let Some(entry) = entries
                .borrow_mut()
                .entries
                .iter_mut()
                .find(|entry| entry.key == key && Arc::ptr_eq(&entry.job, &job))
                && matches!(entry.status, ImageStatus::Loading)
            {
                entry.status = match result {
                    Ok(image) => ImageStatus::Ready(image),
                    Err(_) => ImageStatus::Failed,
                };
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::super::ImageLimits;
    use super::*;
    use ink_rs::{Element, Text, component};
    use std::io::Cursor;
    use std::sync::{Mutex, mpsc};
    use std::time::Instant;

    struct Loader {
        results: Mutex<mpsc::Receiver<Result<Vec<u8>, LoadError>>>,
        entered: mpsc::Sender<()>,
    }

    impl ImageLoader for Loader {
        fn load(&self, _: &str) -> Result<Vec<u8>, LoadError> {
            let results = self.results.lock().unwrap();
            let _ = self.entered.send(());
            results
                .recv()
                .map_err(|error| LoadError::new(error.to_string()))?
        }
    }

    fn provider(cx: &mut Scope, props: &(String, ImageLoaders)) -> Element {
        cx.provide(props.1.clone());
        component(status, (props.0.clone(),))
    }

    fn status(cx: &mut Scope, src: &(String,)) -> Element {
        let states = use_images(
            cx,
            &[ImageRequest {
                src: src.0.clone(),
                limits: ImageLimits {
                    columns: 10,
                    rows: 20,
                    cell_pixels: Some((10, 20)),
                },
                color_level: ColorLevel::TrueColor,
                animate: true,
            }],
        );
        Text::new(match &states[0] {
            ImageStatus::Loading => "loading".to_owned(),
            ImageStatus::Ready(_) => "ready".to_owned(),
            ImageStatus::Failed => "failed".to_owned(),
        })
        .into()
    }

    fn wait_for(app: &mut ink_rs::testing::TestInstance, frame: &str) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while app.last_frame() != frame {
            assert!(Instant::now() < deadline, "image status did not settle");
            app.advance(Duration::from_millis(1));
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn deadline_restarts_after_ready_and_failures_settle_without_a_timeout() {
        let (send, receive) = mpsc::channel();
        let (entered, starts) = mpsc::channel();
        let loaders = ImageLoaders {
            local: Some(Arc::new(Loader {
                results: Mutex::new(receive),
                entered,
            })),
            network: None,
        };
        let mut app =
            ink_rs::testing::render(component(provider, ("first".into(), loaders.clone())));
        assert_eq!(app.last_frame(), "loading");
        starts.recv_timeout(Duration::from_secs(3)).unwrap();
        app.advance(Duration::from_secs(2));
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::new(20, 40)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        send.send(Ok(bytes.into_inner())).unwrap();
        wait_for(&mut app, "ready");

        app.rerender(component(provider, ("second".into(), loaders.clone())));
        starts.recv_timeout(Duration::from_secs(3)).unwrap();
        app.advance(Duration::from_millis(4900));
        assert_eq!(app.last_frame(), "loading");
        app.advance(Duration::from_millis(200));
        assert_eq!(app.last_frame(), "failed");
        send.send(Err(LoadError::new("late failure"))).unwrap();
        app.rerender(component(provider, ("third".into(), loaders)));
        send.send(Err(LoadError::new("missing image"))).unwrap();
        wait_for(&mut app, "failed");
    }
}

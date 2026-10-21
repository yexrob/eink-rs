use std::{cell::RefCell, rc::Rc, time::Duration};

use ink_rs::Scope;

/// Availability reported by the fleet and instance views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Health {
    Healthy,
    Degraded,
    Offline,
}

impl Health {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Offline => "offline",
        }
    }
}

/// A service snapshot with resource utilization percentages.
#[derive(Clone, PartialEq)]
pub(crate) struct Service {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) replicas: u8,
    pub(crate) health: Health,
    pub(crate) cpu: f64,
    pub(crate) memory: f64,
}

/// One running replica and its placement.
#[derive(Clone)]
pub(crate) struct Instance {
    pub(crate) id: String,
    pub(crate) region: String,
    pub(crate) health: Health,
    pub(crate) uptime: Duration,
}

/// A version available for deployment, ordered from newest to oldest.
#[derive(Clone)]
pub(crate) struct Release {
    pub(crate) version: String,
    pub(crate) age: String,
    pub(crate) author: String,
}

/// A deployment destination and its health percentage.
#[derive(Clone)]
pub(crate) struct Region {
    pub(crate) name: String,
    pub(crate) health: f64,
}

/// The active fleet advisory.
#[derive(Clone)]
pub(crate) struct Incident {
    pub(crate) title: String,
    pub(crate) message: String,
}

/// A running phase or a terminal deployment outcome.
#[derive(Clone, PartialEq)]
pub(crate) enum DeploymentState {
    Running(String),
    Succeeded,
    Failed(String),
}

/// Backend-owned work that survives navigation away from its observer.
#[derive(Clone, PartialEq)]
pub(crate) struct Deployment {
    pub(crate) id: u64,
    pub(crate) service: String,
    pub(crate) version: String,
    pub(crate) regions: Vec<String>,
    pub(crate) progress: f64,
    pub(crate) state: DeploymentState,
}

/// Semantic log severity for themed rendering.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Level {
    Info,
    Warn,
    Error,
}

/// A cursor-addressable event; seconds are relative to application mount.
#[derive(Clone, PartialEq)]
pub(crate) struct LogLine {
    pub(crate) id: u64,
    pub(crate) second: u64,
    pub(crate) level: Level,
    pub(crate) message: String,
}

/// Session credentials stored in memory and never sent to a server.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct Account {
    pub(crate) email: String,
    pub(crate) token: String,
}

/// The console's data boundary; commands and routes know no implementation.
pub(crate) trait Backend {
    /// Brings the data up to `elapsed` after mount; the app calls this on each clock tick.
    fn refresh(&mut self, elapsed: Duration);
    /// Lists the fleet in display order.
    fn services(&self) -> Vec<Service>;
    /// Looks up a name, with known names in the error when absent.
    fn service(&self, name: &str) -> Result<Service, String>;
    /// Lists a service's replicas.
    fn instances(&self, name: &str) -> Vec<Instance>;
    /// Lists deployable versions from newest to oldest.
    fn releases(&self, name: &str) -> Vec<Release>;
    /// Lists valid deployment destinations.
    fn regions(&self) -> Vec<Region>;
    /// Returns the current advisory, if one is open.
    fn incident(&self) -> Option<Incident>;
    /// Reports whether a fleet sync is in progress.
    fn syncing(&self) -> bool;
    /// Lists completed and running work from newest to oldest.
    fn deployments(&self) -> Vec<Deployment>;
    /// With a cursor, returns following lines; without one, returns the tail.
    fn logs(&self, name: &str, after: Option<u64>, limit: usize) -> Vec<LogLine>;
    /// Changes replica count after checking the supported range.
    fn scale(&mut self, name: &str, replicas: u8) -> Result<(), String>;
    /// Starts work and returns its stable identifier.
    fn deploy(&mut self, name: &str, version: &str, regions: &[String]) -> Result<u64, String>;
    /// Reads session credentials.
    fn account(&self) -> Account;
    /// Validates and stores session credentials.
    fn save_account(&mut self, account: Account) -> Result<(), String>;
}

/// A shared backend whose identity is its allocation, not its contents.
#[derive(Clone)]
pub(crate) struct BackendHandle(pub(crate) Rc<RefCell<dyn Backend>>);

impl PartialEq for BackendHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Invalidates readers after the clock has advanced the backend.
#[derive(Clone, PartialEq)]
pub(crate) struct Revision(pub(crate) Duration);

/// Reads the backend and subscribes the component to clock revisions.
pub(crate) fn use_backend(cx: &mut Scope) -> BackendHandle {
    cx.use_context::<Revision>();
    cx.use_context().expect("ship backend provider")
}

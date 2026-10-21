use std::{collections::BTreeMap, time::Duration};

use crate::backend::*;

const REQUESTS: [&str; 5] = [
    "GET /health",
    "GET /v1/items",
    "GET /v1/users/me",
    "POST /v1/orders",
    "PUT /v1/carts",
];

/// An in-memory world driven by elapsed time, with no network or threads.
pub(crate) struct Simulator {
    second: u64,
    random: u64,
    services: Vec<Service>,
    logs: BTreeMap<String, Vec<LogLine>>,
    deployments: Vec<(u64, Deployment)>,
    account: Account,
}

impl Simulator {
    pub(crate) fn new() -> Self {
        let services: Vec<_> = [("api", 3), ("web", 2), ("worker", 1)]
            .into_iter()
            .map(|(name, replicas)| Service {
                name: name.into(),
                version: "v1.4.0".into(),
                replicas,
                health: Health::Healthy,
                cpu: 24.0,
                memory: 42.0,
            })
            .collect();
        let logs = services
            .iter()
            .map(|service| {
                let lines = (1..=30)
                    .map(|id| LogLine {
                        id,
                        second: 0,
                        level: Level::Info,
                        message: format!("{} request {id}: 200 OK", service.name),
                    })
                    .collect();
                (service.name.clone(), lines)
            })
            .collect();
        Self {
            second: 0,
            random: 0x5eed,
            services,
            logs,
            deployments: vec![(
                0,
                Deployment {
                    id: 1,
                    service: "web".into(),
                    version: "v1.4.0".into(),
                    regions: vec!["us-east".into()],
                    progress: 100.0,
                    state: DeploymentState::Succeeded,
                },
            )],
            account: Account {
                email: "operator@ship.dev".into(),
                token: String::new(),
            },
        }
    }

    fn tick(&mut self) {
        self.second += 1;
        for (index, service) in self.services.iter_mut().enumerate() {
            let random = splitmix(&mut self.random);
            service.cpu = 12.0 + (random % 68) as f64;
            service.memory = 30.0 + ((random >> 8) % 45) as f64;
            service.health = if service.replicas == 0 {
                Health::Offline
            } else if index == 0 && (12..24).contains(&(self.second % 40)) {
                Health::Degraded
            } else {
                Health::Healthy
            };
            let lines = self.logs.get_mut(&service.name).expect("seeded log stream");
            let id = lines.last().map_or(1, |line| line.id + 1);
            let request = REQUESTS[(random >> 16) as usize % REQUESTS.len()];
            let latency = 8 + (random >> 24) % 120;
            let (level, message) = match service.health {
                Health::Healthy => (Level::Info, format!("{request} 200 {latency}ms")),
                Health::Degraded => (
                    Level::Warn,
                    format!(
                        "{request} 200 {}ms, upstream latency above budget",
                        latency + 400
                    ),
                ),
                Health::Offline => (Level::Error, "no replicas available".into()),
            };
            lines.push(LogLine {
                id,
                second: self.second,
                level,
                message: format!("{} {message}", service.name),
            });
            if lines.len() > 2000 {
                lines.remove(0);
            }
        }
        for (started, deployment) in &mut self.deployments {
            if !matches!(deployment.state, DeploymentState::Running(_)) {
                continue;
            }
            let elapsed = self.second - *started;
            let phases = 2 + deployment.regions.len() as u64;
            deployment.progress = (elapsed as f64 / (phases * 3) as f64 * 100.0).min(100.0);
            if deployment.service == "worker" && deployment.version == "v2.0.0" && elapsed >= 6 {
                deployment.state =
                    DeploymentState::Failed("worker v2.0.0: health check rejected".into());
            } else if elapsed >= phases * 3 {
                deployment.state = DeploymentState::Succeeded;
                let service = self
                    .services
                    .iter_mut()
                    .find(|s| s.name == deployment.service)
                    .expect("validated service");
                service.version = deployment.version.clone();
            } else {
                deployment.state = DeploymentState::Running(match elapsed / 3 {
                    0 => "build".into(),
                    1 => "push".into(),
                    phase => format!("rollout {}", deployment.regions[phase as usize - 2]),
                });
            }
        }
    }
}

/// Advances a SplitMix64 state and returns its next output.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl Backend for Simulator {
    fn refresh(&mut self, elapsed: Duration) {
        while self.second < elapsed.as_secs() {
            self.tick();
        }
    }
    fn services(&self) -> Vec<Service> {
        self.services.clone()
    }
    fn service(&self, name: &str) -> Result<Service, String> {
        self.services
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "unknown service '{name}'; known services: {}",
                    self.services
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    }
    fn instances(&self, name: &str) -> Vec<Instance> {
        let Ok(service) = self.service(name) else {
            return vec![];
        };
        (0..service.replicas)
            .map(|i| Instance {
                id: format!("{name}-{:02}", i + 1),
                region: ["us-east", "eu-west", "ap-south"][i as usize % 3].into(),
                health: service.health,
                uptime: Duration::from_secs(3600 + self.second),
            })
            .collect()
    }
    fn releases(&self, name: &str) -> Vec<Release> {
        if self.service(name).is_err() {
            return vec![];
        }
        [
            ("v2.0.0", "candidate", "lee"),
            ("v1.4.0", "1h", "sam"),
            ("v1.3.0", "1d", "alex"),
            ("v1.2.0", "3d", "jo"),
        ]
        .into_iter()
        .map(|(version, age, author)| Release {
            version: version.into(),
            age: age.into(),
            author: author.into(),
        })
        .collect()
    }
    fn regions(&self) -> Vec<Region> {
        ["us-east", "eu-west", "ap-south"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| Region {
                name: name.into(),
                health: if i == 0 && self.incident().is_some() {
                    82.0
                } else {
                    100.0
                },
            })
            .collect()
    }
    fn incident(&self) -> Option<Incident> {
        (12..24).contains(&(self.second % 40)).then(|| Incident {
            title: "API latency / us-east".into(),
            message: "Elevated upstream latency. Recovery is in progress.".into(),
        })
    }
    fn syncing(&self) -> bool {
        self.second % 10 < 2
    }
    fn deployments(&self) -> Vec<Deployment> {
        self.deployments
            .iter()
            .rev()
            .map(|(_, d)| d.clone())
            .collect()
    }
    fn logs(&self, name: &str, after: Option<u64>, limit: usize) -> Vec<LogLine> {
        let Some(lines) = self.logs.get(name) else {
            return vec![];
        };
        match after {
            Some(id) => lines
                .iter()
                .filter(|line| line.id > id)
                .take(limit)
                .cloned()
                .collect(),
            None => lines[lines.len().saturating_sub(limit)..].to_vec(),
        }
    }
    fn scale(&mut self, name: &str, replicas: u8) -> Result<(), String> {
        self.service(name)?;
        if replicas > 20 {
            return Err("replicas must be between 0 and 20".into());
        }
        let service = self
            .services
            .iter_mut()
            .find(|s| s.name == name)
            .expect("validated service");
        service.replicas = replicas;
        service.health = if replicas == 0 {
            Health::Offline
        } else {
            Health::Healthy
        };
        Ok(())
    }
    fn deploy(&mut self, name: &str, version: &str, regions: &[String]) -> Result<u64, String> {
        self.service(name)?;
        if !self.releases(name).iter().any(|r| r.version == version) {
            return Err("choose a release".into());
        }
        let known = self.regions();
        if regions.is_empty() || regions.iter().any(|r| !known.iter().any(|k| &k.name == r)) {
            return Err("choose at least one known region".into());
        }
        if self
            .deployments
            .iter()
            .any(|(_, d)| d.service == name && matches!(d.state, DeploymentState::Running(_)))
        {
            return Err(format!("{name} has a running deployment"));
        }
        let id = self.deployments.last().map_or(1, |(_, d)| d.id + 1);
        self.deployments.push((
            self.second,
            Deployment {
                id,
                service: name.into(),
                version: version.into(),
                regions: regions.to_vec(),
                progress: 0.0,
                state: DeploymentState::Running("build".into()),
            },
        ));
        Ok(id)
    }
    fn account(&self) -> Account {
        self.account.clone()
    }
    fn save_account(&mut self, account: Account) -> Result<(), String> {
        let valid_email = account
            .email
            .split_once('@')
            .is_some_and(|(local, domain)| {
                !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
            })
            && !account.email.chars().any(char::is_whitespace);
        if !valid_email || account.token.is_empty() {
            return Err("enter an email address and an API token".into());
        }
        self.account = account;
        Ok(())
    }
}

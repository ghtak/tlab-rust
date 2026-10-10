use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use chrono::{DateTime, Utc};
use sysinfo::{CpuRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{
    sync::RwLock,
    task::JoinHandle,
    time::{Duration, MissedTickBehavior},
};

use crate::app_container::AppContainer;

#[derive(Clone, serde::Serialize)]
pub struct SystemSnapshot {
    pub sampled_at: DateTime<Utc>,
    pub scope: &'static str,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub processes_sampled_at: DateTime<Utc>,
    pub processes: Vec<ProcessSnapshot>,
}

#[derive(Clone, serde::Serialize)]
pub struct CpuSnapshot {
    pub total_usage_percent: Option<f32>,
    pub cores_usage_percent: Option<Vec<f32>>,
}

#[derive(Clone, serde::Serialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub cpu_usage_percent: f32,
}

struct Collector {
    system: System,
    samples: u64,
    processes_sampled_at: Option<DateTime<Utc>>,
    processes: Vec<ProcessSnapshot>,
}

impl Collector {
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_list(CpuRefreshKind::nothing().with_cpu_usage());
        Self {
            system,
            samples: 0,
            processes_sampled_at: None,
            processes: Vec::new(),
        }
    }

    fn sample(&mut self) -> SystemSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        let sampled_at = Utc::now();

        if self.samples.is_multiple_of(3) {
            self.system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory()
                    .without_tasks(),
            );
            let mut processes: Vec<_> = self
                .system
                .processes()
                .iter()
                .map(|(pid, process)| ProcessSnapshot {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    memory_bytes: process.memory(),
                    cpu_usage_percent: process.cpu_usage(),
                })
                .collect();
            processes.sort_unstable_by(|a, b| {
                b.memory_bytes
                    .cmp(&a.memory_bytes)
                    .then_with(|| a.pid.cmp(&b.pid))
            });
            processes.truncate(20);
            self.processes = processes;
            self.processes_sampled_at = Some(sampled_at);
        }

        let cpu = if self.samples == 0 {
            CpuSnapshot {
                total_usage_percent: None,
                cores_usage_percent: None,
            }
        } else {
            CpuSnapshot {
                total_usage_percent: Some(self.system.global_cpu_usage()),
                cores_usage_percent: Some(
                    self.system
                        .cpus()
                        .iter()
                        .map(|cpu| cpu.cpu_usage())
                        .collect(),
                ),
            }
        };
        self.samples += 1;

        SystemSnapshot {
            sampled_at,
            scope: "host",
            cpu,
            memory: MemorySnapshot {
                total_bytes: self.system.total_memory(),
                available_bytes: self.system.available_memory(),
            },
            processes_sampled_at: self.processes_sampled_at.unwrap_or(sampled_at),
            processes: self.processes.clone(),
        }
    }
}

pub struct Metrics {
    latest: RwLock<Option<SystemSnapshot>>,
    requests_in_flight: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            latest: RwLock::new(None),
            requests_in_flight: AtomicU64::new(0),
        }
    }

    pub fn start(metrics: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut collector = match tokio::task::spawn_blocking(Collector::new).await {
                Ok(collector) => collector,
                Err(error) => {
                    tracing::error!(?error, "Failed to initialize system metrics");
                    return;
                }
            };
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let (next_collector, snapshot) = match tokio::task::spawn_blocking(move || {
                    let snapshot = collector.sample();
                    (collector, snapshot)
                })
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::error!(?error, "Failed to collect system metrics");
                        break;
                    }
                };
                collector = next_collector;
                *metrics.latest.write().await = Some(snapshot);
            }
        })
    }

    pub async fn latest(&self) -> Option<SystemSnapshot> {
        self.latest.read().await.clone()
    }

    pub fn requests_in_flight(&self) -> u64 {
        self.requests_in_flight.load(Ordering::Relaxed)
    }

    fn track_request(&self) -> RequestGuard<'_> {
        self.requests_in_flight.fetch_add(1, Ordering::Relaxed);
        RequestGuard(&self.requests_in_flight)
    }
}

struct RequestGuard<'a>(&'a AtomicU64);

impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

pub async fn track_requests(
    State(container): State<Arc<AppContainer>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let _guard = container.metrics.track_request();
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::{Collector, Metrics};

    #[test]
    fn first_sample_omits_cpu_usage_and_limits_processes() {
        let snapshot = Collector::new().sample();
        assert_eq!(snapshot.scope, "host");
        assert!(snapshot.cpu.total_usage_percent.is_none());
        assert!(snapshot.cpu.cores_usage_percent.is_none());
        assert!(snapshot.processes.len() <= 20);
        assert!(
            snapshot
                .processes
                .windows(2)
                .all(|pair| { pair[0].memory_bytes >= pair[1].memory_bytes })
        );
    }

    #[test]
    fn request_count_returns_to_zero_when_guard_is_dropped() {
        let metrics = Metrics::new();
        let first = metrics.track_request();
        let second = metrics.track_request();
        assert_eq!(metrics.requests_in_flight(), 2);
        drop(first);
        assert_eq!(metrics.requests_in_flight(), 1);
        drop(second);
        assert_eq!(metrics.requests_in_flight(), 0);
    }
}

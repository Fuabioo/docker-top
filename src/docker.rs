use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{Context, Result};
use bollard::models::{ContainerStatsResponse, ContainerSummaryStateEnum};
use bollard::query_parameters::{ListContainersOptionsBuilder, StatsOptionsBuilder};
use bollard::Docker;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use futures::StreamExt;
use tokio::sync::Semaphore;

use crate::model::{ComposeProject, ContainerSnapshot};

/// Maximum concurrent stats/inspect fetches.
const MAX_CONCURRENT_FETCHES: usize = 20;

/// Cached container list (single-writer: only docker_poller calls fetch_projects).
static CONTAINER_CACHE: Mutex<Option<Vec<ContainerInfo>>> = Mutex::new(None);

#[derive(Debug, Clone)]
struct ContainerInfo {
    id: String,
    name: String,
    service: String,
    project: String,
    working_dir: String,
    status: String,
    running: bool,
}

/// Connect to Docker using local defaults.
pub fn connect() -> Result<Docker> {
    Docker::connect_with_local_defaults().context("Failed to connect to Docker")
}

/// Recover from a poisoned mutex rather than panicking.
fn lock_cache(cache: &Mutex<Option<Vec<ContainerInfo>>>) -> std::sync::MutexGuard<'_, Option<Vec<ContainerInfo>>> {
    cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Fetch compose projects. If `relist` is true, re-queries the container list.
pub async fn fetch_projects(client: &Docker, relist: bool) -> Result<Vec<ComposeProject>> {
    if relist || lock_cache(&CONTAINER_CACHE).is_none() {
        let infos = list_compose_containers(client).await?;
        *lock_cache(&CONTAINER_CACHE) = Some(infos);
    }

    let infos = lock_cache(&CONTAINER_CACHE).clone().unwrap_or_default();
    if infos.is_empty() {
        return Ok(Vec::new());
    }

    let snapshots = fetch_all_stats(client, &infos).await?;

    // Group by project
    let mut groups: HashMap<String, (String, Vec<ContainerSnapshot>)> = HashMap::new();
    for snap in snapshots {
        let info = infos.iter().find(|i| i.id == snap.id);
        let (project, working_dir) = match info {
            Some(i) => (i.project.clone(), i.working_dir.clone()),
            None => continue,
        };
        groups
            .entry(project)
            .or_insert_with(|| (working_dir, Vec::new()))
            .1
            .push(snap);
    }

    let projects: Vec<ComposeProject> = groups
        .into_iter()
        .map(|(name, (wd, containers))| ComposeProject::aggregate(name, wd, containers))
        .collect();

    Ok(projects)
}

/// List all containers that belong to a Docker Compose project.
async fn list_compose_containers(client: &Docker) -> Result<Vec<ContainerInfo>> {
    let filters: HashMap<String, Vec<String>> = HashMap::from([(
        "label".to_string(),
        vec!["com.docker.compose.project".to_string()],
    )]);

    let opts = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let containers = client.list_containers(Some(opts)).await?;
    let mut infos = Vec::new();

    for c in containers {
        let id = match &c.id {
            Some(id) => id.clone(),
            None => continue,
        };

        let labels = c.labels.unwrap_or_default();
        let project = match labels.get("com.docker.compose.project") {
            Some(p) => p.clone(),
            None => continue,
        };
        let service = labels
            .get("com.docker.compose.service")
            .cloned()
            .unwrap_or_default();
        let working_dir = labels
            .get("com.docker.compose.project.working_dir")
            .cloned()
            .unwrap_or_default();

        let name = c
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| id[..12].to_string());

        let status_str = c.status.clone().unwrap_or_default();
        let running = matches!(c.state, Some(ContainerSummaryStateEnum::RUNNING));

        infos.push(ContainerInfo {
            id,
            name,
            service,
            project,
            working_dir,
            status: status_str,
            running,
        });
    }

    Ok(infos)
}

/// Fetch stats (and inspect for start time) for all containers concurrently.
async fn fetch_all_stats(
    client: &Docker,
    infos: &[ContainerInfo],
) -> Result<Vec<ContainerSnapshot>> {
    let semaphore = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT_FETCHES));

    let futures: Vec<_> = infos
        .iter()
        .map(|info| {
            let client = client.clone();
            let info = info.clone();
            let sem = semaphore.clone();
            async move {
                let _permit = match sem.acquire().await {
                    Ok(p) => p,
                    Err(_) => return Err(anyhow::anyhow!("semaphore closed")),
                };
                fetch_container_stats(&client, &info).await
            }
        })
        .collect();

    let results = join_all(futures).await;
    let mut snapshots = Vec::new();
    for result in results {
        match result {
            Ok(snap) => snapshots.push(snap),
            Err(e) => {
                tracing::warn!("Failed to fetch stats for container: {}", e);
            }
        }
    }

    Ok(snapshots)
}

/// Fetch one-shot stats and inspect data for a single container.
async fn fetch_container_stats(client: &Docker, info: &ContainerInfo) -> Result<ContainerSnapshot> {
    // For non-running containers, return zeroed stats but still get started_at from inspect
    if !info.running {
        let started_at = fetch_started_at(client, &info.id).await;
        return Ok(ContainerSnapshot {
            id: info.id.clone(),
            name: info.name.clone(),
            service: info.service.clone(),
            status: info.status.clone(),
            running: false,
            cpu_percent: 0.0,
            mem_bytes: 0,
            mem_limit: 0,
            net_rx: 0,
            net_tx: 0,
            blk_read: 0,
            blk_write: 0,
            started_at,
        });
    }

    // Fetch stats and inspect concurrently
    let stats_fut = async {
        let opts = StatsOptionsBuilder::default()
            .stream(false)
            .one_shot(true)
            .build();
        let mut stream = client.stats(&info.id, Some(opts));
        stream.next().await
    };
    let inspect_fut = fetch_started_at(client, &info.id);

    let (stats_result, started_at) = tokio::join!(stats_fut, inspect_fut);

    let stats = match stats_result {
        Some(Ok(s)) => s,
        Some(Err(e)) => return Err(e.into()),
        None => {
            return Ok(ContainerSnapshot {
                id: info.id.clone(),
                name: info.name.clone(),
                service: info.service.clone(),
                status: info.status.clone(),
                running: info.running,
                cpu_percent: 0.0,
                mem_bytes: 0,
                mem_limit: 0,
                net_rx: 0,
                net_tx: 0,
                blk_read: 0,
                blk_write: 0,
                started_at,
            });
        }
    };

    let cpu_percent = compute_cpu(&stats);
    let (mem_bytes, mem_limit) = compute_memory(&stats);
    let (net_rx, net_tx) = compute_network(&stats);
    let (blk_read, blk_write) = compute_block_io(&stats);

    Ok(ContainerSnapshot {
        id: info.id.clone(),
        name: info.name.clone(),
        service: info.service.clone(),
        status: info.status.clone(),
        running: info.running,
        cpu_percent,
        mem_bytes,
        mem_limit,
        net_rx,
        net_tx,
        blk_read,
        blk_write,
        started_at,
    })
}

/// Get the actual started_at time from container inspect.
async fn fetch_started_at(client: &Docker, container_id: &str) -> Option<DateTime<Utc>> {
    let inspect = client.inspect_container(container_id, None).await.ok()?;
    let state = inspect.state?;
    let started_str = state.started_at?;
    started_str.parse::<DateTime<Utc>>().ok()
}

/// CPU%: (cpu_delta / system_delta) * online_cpus * 100.0
fn compute_cpu(stats: &ContainerStatsResponse) -> f64 {
    let cpu_stats = match &stats.cpu_stats {
        Some(s) => s,
        None => return 0.0,
    };
    let precpu_stats = match &stats.precpu_stats {
        Some(s) => s,
        None => return 0.0,
    };

    let cur_total = cpu_stats
        .cpu_usage
        .as_ref()
        .and_then(|u| u.total_usage)
        .unwrap_or(0);
    let prev_total = precpu_stats
        .cpu_usage
        .as_ref()
        .and_then(|u| u.total_usage)
        .unwrap_or(0);

    // Compute delta in integer space to avoid f64 precision loss on large values
    let cpu_delta = cur_total.wrapping_sub(prev_total) as f64;

    let system_delta = match (cpu_stats.system_cpu_usage, precpu_stats.system_cpu_usage) {
        (Some(cur), Some(prev)) => cur.wrapping_sub(prev) as f64,
        _ => return 0.0,
    };

    if system_delta <= 0.0 || cpu_delta < 0.0 {
        return 0.0;
    }

    let online_cpus = cpu_stats.online_cpus.unwrap_or(1);
    (cpu_delta / system_delta) * online_cpus as f64 * 100.0
}

/// Memory: usage - inactive_file (cgroup v2) or usage - total_inactive_file (cgroup v1)
fn compute_memory(stats: &ContainerStatsResponse) -> (u64, u64) {
    let mem = match &stats.memory_stats {
        Some(m) => m,
        None => return (0, 0),
    };
    let usage = mem.usage.unwrap_or(0);
    let limit = mem.limit.unwrap_or(0);

    let inactive: u64 = mem
        .stats
        .as_ref()
        .and_then(|s: &HashMap<String, u64>| {
            s.get("inactive_file")
                .or_else(|| s.get("total_inactive_file"))
                .copied()
        })
        .unwrap_or(0);

    let actual = usage.saturating_sub(inactive);
    (actual, limit)
}

/// Network: sum rx_bytes/tx_bytes across all interfaces.
fn compute_network(stats: &ContainerStatsResponse) -> (u64, u64) {
    let networks: &HashMap<String, _> = match &stats.networks {
        Some(n) => n,
        None => return (0, 0),
    };

    let mut rx: u64 = 0;
    let mut tx: u64 = 0;
    for net in networks.values() {
        rx += net.rx_bytes.unwrap_or(0);
        tx += net.tx_bytes.unwrap_or(0);
    }
    (rx, tx)
}

/// Block IO: sum read/write ops from io_service_bytes_recursive.
fn compute_block_io(stats: &ContainerStatsResponse) -> (u64, u64) {
    let blkio = match &stats.blkio_stats {
        Some(b) => b,
        None => return (0, 0),
    };
    let io = match &blkio.io_service_bytes_recursive {
        Some(entries) => entries,
        None => return (0, 0),
    };

    let mut read: u64 = 0;
    let mut write: u64 = 0;
    for entry in io {
        let op = match &entry.op {
            Some(o) => o.as_str(),
            None => continue,
        };
        let val = entry.value.unwrap_or(0);
        match op {
            "Read" | "read" => read += val,
            "Write" | "write" => write += val,
            _ => {}
        }
    }
    (read, write)
}

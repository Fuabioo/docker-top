use chrono::{DateTime, Utc};

/// Status of a compose project, derived from its containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    /// All containers running.
    Running,
    /// Some containers running, some stopped.
    Partial,
    /// All containers stopped.
    Stopped,
    /// At least one container in a dead/error state.
    Dead,
}

/// Per-container snapshot of stats at a point in time.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ContainerSnapshot {
    pub id: String,
    pub name: String,
    pub service: String,
    pub status: String,
    pub running: bool,
    pub cpu_percent: f64,
    pub mem_bytes: u64,
    pub mem_limit: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub blk_read: u64,
    pub blk_write: u64,
    pub started_at: Option<DateTime<Utc>>,
}

/// Aggregated view of a Docker Compose project.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ComposeProject {
    pub name: String,
    pub working_dir: String,
    pub containers: Vec<ContainerSnapshot>,
    pub status: ProjectStatus,
    // Aggregated metrics
    pub total_cpu: f64,
    pub total_mem: u64,
    pub mem_limit: u64,
    pub total_net_rx: u64,
    pub total_net_tx: u64,
    pub total_blk_read: u64,
    pub total_blk_write: u64,
    /// Oldest running container's started_at — how long the project has been up.
    pub oldest_started_at: Option<DateTime<Utc>>,
    /// Newest container's started_at — when the stack was last modified.
    pub newest_started_at: Option<DateTime<Utc>>,
}

impl ComposeProject {
    /// Build a ComposeProject from a set of containers, computing aggregates.
    pub fn aggregate(
        name: String,
        working_dir: String,
        containers: Vec<ContainerSnapshot>,
    ) -> Self {
        let total_cpu: f64 = containers.iter().map(|c| c.cpu_percent).sum();
        let total_mem: u64 = containers.iter().map(|c| c.mem_bytes).sum();
        // If all containers share the same limit (host RAM, no cgroup limit), use that value.
        // If containers have individual limits, sum them for the total project capacity.
        let mem_limit = {
            let limits: Vec<u64> = containers.iter().map(|c| c.mem_limit).collect();
            let all_same = limits.windows(2).all(|w| w[0] == w[1]);
            if all_same {
                limits.first().copied().unwrap_or(0)
            } else {
                limits.iter().sum()
            }
        };
        let total_net_rx: u64 = containers.iter().map(|c| c.net_rx).sum();
        let total_net_tx: u64 = containers.iter().map(|c| c.net_tx).sum();
        let total_blk_read: u64 = containers.iter().map(|c| c.blk_read).sum();
        let total_blk_write: u64 = containers.iter().map(|c| c.blk_write).sum();

        let running_starts: Vec<DateTime<Utc>> = containers
            .iter()
            .filter(|c| c.running)
            .filter_map(|c| c.started_at)
            .collect();

        let oldest_started_at = running_starts.iter().min().copied();

        let all_starts: Vec<DateTime<Utc>> =
            containers.iter().filter_map(|c| c.started_at).collect();

        let newest_started_at = all_starts.iter().max().copied();

        let status = Self::derive_status(&containers);

        Self {
            name,
            working_dir,
            containers,
            status,
            total_cpu,
            total_mem,
            mem_limit,
            total_net_rx,
            total_net_tx,
            total_blk_read,
            total_blk_write,
            oldest_started_at,
            newest_started_at,
        }
    }

    fn derive_status(containers: &[ContainerSnapshot]) -> ProjectStatus {
        if containers.is_empty() {
            return ProjectStatus::Stopped;
        }

        let has_dead = containers
            .iter()
            .any(|c| c.status.contains("dead") || c.status.contains("Dead"));
        if has_dead {
            return ProjectStatus::Dead;
        }

        let running_count = containers.iter().filter(|c| c.running).count();
        if running_count == containers.len() {
            ProjectStatus::Running
        } else if running_count > 0 {
            ProjectStatus::Partial
        } else {
            ProjectStatus::Stopped
        }
    }

    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    pub fn mem_percent(&self) -> f64 {
        if self.mem_limit == 0 {
            0.0
        } else {
            (self.total_mem as f64 / self.mem_limit as f64) * 100.0
        }
    }
}

/// Active view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Table,
    Chart,
}

/// Format a duration from a past timestamp to now as a human-readable string.
pub fn uptime_str(dt: DateTime<Utc>) -> String {
    let dur = Utc::now() - dt;
    let total_secs = dur.num_seconds();
    if total_secs < 0 {
        return "0s".to_string();
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

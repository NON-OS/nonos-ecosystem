use std::time::{Duration, Instant};

pub const MAX_RESTART_ATTEMPTS: u32 = 5;
pub const RESTART_BACKOFF_BASE_MS: u64 = 1000;
pub const RESTART_BACKOFF_MAX_MS: u64 = 60000;
pub const HEALTH_WINDOW_SIZE: usize = 100;
pub const MAX_RESTARTS_IN_WINDOW: u32 = 5;
pub const RESTART_RATE_WINDOW_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    Always,
    #[default]
    OnFailure,
    ExponentialBackoff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TaskState {
    Pending,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Terminated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum HealthClassification {
    Healthy,
    Degraded,
    Critical,
}

impl HealthClassification {
    pub fn from_health(health: &TaskHealth) -> Self {
        if health.is_restart_rate_exceeded() {
            return Self::Critical;
        }
        if health.health_score < 0.3 {
            return Self::Critical;
        }
        if health.restart_count >= MAX_RESTART_ATTEMPTS {
            return Self::Critical;
        }
        if health.health_score < 0.7 || health.restart_count > 2 {
            return Self::Degraded;
        }
        Self::Healthy
    }
}

#[derive(Clone, Debug)]
pub struct TaskHealth {
    pub state: TaskState,
    pub restart_count: u32,
    pub last_error: Option<String>,
    pub uptime_secs: u64,
    pub last_state_change: Instant,
    pub health_samples: Vec<bool>,
    pub health_score: f64,
    pub restart_timestamps: Vec<Instant>,
    pub classification: HealthClassification,
}

impl Default for TaskHealth {
    fn default() -> Self {
        Self {
            state: TaskState::Pending,
            restart_count: 0,
            last_error: None,
            uptime_secs: 0,
            last_state_change: Instant::now(),
            health_samples: Vec::with_capacity(HEALTH_WINDOW_SIZE),
            health_score: 1.0,
            restart_timestamps: Vec::new(),
            classification: HealthClassification::Healthy,
        }
    }
}

impl TaskHealth {
    pub fn record_sample(&mut self, healthy: bool) {
        self.health_samples.push(healthy);
        if self.health_samples.len() > HEALTH_WINDOW_SIZE {
            self.health_samples.remove(0);
        }
        self.update_health_score();
    }

    pub fn update_health_score(&mut self) {
        if self.health_samples.is_empty() {
            self.health_score = 1.0;
            return;
        }
        let healthy_count = self.health_samples.iter().filter(|&&h| h).count();
        self.health_score = healthy_count as f64 / self.health_samples.len() as f64;
    }

    pub fn update_uptime(&mut self) {
        if self.state == TaskState::Running {
            self.uptime_secs = self.last_state_change.elapsed().as_secs();
        }
    }

    pub fn record_restart(&mut self) {
        self.restart_timestamps.push(Instant::now());
        self.restart_count += 1;
        self.update_classification();
    }

    pub fn cleanup_old_restarts(&mut self) {
        let window = Duration::from_secs(RESTART_RATE_WINDOW_SECS);
        let now = Instant::now();
        self.restart_timestamps.retain(|&ts| now.duration_since(ts) < window);
    }

    pub fn is_restart_rate_exceeded(&self) -> bool {
        let window = Duration::from_secs(RESTART_RATE_WINDOW_SECS);
        let now = Instant::now();
        let recent_restarts = self
            .restart_timestamps
            .iter()
            .filter(|&&ts| now.duration_since(ts) < window)
            .count();
        recent_restarts >= MAX_RESTARTS_IN_WINDOW as usize
    }

    pub fn restarts_in_window(&self) -> u32 {
        let window = Duration::from_secs(RESTART_RATE_WINDOW_SECS);
        let now = Instant::now();
        self.restart_timestamps
            .iter()
            .filter(|&&ts| now.duration_since(ts) < window)
            .count() as u32
    }

    pub fn update_classification(&mut self) {
        self.classification = HealthClassification::from_health(self);
    }

    pub fn record_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.record_sample(false);
        self.update_classification();
    }
}

use super::types::{HealthClassification, TaskState};

#[derive(Clone, Debug, serde::Serialize)]
pub struct TaskMetrics {
    pub name: String,
    pub state: TaskState,
    pub classification: HealthClassification,
    pub restart_count: u32,
    pub restarts_in_window: u32,
    pub health_score: f64,
    pub uptime_secs: u64,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SupervisorStats {
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub failed_tasks: usize,
    pub total_spawned: u64,
    pub total_restarts: u64,
    pub uptime_secs: u64,
    pub healthy_tasks: usize,
    pub degraded_tasks: usize,
    pub critical_tasks: usize,
}

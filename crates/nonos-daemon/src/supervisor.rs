// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Task Supervisor Module
//!
//! Provides task supervision with:
//! - JoinHandle management for all async tasks
//! - Configurable restart policies (never, always, on-failure)
//! - Health tracking with sliding window
//! - Graceful shutdown with cancellation tokens
//! - Task introspection for monitoring

use nonos_types::{NonosError, NonosResult};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

/// Maximum restart attempts before giving up
const MAX_RESTART_ATTEMPTS: u32 = 5;

/// Backoff base delay for restarts
const RESTART_BACKOFF_BASE_MS: u64 = 1000;

/// Maximum backoff delay
const RESTART_BACKOFF_MAX_MS: u64 = 60000;

/// Health check window size
const HEALTH_WINDOW_SIZE: usize = 100;

/// Maximum restarts allowed in the restart window before marking unhealthy
const MAX_RESTARTS_IN_WINDOW: u32 = 5;

/// Window duration for restart rate limiting (in seconds)
const RESTART_RATE_WINDOW_SECS: u64 = 60;

/// Restart policy for supervised tasks
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never restart on failure
    Never,
    /// Always restart on failure
    Always,
    /// Restart only on failure (not clean exit)
    OnFailure,
    /// Restart with exponential backoff
    ExponentialBackoff,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::OnFailure
    }
}

/// Task state
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TaskState {
    /// Task is pending start
    Pending,
    /// Task is starting
    Starting,
    /// Task is running
    Running,
    /// Task is stopping
    Stopping,
    /// Task has stopped cleanly
    Stopped,
    /// Task has failed
    Failed,
    /// Task has been terminated
    Terminated,
}

/// Health classification for tasks
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum HealthClassification {
    /// Task is healthy and running normally
    Healthy,
    /// Task is degraded (restarts recently, lower health score)
    Degraded,
    /// Task is critical (too many restarts, likely to fail)
    Critical,
}

impl HealthClassification {
    /// Get classification from health metrics
    pub fn from_health(health: &TaskHealth) -> Self {
        // Critical if restarting too fast
        if health.is_restart_rate_exceeded() {
            return Self::Critical;
        }

        // Critical if health score is very low
        if health.health_score < 0.3 {
            return Self::Critical;
        }

        // Critical if too many restarts total
        if health.restart_count >= MAX_RESTART_ATTEMPTS {
            return Self::Critical;
        }

        // Degraded if health score is moderate or recent restarts
        if health.health_score < 0.7 || health.restart_count > 2 {
            return Self::Degraded;
        }

        Self::Healthy
    }
}

/// Task health status
#[derive(Clone, Debug)]
pub struct TaskHealth {
    /// Current state
    pub state: TaskState,
    /// Number of restarts
    pub restart_count: u32,
    /// Last error message if any
    pub last_error: Option<String>,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Time of last state change
    pub last_state_change: Instant,
    /// Recent health samples (true = healthy)
    pub health_samples: Vec<bool>,
    /// Overall health score (0.0 - 1.0)
    pub health_score: f64,
    /// Timestamps of recent restarts for rate limiting
    pub restart_timestamps: Vec<Instant>,
    /// Health classification
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
    fn record_sample(&mut self, healthy: bool) {
        self.health_samples.push(healthy);
        if self.health_samples.len() > HEALTH_WINDOW_SIZE {
            self.health_samples.remove(0);
        }
        self.update_health_score();
    }

    fn update_health_score(&mut self) {
        if self.health_samples.is_empty() {
            self.health_score = 1.0;
            return;
        }
        let healthy_count = self.health_samples.iter().filter(|&&h| h).count();
        self.health_score = healthy_count as f64 / self.health_samples.len() as f64;
    }

    fn update_uptime(&mut self) {
        if self.state == TaskState::Running {
            self.uptime_secs = self.last_state_change.elapsed().as_secs();
        }
    }

    /// Record a restart timestamp
    pub fn record_restart(&mut self) {
        self.restart_timestamps.push(Instant::now());
        self.restart_count += 1;
        self.update_classification();
    }

    /// Clean up old restart timestamps outside the window
    pub fn cleanup_old_restarts(&mut self) {
        let window = Duration::from_secs(RESTART_RATE_WINDOW_SECS);
        let now = Instant::now();
        self.restart_timestamps.retain(|&ts| now.duration_since(ts) < window);
    }

    /// Check if restart rate is exceeded
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

    /// Get the number of restarts in the current window
    pub fn restarts_in_window(&self) -> u32 {
        let window = Duration::from_secs(RESTART_RATE_WINDOW_SECS);
        let now = Instant::now();
        self.restart_timestamps
            .iter()
            .filter(|&&ts| now.duration_since(ts) < window)
            .count() as u32
    }

    /// Update health classification
    pub fn update_classification(&mut self) {
        self.classification = HealthClassification::from_health(self);
    }

    /// Record an error
    pub fn record_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.record_sample(false);
        self.update_classification();
    }
}

/// Supervised task information
struct SupervisedTask {
    /// Task name for identification
    name: String,
    /// Restart policy
    policy: RestartPolicy,
    /// Task handle
    handle: Option<JoinHandle<NonosResult<()>>>,
    /// Task health
    health: TaskHealth,
    /// Cancellation sender
    cancel_tx: watch::Sender<bool>,
    /// Task factory for restarts
    factory: Option<Box<dyn TaskFactory>>,
}

impl SupervisedTask {
    /// Get the task name
    fn name(&self) -> &str {
        &self.name
    }

    /// Get task summary for logging/debugging
    fn summary(&self) -> String {
        format!(
            "Task[{}] state={:?} restarts={} health={:.2}",
            self.name(),
            self.health.state,
            self.health.restart_count,
            self.health.health_score
        )
    }
}

/// Task factory trait for creating new task instances
trait TaskFactory: Send + Sync {
    fn create(&self, cancel_rx: watch::Receiver<bool>) -> JoinHandle<NonosResult<()>>;
}

struct TaskFactoryImpl<F, Fut>
where
    F: Fn(watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = NonosResult<()>> + Send + 'static,
{
    factory: F,
}

impl<F, Fut> TaskFactory for TaskFactoryImpl<F, Fut>
where
    F: Fn(watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = NonosResult<()>> + Send + 'static,
{
    fn create(&self, cancel_rx: watch::Receiver<bool>) -> JoinHandle<NonosResult<()>> {
        tokio::spawn((self.factory)(cancel_rx))
    }
}

/// Task supervisor for managing long-running async tasks
pub struct TaskSupervisor {
    /// All supervised tasks
    tasks: Arc<RwLock<HashMap<String, SupervisedTask>>>,
    /// Global shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Total tasks spawned
    total_spawned: AtomicU64,
    /// Total restarts performed
    total_restarts: AtomicU64,
    /// Supervisor started time
    started_at: Instant,
}

impl TaskSupervisor {
    /// Create a new task supervisor
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
            total_spawned: AtomicU64::new(0),
            total_restarts: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    /// Spawn a supervised task with the given name, policy, and future factory
    pub fn spawn<F, Fut>(&self, name: &str, policy: RestartPolicy, factory: F) -> NonosResult<()>
    where
        F: Fn(watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = NonosResult<()>> + Send + 'static,
    {
        let mut tasks = self.tasks.write();

        if tasks.contains_key(name) {
            return Err(NonosError::Internal(format!(
                "Task '{}' already exists",
                name
            )));
        }

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let factory_box: Box<dyn TaskFactory> = Box::new(TaskFactoryImpl { factory });
        let handle = factory_box.create(cancel_rx);

        let mut health = TaskHealth::default();
        health.state = TaskState::Running;
        health.last_state_change = Instant::now();

        tasks.insert(
            name.to_string(),
            SupervisedTask {
                name: name.to_string(),
                policy,
                handle: Some(handle),
                health,
                cancel_tx,
                factory: Some(factory_box),
            },
        );

        self.total_spawned.fetch_add(1, Ordering::Relaxed);
        info!("Spawned supervised task: {} (policy: {:?})", name, policy);

        Ok(())
    }

    /// Spawn a one-shot task (no restart, no factory needed)
    pub fn spawn_oneshot<Fut>(&self, name: &str, future: Fut) -> NonosResult<()>
    where
        Fut: Future<Output = NonosResult<()>> + Send + 'static,
    {
        let mut tasks = self.tasks.write();

        if tasks.contains_key(name) {
            return Err(NonosError::Internal(format!(
                "Task '{}' already exists",
                name
            )));
        }

        let (cancel_tx, _cancel_rx) = watch::channel(false);
        let handle = tokio::spawn(future);

        let mut health = TaskHealth::default();
        health.state = TaskState::Running;
        health.last_state_change = Instant::now();

        tasks.insert(
            name.to_string(),
            SupervisedTask {
                name: name.to_string(),
                policy: RestartPolicy::Never,
                handle: Some(handle),
                health,
                cancel_tx,
                factory: None,
            },
        );

        self.total_spawned.fetch_add(1, Ordering::Relaxed);
        debug!("Spawned oneshot task: {}", name);

        Ok(())
    }

    /// Stop a specific task by name
    pub async fn stop_task(&self, name: &str) -> NonosResult<()> {
        let mut tasks = self.tasks.write();

        if let Some(task) = tasks.get_mut(name) {
            task.health.state = TaskState::Stopping;
            let _ = task.cancel_tx.send(true);

            if let Some(handle) = task.handle.take() {
                handle.abort();
            }

            task.health.state = TaskState::Stopped;
            task.health.last_state_change = Instant::now();
            info!("Stopped task: {}", name);
            Ok(())
        } else {
            Err(NonosError::Internal(format!("Task '{}' not found", name)))
        }
    }

    /// Remove a task from supervision
    pub async fn remove_task(&self, name: &str) -> NonosResult<()> {
        self.stop_task(name).await?;
        self.tasks.write().remove(name);
        debug!("Removed task: {}", name);
        Ok(())
    }

    /// Get health status of a specific task
    pub fn task_health(&self, name: &str) -> Option<TaskHealth> {
        self.tasks.read().get(name).map(|t| {
            let mut health = t.health.clone();
            health.update_uptime();
            health
        })
    }

    /// Get health status of all tasks
    pub fn all_task_health(&self) -> HashMap<String, TaskHealth> {
        self.tasks
            .read()
            .iter()
            .map(|(name, task)| {
                let mut health = task.health.clone();
                health.update_uptime();
                (name.clone(), health)
            })
            .collect()
    }

    /// Check and restart failed tasks according to their policies
    pub async fn check_and_restart(&self) {
        let tasks_to_restart: Vec<(String, RestartPolicy, u32)> = {
            let tasks = self.tasks.read();
            tasks
                .iter()
                .filter_map(|(name, task)| {
                    if let Some(ref handle) = task.handle {
                        if handle.is_finished() && task.policy != RestartPolicy::Never {
                            return Some((
                                name.clone(),
                                task.policy,
                                task.health.restart_count,
                            ));
                        }
                    }
                    None
                })
                .collect()
        };

        for (name, policy, restart_count) in tasks_to_restart {
            if restart_count >= MAX_RESTART_ATTEMPTS && policy != RestartPolicy::Always {
                warn!(
                    "Task '{}' exceeded max restart attempts ({})",
                    name, MAX_RESTART_ATTEMPTS
                );
                continue;
            }

            if let Err(e) = self.restart_task(&name, policy, restart_count).await {
                error!("Failed to restart task '{}': {}", name, e);
            }
        }
    }

    async fn restart_task(
        &self,
        name: &str,
        policy: RestartPolicy,
        restart_count: u32,
    ) -> NonosResult<()> {
        // Check if restart rate is exceeded
        {
            let tasks = self.tasks.read();
            if let Some(task) = tasks.get(name) {
                if task.health.is_restart_rate_exceeded() {
                    warn!(
                        "Task '{}' restart rate exceeded ({} restarts in {} seconds) - marking as unhealthy",
                        name, task.health.restarts_in_window(), RESTART_RATE_WINDOW_SECS
                    );
                    drop(tasks);
                    let mut tasks_write = self.tasks.write();
                    if let Some(task) = tasks_write.get_mut(name) {
                        task.health.state = TaskState::Failed;
                        task.health.last_error = Some(format!(
                            "Restart rate exceeded: {} restarts in {} seconds",
                            task.health.restarts_in_window(),
                            RESTART_RATE_WINDOW_SECS
                        ));
                        task.health.update_classification();
                    }
                    return Err(NonosError::Internal(format!(
                        "Task '{}' restart rate exceeded",
                        name
                    )));
                }
            }
        }

        let backoff_ms = if policy == RestartPolicy::ExponentialBackoff {
            let delay = RESTART_BACKOFF_BASE_MS * 2u64.pow(restart_count);
            delay.min(RESTART_BACKOFF_MAX_MS)
        } else {
            RESTART_BACKOFF_BASE_MS
        };

        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(name) {
            if let Some(ref factory) = task.factory {
                let (cancel_tx, cancel_rx) = watch::channel(false);
                let handle = factory.create(cancel_rx);

                task.handle = Some(handle);
                task.cancel_tx = cancel_tx;
                task.health.record_restart(); // Uses new method
                task.health.state = TaskState::Running;
                task.health.last_state_change = Instant::now();
                task.health.record_sample(false);
                task.health.update_classification();

                self.total_restarts.fetch_add(1, Ordering::Relaxed);
                info!(
                    "Restarted task: {} (attempt {}, classification: {:?})",
                    name, task.health.restart_count, task.health.classification
                );
            }
        }

        Ok(())
    }

    /// Initiate graceful shutdown of all tasks
    pub async fn shutdown(&self, timeout: Duration) -> NonosResult<()> {
        info!("Initiating supervisor shutdown with {:?} timeout", timeout);
        self.shutdown.store(true, Ordering::SeqCst);

        let task_names: Vec<String> = self.tasks.read().keys().cloned().collect();

        for name in &task_names {
            if let Some(task) = self.tasks.write().get_mut(name) {
                let _ = task.cancel_tx.send(true);
                task.health.state = TaskState::Stopping;
            }
        }

        let deadline = Instant::now() + timeout;

        for name in &task_names {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                warn!("Shutdown timeout reached, aborting remaining tasks");
                break;
            }

            let handle = {
                let mut tasks = self.tasks.write();
                tasks.get_mut(name).and_then(|t| t.handle.take())
            };

            if let Some(handle) = handle {
                match tokio::time::timeout(remaining, handle).await {
                    Ok(result) => match result {
                        Ok(Ok(())) => {
                            debug!("Task '{}' shut down cleanly", name);
                        }
                        Ok(Err(e)) => {
                            warn!("Task '{}' exited with error: {}", name, e);
                        }
                        Err(e) => {
                            warn!("Task '{}' panicked: {}", name, e);
                        }
                    },
                    Err(_) => {
                        warn!("Task '{}' did not stop in time, aborting", name);
                    }
                }
            }

            if let Some(task) = self.tasks.write().get_mut(name) {
                task.health.state = TaskState::Stopped;
                task.health.last_state_change = Instant::now();
            }
        }

        info!("Supervisor shutdown complete");
        Ok(())
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get supervisor statistics
    pub fn stats(&self) -> SupervisorStats {
        let tasks = self.tasks.read();
        let running = tasks
            .values()
            .filter(|t| t.health.state == TaskState::Running)
            .count();
        let failed = tasks
            .values()
            .filter(|t| t.health.state == TaskState::Failed)
            .count();

        let healthy = tasks
            .values()
            .filter(|t| t.health.classification == HealthClassification::Healthy)
            .count();
        let degraded = tasks
            .values()
            .filter(|t| t.health.classification == HealthClassification::Degraded)
            .count();
        let critical = tasks
            .values()
            .filter(|t| t.health.classification == HealthClassification::Critical)
            .count();

        SupervisorStats {
            total_tasks: tasks.len(),
            running_tasks: running,
            failed_tasks: failed,
            total_spawned: self.total_spawned.load(Ordering::Relaxed),
            total_restarts: self.total_restarts.load(Ordering::Relaxed),
            uptime_secs: self.started_at.elapsed().as_secs(),
            healthy_tasks: healthy,
            degraded_tasks: degraded,
            critical_tasks: critical,
        }
    }

    /// Run the supervision loop (call this in a spawned task)
    pub async fn run_supervision_loop(&self, check_interval: Duration) {
        let mut interval = tokio::time::interval(check_interval);

        loop {
            interval.tick().await;

            if self.is_shutting_down() {
                break;
            }

            self.check_and_restart().await;

            {
                let mut tasks = self.tasks.write();
                for task in tasks.values_mut() {
                    if task.health.state == TaskState::Running {
                        if let Some(ref handle) = task.handle {
                            task.health.record_sample(!handle.is_finished());
                        }
                    }
                    task.health.update_uptime();
                    task.health.cleanup_old_restarts();
                    task.health.update_classification();
                    trace!("{}", task.summary());
                }
            }
        }

        debug!("Supervision loop ended");
    }

    /// Get tasks that are in critical health state
    pub fn critical_tasks(&self) -> Vec<String> {
        self.tasks
            .read()
            .iter()
            .filter(|(_, task)| task.health.classification == HealthClassification::Critical)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get task health metrics for a specific task
    pub fn task_metrics(&self, name: &str) -> Option<TaskMetrics> {
        self.tasks.read().get(name).map(|task| TaskMetrics {
            name: task.name.clone(),
            state: task.health.state,
            classification: task.health.classification,
            restart_count: task.health.restart_count,
            restarts_in_window: task.health.restarts_in_window(),
            health_score: task.health.health_score,
            uptime_secs: task.health.uptime_secs,
            last_error: task.health.last_error.clone(),
        })
    }

    /// Get metrics for all tasks
    pub fn all_task_metrics(&self) -> Vec<TaskMetrics> {
        self.tasks
            .read()
            .values()
            .map(|task| TaskMetrics {
                name: task.name.clone(),
                state: task.health.state,
                classification: task.health.classification,
                restart_count: task.health.restart_count,
                restarts_in_window: task.health.restarts_in_window(),
                health_score: task.health.health_score,
                uptime_secs: task.health.uptime_secs,
                last_error: task.health.last_error.clone(),
            })
            .collect()
    }
}

/// Task metrics for monitoring/API exposure
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

impl Default for TaskSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Supervisor statistics
#[derive(Clone, Debug)]
pub struct SupervisorStats {
    /// Total number of tasks
    pub total_tasks: usize,
    /// Number of running tasks
    pub running_tasks: usize,
    /// Number of failed tasks
    pub failed_tasks: usize,
    /// Total tasks ever spawned
    pub total_spawned: u64,
    /// Total restarts performed
    pub total_restarts: u64,
    /// Supervisor uptime in seconds
    pub uptime_secs: u64,
    /// Tasks by health classification
    pub healthy_tasks: usize,
    /// Degraded tasks
    pub degraded_tasks: usize,
    /// Critical tasks
    pub critical_tasks: usize,
}

/// Cancellation token for graceful shutdown
#[derive(Clone)]
pub struct CancellationToken {
    receiver: watch::Receiver<bool>,
}

impl CancellationToken {
    /// Create a new cancellation token pair (sender for owner, receiver for workers)
    pub fn new() -> (watch::Sender<bool>, Self) {
        let (tx, rx) = watch::channel(false);
        (tx, Self { receiver: rx })
    }

    /// Check if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        *self.receiver.borrow()
    }

    /// Wait for cancellation
    pub async fn cancelled(&mut self) {
        while !*self.receiver.borrow() {
            if self.receiver.changed().await.is_err() {
                break;
            }
        }
    }

    /// Get the receiver for use in select!
    pub fn receiver(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        let (_, rx) = watch::channel(false);
        Self { receiver: rx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supervisor_spawn_and_stop() {
        let supervisor = TaskSupervisor::new();

        supervisor
            .spawn("test_task", RestartPolicy::Never, |mut cancel_rx| async move {
                loop {
                    tokio::select! {
                        _ = cancel_rx.changed() => {
                            if *cancel_rx.borrow() {
                                break;
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                    }
                }
                Ok(())
            })
            .expect("Failed to spawn task");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let health = supervisor.task_health("test_task").expect("Task not found");
        assert_eq!(health.state, TaskState::Running);

        supervisor
            .stop_task("test_task")
            .await
            .expect("Failed to stop task");

        let health = supervisor.task_health("test_task").expect("Task not found");
        assert_eq!(health.state, TaskState::Stopped);
    }

    #[tokio::test]
    async fn test_supervisor_stats() {
        let supervisor = TaskSupervisor::new();

        supervisor
            .spawn_oneshot("oneshot", async { Ok(()) })
            .expect("Failed to spawn oneshot");

        tokio::time::sleep(Duration::from_millis(10)).await;

        let stats = supervisor.stats();
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.total_spawned, 1);
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let (tx, token) = CancellationToken::new();

        assert!(!token.is_cancelled());

        tx.send(true).expect("Failed to send cancellation");

        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let supervisor = TaskSupervisor::new();

        for i in 0..3 {
            supervisor
                .spawn(
                    &format!("task_{}", i),
                    RestartPolicy::Never,
                    |mut cancel_rx| async move {
                        loop {
                            tokio::select! {
                                _ = cancel_rx.changed() => {
                                    if *cancel_rx.borrow() {
                                        break;
                                    }
                                }
                                _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                            }
                        }
                        Ok(())
                    },
                )
                .expect("Failed to spawn task");
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        supervisor
            .shutdown(Duration::from_secs(5))
            .await
            .expect("Shutdown failed");

        let stats = supervisor.stats();
        assert_eq!(stats.running_tasks, 0);
    }
}

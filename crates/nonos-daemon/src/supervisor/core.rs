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

use super::types::*;
use super::stats::{SupervisorStats, TaskMetrics};

struct SupervisedTask {
    name: String,
    policy: RestartPolicy,
    handle: Option<JoinHandle<NonosResult<()>>>,
    health: TaskHealth,
    cancel_tx: watch::Sender<bool>,
    factory: Option<Box<dyn TaskFactory>>,
}

impl SupervisedTask {
    fn name(&self) -> &str {
        &self.name
    }

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

pub struct TaskSupervisor {
    tasks: Arc<RwLock<HashMap<String, SupervisedTask>>>,
    shutdown: Arc<AtomicBool>,
    total_spawned: AtomicU64,
    total_restarts: AtomicU64,
    started_at: Instant,
}

impl TaskSupervisor {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
            total_spawned: AtomicU64::new(0),
            total_restarts: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

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

        let health = TaskHealth {
            state: TaskState::Running,
            last_state_change: Instant::now(),
            ..Default::default()
        };

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

        let health = TaskHealth {
            state: TaskState::Running,
            last_state_change: Instant::now(),
            ..Default::default()
        };

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

    pub async fn remove_task(&self, name: &str) -> NonosResult<()> {
        self.stop_task(name).await?;
        self.tasks.write().remove(name);
        debug!("Removed task: {}", name);
        Ok(())
    }

    pub fn task_health(&self, name: &str) -> Option<TaskHealth> {
        self.tasks.read().get(name).map(|t| {
            let mut health = t.health.clone();
            health.update_uptime();
            health
        })
    }

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
                task.health.record_restart();
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

    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

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

    pub fn critical_tasks(&self) -> Vec<String> {
        self.tasks
            .read()
            .iter()
            .filter(|(_, task)| task.health.classification == HealthClassification::Critical)
            .map(|(name, _)| name.clone())
            .collect()
    }

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

impl Default for TaskSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

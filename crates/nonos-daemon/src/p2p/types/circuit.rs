use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    success_count: u32,
    success_threshold: u32,
    last_failure: Option<Instant>,
    reset_timeout: Duration,
    is_open: AtomicBool,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            success_threshold,
            last_failure: None,
            reset_timeout,
            is_open: AtomicBool::new(false),
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn should_allow(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        self.is_open.store(false, Ordering::Relaxed);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.is_open.store(false, Ordering::Relaxed);
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.is_open.store(true, Ordering::Relaxed);
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.is_open.store(true, Ordering::Relaxed);
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.is_open.store(false, Ordering::Relaxed);
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure = None;
    }
}

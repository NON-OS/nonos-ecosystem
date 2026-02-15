use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::collector::NodeMetricsCollector;

pub struct RequestTimer {
    start: Instant,
    collector: Arc<NodeMetricsCollector>,
    service: Option<String>,
    recorded: AtomicBool,
}

impl RequestTimer {
    pub fn new(collector: Arc<NodeMetricsCollector>) -> Self {
        Self {
            start: Instant::now(),
            collector,
            service: None,
            recorded: AtomicBool::new(false),
        }
    }

    pub fn for_service(collector: Arc<NodeMetricsCollector>, service: &str) -> Self {
        Self {
            start: Instant::now(),
            collector,
            service: Some(service.to_string()),
            recorded: AtomicBool::new(false),
        }
    }

    pub fn success(self) {
        if self.recorded.swap(true, Ordering::SeqCst) {
            return;
        }
        let duration = self.start.elapsed();
        self.collector.record_request(true, duration);
        if let Some(ref service) = self.service {
            self.collector.record_service_request(service, true, duration);
        }
    }

    pub fn failure(self) {
        if self.recorded.swap(true, Ordering::SeqCst) {
            return;
        }
        let duration = self.start.elapsed();
        self.collector.record_request(false, duration);
        if let Some(ref service) = self.service {
            self.collector.record_service_request(service, false, duration);
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        if !self.recorded.load(Ordering::SeqCst) {
            let duration = self.start.elapsed();
            self.collector.record_request(false, duration);
            if let Some(ref service) = self.service {
                self.collector.record_service_request(service, false, duration);
            }
        }
    }
}

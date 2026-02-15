use std::time::Duration;

pub struct BackoffStrategy {
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    current_delay: Duration,
    attempt: u32,
    max_attempts: Option<u32>,
    jitter: bool,
}

impl BackoffStrategy {
    pub fn exponential(base: Duration, max: Duration) -> Self {
        Self {
            base_delay: base,
            max_delay: max,
            multiplier: 2.0,
            current_delay: base,
            attempt: 0,
            max_attempts: None,
            jitter: true,
        }
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = Some(max);
        self
    }

    pub fn with_jitter(mut self, enabled: bool) -> Self {
        self.jitter = enabled;
        self
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max) = self.max_attempts {
            if self.attempt >= max {
                return None;
            }
        }

        self.attempt += 1;

        let delay = if self.attempt == 1 {
            self.base_delay
        } else {
            let multiplied = self.current_delay.as_secs_f64() * self.multiplier;
            let capped = multiplied.min(self.max_delay.as_secs_f64());
            Duration::from_secs_f64(capped)
        };

        self.current_delay = delay;

        let final_delay = if self.jitter {
            let jitter_factor = 0.5 + rand::random::<f64>() * 0.5;
            Duration::from_secs_f64(delay.as_secs_f64() * jitter_factor)
        } else {
            delay
        };

        Some(final_delay)
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
        self.current_delay = self.base_delay;
    }

    pub fn attempts(&self) -> u32 {
        self.attempt
    }

    pub fn is_exhausted(&self) -> bool {
        if let Some(max) = self.max_attempts {
            self.attempt >= max
        } else {
            false
        }
    }
}

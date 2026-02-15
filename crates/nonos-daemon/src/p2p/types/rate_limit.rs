use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    TooManyMessages,
    TooManyBytes,
    InvalidMessage,
    SpamDetected,
    OversizedMessage,
    MalformedMessage,
    DecodeError,
    UnexpectedMessageType,
}

pub struct RateLimiter {
    messages_per_sec: u32,
    bytes_per_sec: u64,
    message_tokens: f64,
    byte_tokens: f64,
    max_message_burst: u32,
    max_byte_burst: u64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(messages_per_sec: u32, bytes_per_sec: u64) -> Self {
        let max_message_burst = messages_per_sec.saturating_mul(2);
        let max_byte_burst = bytes_per_sec.saturating_mul(2);

        Self {
            messages_per_sec,
            bytes_per_sec,
            message_tokens: max_message_burst as f64,
            byte_tokens: max_byte_burst as f64,
            max_message_burst,
            max_byte_burst,
            last_update: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        self.message_tokens = (self.message_tokens + elapsed * self.messages_per_sec as f64)
            .min(self.max_message_burst as f64);

        self.byte_tokens = (self.byte_tokens + elapsed * self.bytes_per_sec as f64)
            .min(self.max_byte_burst as f64);
    }

    pub fn check_message(&mut self, bytes: u64) -> Result<(), RateLimitReason> {
        self.refill();

        if self.message_tokens < 1.0 {
            return Err(RateLimitReason::TooManyMessages);
        }

        if (self.byte_tokens as u64) < bytes {
            return Err(RateLimitReason::TooManyBytes);
        }

        self.message_tokens -= 1.0;
        self.byte_tokens -= bytes as f64;

        Ok(())
    }

    pub fn update_limits(&mut self, messages_per_sec: u32, bytes_per_sec: u64) {
        self.messages_per_sec = messages_per_sec;
        self.bytes_per_sec = bytes_per_sec;
        self.max_message_burst = messages_per_sec.saturating_mul(2);
        self.max_byte_burst = bytes_per_sec.saturating_mul(2);
    }

    pub fn available_messages(&self) -> u32 {
        self.message_tokens as u32
    }

    pub fn available_bytes(&self) -> u64 {
        self.byte_tokens as u64
    }
}

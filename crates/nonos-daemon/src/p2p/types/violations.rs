use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageViolation {
    OversizedMessage,
    DecodeFailure,
    MalformedContent,
    UnexpectedType,
    RateLimitExceeded,
    SpamBehavior,
    ProtocolViolation,
}

impl MessageViolation {
    pub fn penalty_score(&self) -> i32 {
        match self {
            Self::OversizedMessage => 15,
            Self::DecodeFailure => 10,
            Self::MalformedContent => 20,
            Self::UnexpectedType => 5,
            Self::RateLimitExceeded => 10,
            Self::SpamBehavior => 25,
            Self::ProtocolViolation => 30,
        }
    }

    pub fn ban_multiplier(&self) -> u32 {
        match self {
            Self::OversizedMessage => 2,
            Self::DecodeFailure => 1,
            Self::MalformedContent => 3,
            Self::UnexpectedType => 1,
            Self::RateLimitExceeded => 2,
            Self::SpamBehavior => 4,
            Self::ProtocolViolation => 5,
        }
    }

    pub fn immediate_ban(&self) -> bool {
        matches!(self, Self::ProtocolViolation | Self::SpamBehavior)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViolationCounts {
    pub oversized_messages: u32,
    pub decode_failures: u32,
    pub malformed_content: u32,
    pub unexpected_types: u32,
    pub rate_limit_exceeded: u32,
    pub spam_behavior: u32,
    pub protocol_violations: u32,
}

impl ViolationCounts {
    pub fn record(&mut self, violation: MessageViolation) {
        match violation {
            MessageViolation::OversizedMessage => self.oversized_messages += 1,
            MessageViolation::DecodeFailure => self.decode_failures += 1,
            MessageViolation::MalformedContent => self.malformed_content += 1,
            MessageViolation::UnexpectedType => self.unexpected_types += 1,
            MessageViolation::RateLimitExceeded => self.rate_limit_exceeded += 1,
            MessageViolation::SpamBehavior => self.spam_behavior += 1,
            MessageViolation::ProtocolViolation => self.protocol_violations += 1,
        }
    }

    pub fn total(&self) -> u32 {
        self.oversized_messages
            + self.decode_failures
            + self.malformed_content
            + self.unexpected_types
            + self.rate_limit_exceeded
            + self.spam_behavior
            + self.protocol_violations
    }

    pub fn is_repeat_offender(&self) -> bool {
        self.total() >= 5
    }
}

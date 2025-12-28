// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! API Module
//!
//! Provides HTTP API for node management and monitoring:
//! - Node info, health, and metrics endpoints
//! - Privacy service endpoints
//! - Staking and rewards endpoints
//! - Versioned API for nonos-dash integration
//! - Bearer token authentication (optional)
//! - Per-IP rate limiting

mod handlers;
mod middleware;
mod node_handlers;
mod privacy_handlers;
mod responses;
mod rewards_handlers;
mod server;
mod staking_handlers;

pub use middleware::{
    ApiAuthenticator, ApiContext, ApiRateLimiter, AuthResult, RateLimitResult, RequestHeaders,
};
pub use node_handlers::{
    ApiErrorResponse, ApiResponse, BuildInfo, NodeConfigSummaryResponse, NodeHealthResponse,
    NodeInfoResponse, NodeMetricsResponse, NodeNetworkResponse, NodePeersResponse,
    NodeRewardsResponse, NodeServicesResponse, P2pMetrics, PeerSummary, ServiceMetricsSummary,
    ServiceStatus, API_VERSION,
};
pub use responses::*;
pub use server::ApiServer;

#[cfg(test)]
mod tests;

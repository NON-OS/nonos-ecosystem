// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

mod manager;
mod health_beacon;
mod quality_oracle;
mod bootstrap;
mod cache;

pub use manager::{ServiceManager, ServiceConfig, ServiceType, ServiceState};
pub use health_beacon::{HealthBeacon, HealthStatus};
pub use quality_oracle::QualityOracle;
pub use bootstrap::{BootstrapService, BootstrapConfig};
pub use cache::{CacheService, CacheStats};

#[cfg(test)]
mod tests;

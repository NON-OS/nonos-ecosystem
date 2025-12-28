// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

#![allow(missing_docs)]
#![warn(clippy::all)]

pub mod node;
pub mod p2p;
pub mod metrics;
pub mod rewards;
pub mod config;
pub mod contracts;
pub mod storage;
pub mod services;
pub mod privacy;
pub mod api;
pub mod tokenomics;
pub mod supervisor;

pub use node::{Node, CheckResult, DiagnosticReport};
pub use p2p::{P2pNetwork, PeerInfo, NetworkStats, NetworkEvent, P2pMessage, topics};
pub use metrics::{NodeMetricsCollector, PrometheusExporter};
pub use rewards::RewardTracker;
pub use config::{NodeConfig, ServicesConfig, NetworkConfig, RewardsConfig, ApiConfig};
pub use contracts::{ContractClient, ContractConfig, EPOCH_DURATION_SECS, current_epoch};
pub use storage::{NodeStorage, StorageConfig};
pub use services::{ServiceManager, ServiceConfig, ServiceType, ServiceState};
pub use privacy::{
    // Core services
    PrivacyServiceManager, PrivacyStats, ZkIdentityService, CacheMixingService,
    TrackingBlockerService, StealthScannerService,
    // Advanced privacy features
    AdvancedPrivacyManager, AdvancedPrivacyStats, ZkSessionManager, ZkSessionProof,
    MixnetProcessor, MixnetRequest, PrivateContentRetrieval, CachedContent,
    PrivacyOracle, DomainPrivacyScore, CookieBehavior, StealthSession, StealthSessionManager,
    CredentialProver, CredentialType, ZkCredentialProof, FingerprintNormalizer,
    NormalizedRequest, DistributedCookieVault, SecretShare,
};
pub use api::ApiServer;
pub use supervisor::{
    TaskSupervisor, SupervisorStats, TaskState, TaskHealth, RestartPolicy, CancellationToken,
};
pub use tokenomics::{
    calculate_daily_emission, calculate_epoch_emission, calculate_yearly_emission,
    calculate_staker_reward, calculate_effective_stake, EmissionSchedule, RewardParams,
    TierBenefits, NetworkEmissionState,
};

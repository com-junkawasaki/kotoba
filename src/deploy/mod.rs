//! Kotoba Deploy モジュール
//!
//! このモジュールはDeno Deployと同等の機能をKotoba上で提供します。
//! Live Graph ModelとISO GQLプロトコルを使用して、
//! グローバル分散ネットワーク、自動スケーリング、GitHub連携を実現します。

pub mod config;
pub mod controller;
pub mod cli;
pub mod runtime;
pub mod hosting_server;
pub mod hosting_manager;
pub mod scaling;
pub mod network;
pub mod git_integration;
pub mod parser;

// 再エクスポート
pub use config::{
    DeployConfig, DeployMetadata, ApplicationConfig, RuntimeType, BuildConfig,
    StaticFilesConfig, CacheConfig, CorsConfig, ScalingConfig, ScalingPolicy,
    NetworkConfig, DomainConfig, SslConfig, CertType, RedirectRule,
    CdnConfig, CdnProvider, TlsConfig, HstsConfig, RegionConfig,
    GeographyConfig, DeploymentStatus, DeployScript, ScriptTrigger,
    DeployConfigBuilder,
};
pub use controller::{
    DeployController, DeploymentManager, DeploymentState, DeploymentRequest,
    RunningDeployment, ResourceUsage, DeploymentPriority, GqlDeploymentQuery,
    DeploymentQueryType, GqlDeploymentResponse, GqlDeploymentExtensions,
};
pub use cli::{
    DeployCli, DeployCommands, DeployCliImpl, run_cli,
};
pub use runtime::{
    DeployRuntime, RuntimeManager, RuntimeConfig, WasmInstance, ResourceUsage as RuntimeResourceUsage, run_hosting_server,
};
pub use hosting_server::{
    HostingServer, HostingManager as HostingManagerInner, HostedApp, HostingStats, run_hosting_server_system,
};
pub use hosting_manager::{
    HostingManager, DeploymentLifecycle, LifecyclePhase, DeploymentMetrics, SystemStats,
};
pub use scaling::{
    ScalingEngine, LoadBalancer, AutoScaler, InstanceInfo, InstanceStatus, LoadBalancingAlgorithm,
};
pub use network::{
    NetworkManager, RegionManager, EdgeRouter, NetworkConfig as NetworkConfigTrait,
};
pub use git_integration::{
    GitIntegration, WebhookHandler, GitHubConfig, GitHubEvent, DeploymentStatus,
};
pub use parser::{
    DeployConfigParser,
};

/// デプロイモジュールのバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// デプロイモジュールの初期化
pub fn init() -> kotoba_core::types::Result<()> {
    println!("Initializing Kotoba Deploy v{}", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}

//! Kotoba Deploy モジュール
//!
//! このモジュールはDeno Deployと同等の機能をKotoba上で提供します。
//! Live Graph ModelとISO GQLプロトコルを使用して、
//! グローバル分散ネットワーク、自動スケーリング、GitHub連携を実現します。

pub mod config;
pub mod controller;
pub mod cli;

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

/// デプロイモジュールのバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// デプロイモジュールの初期化
pub fn init() -> crate::types::Result<()> {
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

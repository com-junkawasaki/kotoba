//! # Kotoba Deploy
//!
//! Deployment and hosting system for Kotoba applications with global distribution and auto-scaling.
//!
//! This crate provides Deno Deploy-like functionality for Kotoba applications,
//! including global edge deployment, automatic scaling, Git integration, and more.

// Re-export the main module
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

/// Prelude module for convenient imports
pub mod prelude {
    // Re-export commonly used items
    pub use crate::config::*;
    pub use crate::controller::*;
    pub use crate::cli::*;
    pub use crate::runtime::*;
    pub use crate::hosting_server::*;
    pub use crate::hosting_manager::*;
    pub use crate::scaling::*;
    pub use crate::network::*;
    pub use crate::git_integration::*;
    pub use crate::parser::*;
}

// Re-exports for backward compatibility
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
    NetworkManager, RegionManager, EdgeRouter,
};
pub use git_integration::{
    GitIntegration, WebhookHandler, GitHubConfig, GitHubEvent,
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
    use std::collections::HashMap;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.chars().all(|c| c.is_ascii_digit() || c == '.'));
    }

    #[test]
    fn test_runtime_type_variants() {
        let runtime_types = vec![
            RuntimeType::JavaScript,
            RuntimeType::TypeScript,
            RuntimeType::WebAssembly,
            RuntimeType::Jsonnet,
            RuntimeType::GraphQL,
        ];

        for rt in runtime_types {
            // Just test that they can be created
            assert!(matches!(rt, RuntimeType::JavaScript | RuntimeType::TypeScript | RuntimeType::WebAssembly | RuntimeType::Jsonnet | RuntimeType::GraphQL));
        }
    }

    #[test]
    fn test_deployment_status_variants() {
        let statuses = vec![
            DeploymentStatus::Queued,
            DeploymentStatus::Building,
            DeploymentStatus::Deploying,
            DeploymentStatus::Running,
            DeploymentStatus::Failed,
            DeploymentStatus::Stopped,
            DeploymentStatus::Scaling,
        ];

        for status in statuses {
            assert!(matches!(status, DeploymentStatus::Queued | DeploymentStatus::Building | DeploymentStatus::Deploying | DeploymentStatus::Running | DeploymentStatus::Failed | DeploymentStatus::Stopped | DeploymentStatus::Scaling));
        }
    }

    #[test]
    fn test_scaling_policy_variants() {
        let policies = vec![
            ScalingPolicy::Manual,
            ScalingPolicy::CpuBased,
            ScalingPolicy::MemoryBased,
            ScalingPolicy::RequestBased,
            ScalingPolicy::ScheduleBased,
        ];

        for policy in policies {
            assert!(matches!(policy, ScalingPolicy::Manual | ScalingPolicy::CpuBased | ScalingPolicy::MemoryBased | ScalingPolicy::RequestBased | ScalingPolicy::ScheduleBased));
        }
    }

    #[test]
    fn test_cdn_provider_variants() {
        let providers = vec![
            CdnProvider::Cloudflare,
            CdnProvider::Akamai,
            CdnProvider::Fastly,
            CdnProvider::CloudFront,
            CdnProvider::Custom("custom-cdn".to_string()),
        ];

        for provider in providers {
            assert!(matches!(provider, CdnProvider::Cloudflare | CdnProvider::Akamai | CdnProvider::Fastly | CdnProvider::CloudFront | CdnProvider::Custom(_)));
        }
    }

    #[test]
    fn test_cert_type_variants() {
        let cert_types = vec![
            CertType::LetsEncrypt,
            CertType::Custom,
            CertType::SelfSigned,
        ];

        for cert_type in cert_types {
            assert!(matches!(cert_type, CertType::LetsEncrypt | CertType::Custom | CertType::SelfSigned));
        }
    }

    #[test]
    fn test_script_trigger_variants() {
        let triggers = vec![
            ScriptTrigger::PreBuild,
            ScriptTrigger::PostBuild,
            ScriptTrigger::PreDeploy,
            ScriptTrigger::PostDeploy,
            ScriptTrigger::PreStart,
            ScriptTrigger::PostStart,
        ];

        for trigger in triggers {
            assert!(matches!(trigger, ScriptTrigger::PreBuild | ScriptTrigger::PostBuild | ScriptTrigger::PreDeploy | ScriptTrigger::PostDeploy | ScriptTrigger::PreStart | ScriptTrigger::PostStart));
        }
    }

    #[test]
    fn test_deployment_priority_variants() {
        let priorities = vec![
            DeploymentPriority::Low,
            DeploymentPriority::Normal,
            DeploymentPriority::High,
            DeploymentPriority::Critical,
        ];

        for priority in priorities {
            assert!(matches!(priority, DeploymentPriority::Low | DeploymentPriority::Normal | DeploymentPriority::High | DeploymentPriority::Critical));
        }
    }

    #[test]
    fn test_load_balancing_algorithm_variants() {
        let algorithms = vec![
            LoadBalancingAlgorithm::RoundRobin,
            LoadBalancingAlgorithm::LeastConnections,
            LoadBalancingAlgorithm::LeastResponseTime,
            LoadBalancingAlgorithm::IpHash,
            LoadBalancingAlgorithm::WeightedRoundRobin,
        ];

        for algorithm in algorithms {
            assert!(matches!(algorithm, LoadBalancingAlgorithm::RoundRobin | LoadBalancingAlgorithm::LeastConnections | LoadBalancingAlgorithm::LeastResponseTime | LoadBalancingAlgorithm::IpHash | LoadBalancingAlgorithm::WeightedRoundRobin));
        }
    }

    #[test]
    fn test_deployment_state_variants() {
        let states = vec![
            DeploymentState::Pending,
            DeploymentState::Building,
            DeploymentState::Built,
            DeploymentState::Deploying,
            DeploymentState::Deployed,
            DeploymentState::Failed,
            DeploymentState::Stopped,
            DeploymentState::Scaling,
        ];

        for state in states {
            assert!(matches!(state, DeploymentState::Pending | DeploymentState::Building | DeploymentState::Built | DeploymentState::Deploying | DeploymentState::Deployed | DeploymentState::Failed | DeploymentState::Stopped | DeploymentState::Scaling));
        }
    }

    #[test]
    fn test_lifecycle_phase_variants() {
        let phases = vec![
            LifecyclePhase::Created,
            LifecyclePhase::Building,
            LifecyclePhase::Built,
            LifecyclePhase::Starting,
            LifecyclePhase::Running,
            LifecyclePhase::Stopping,
            LifecyclePhase::Stopped,
            LifecyclePhase::Failed,
            LifecyclePhase::Scaling,
        ];

        for phase in phases {
            assert!(matches!(phase, LifecyclePhase::Created | LifecyclePhase::Building | LifecyclePhase::Built | LifecyclePhase::Starting | LifecyclePhase::Running | LifecyclePhase::Stopping | LifecyclePhase::Stopped | LifecyclePhase::Failed | LifecyclePhase::Scaling));
        }
    }

    #[test]
    fn test_instance_status_variants() {
        let statuses = vec![
            InstanceStatus::Starting,
            InstanceStatus::Running,
            InstanceStatus::Stopping,
            InstanceStatus::Stopped,
            InstanceStatus::Failed,
            InstanceStatus::Unhealthy,
        ];

        for status in statuses {
            assert!(matches!(status, InstanceStatus::Starting | InstanceStatus::Running | InstanceStatus::Stopping | InstanceStatus::Stopped | InstanceStatus::Failed | InstanceStatus::Unhealthy));
        }
    }

    #[test]
    fn test_deploy_commands_variants() {
        let commands = vec![
            DeployCommands::Deploy,
            DeployCommands::List,
            DeployCommands::Status,
            DeployCommands::Logs,
            DeployCommands::Stop,
            DeployCommands::Restart,
            DeployCommands::Scale,
            DeployCommands::Delete,
            DeployCommands::Config,
        ];

        for command in commands {
            assert!(matches!(command, DeployCommands::Deploy | DeployCommands::List | DeployCommands::Status | DeployCommands::Logs | DeployCommands::Stop | DeployCommands::Restart | DeployCommands::Scale | DeployCommands::Delete | DeployCommands::Config));
        }
    }

    #[test]
    fn test_runtime_type_serialization() {
        // Test serialization of all variants
        let variants = vec![
            (RuntimeType::JavaScript, "JavaScript"),
            (RuntimeType::TypeScript, "TypeScript"),
            (RuntimeType::WebAssembly, "WebAssembly"),
            (RuntimeType::Jsonnet, "Jsonnet"),
            (RuntimeType::GraphQL, "GraphQL"),
        ];

        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert!(json.contains(expected));
        }
    }

    #[test]
    fn test_deployment_status_serialization() {
        let statuses = vec![
            (DeploymentStatus::Queued, "Queued"),
            (DeploymentStatus::Building, "Building"),
            (DeploymentStatus::Deploying, "Deploying"),
            (DeploymentStatus::Running, "Running"),
            (DeploymentStatus::Failed, "Failed"),
            (DeploymentStatus::Stopped, "Stopped"),
            (DeploymentStatus::Scaling, "Scaling"),
        ];

        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(json.contains(expected));
        }
    }

    #[test]
    fn test_scaling_policy_serialization() {
        let policies = vec![
            (ScalingPolicy::Manual, "Manual"),
            (ScalingPolicy::CpuBased, "CpuBased"),
            (ScalingPolicy::MemoryBased, "MemoryBased"),
            (ScalingPolicy::RequestBased, "RequestBased"),
            (ScalingPolicy::ScheduleBased, "ScheduleBased"),
        ];

        for (policy, expected) in policies {
            let json = serde_json::to_string(&policy).unwrap();
            assert!(json.contains(expected));
        }
    }

    #[test]
    fn test_cdn_provider_serialization() {
        let providers = vec![
            (CdnProvider::Cloudflare, "Cloudflare"),
            (CdnProvider::Akamai, "Akamai"),
            (CdnProvider::Fastly, "Fastly"),
            (CdnProvider::CloudFront, "CloudFront"),
        ];

        for (provider, expected) in providers {
            let json = serde_json::to_string(&provider).unwrap();
            assert!(json.contains(expected));
        }

        // Test custom provider
        let custom = CdnProvider::Custom("my-cdn".to_string());
        let json = serde_json::to_string(&custom).unwrap();
        assert!(json.contains("my-cdn"));
    }

    #[test]
    fn test_cert_type_serialization() {
        let cert_types = vec![
            (CertType::LetsEncrypt, "LetsEncrypt"),
            (CertType::Custom, "Custom"),
            (CertType::SelfSigned, "SelfSigned"),
        ];

        for (cert_type, expected) in cert_types {
            let json = serde_json::to_string(&cert_type).unwrap();
            assert!(json.contains(expected));
        }
    }

    #[test]
    fn test_deploy_metadata_creation() {
        let metadata = DeployMetadata {
            name: "my-app".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test application".to_string()),
            author: Some("Test Author".to_string()),
            repository: Some("https://github.com/test/repo".to_string()),
            homepage: Some("https://example.com".to_string()),
            license: Some("MIT".to_string()),
            tags: vec!["web".to_string(), "api".to_string()],
        };

        assert_eq!(metadata.name, "my-app");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("A test application".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.repository, Some("https://github.com/test/repo".to_string()));
        assert_eq!(metadata.homepage, Some("https://example.com".to_string()));
        assert_eq!(metadata.license, Some("MIT".to_string()));
        assert_eq!(metadata.tags, vec!["web".to_string(), "api".to_string()]);
    }

    #[test]
    fn test_application_config_creation() {
        let config = ApplicationConfig {
            runtime: RuntimeType::TypeScript,
            entry_point: "main.ts".to_string(),
            build_command: Some("npm run build".to_string()),
            install_command: Some("npm install".to_string()),
            environment: HashMap::from([
                ("NODE_ENV".to_string(), "production".to_string()),
                ("PORT".to_string(), "8080".to_string()),
            ]),
            secrets: vec!["API_KEY".to_string(), "DATABASE_URL".to_string()],
            dependencies: vec!["lodash".to_string(), "express".to_string()],
        };

        assert!(matches!(config.runtime, RuntimeType::TypeScript));
        assert_eq!(config.entry_point, "main.ts");
        assert_eq!(config.build_command, Some("npm run build".to_string()));
        assert_eq!(config.install_command, Some("npm install".to_string()));
        assert_eq!(config.environment.len(), 2);
        assert_eq!(config.secrets.len(), 2);
        assert_eq!(config.dependencies.len(), 2);
    }

    #[test]
    fn test_build_config_creation() {
        let config = BuildConfig {
            output_dir: "dist".to_string(),
            assets_dir: Some("public".to_string()),
            exclude_patterns: vec!["*.log".to_string(), "node_modules".to_string()],
            include_patterns: vec!["src/**/*".to_string(), "public/**/*".to_string()],
            minify: true,
            source_map: false,
            target: Some("es2020".to_string()),
        };

        assert_eq!(config.output_dir, "dist");
        assert_eq!(config.assets_dir, Some("public".to_string()));
        assert_eq!(config.exclude_patterns.len(), 2);
        assert_eq!(config.include_patterns.len(), 2);
        assert!(config.minify);
        assert!(!config.source_map);
        assert_eq!(config.target, Some("es2020".to_string()));
    }

    #[test]
    fn test_static_files_config_creation() {
        let config = StaticFilesConfig {
            root_dir: "public".to_string(),
            index_file: "index.html".to_string(),
            spa_fallback: true,
            cache_control: Some("max-age=31536000".to_string()),
            gzip: true,
            brotli: false,
        };

        assert_eq!(config.root_dir, "public");
        assert_eq!(config.index_file, "index.html");
        assert!(config.spa_fallback);
        assert_eq!(config.cache_control, Some("max-age=31536000".to_string()));
        assert!(config.gzip);
        assert!(!config.brotli);
    }

    #[test]
    fn test_scaling_config_creation() {
        let config = ScalingConfig {
            min_instances: 1,
            max_instances: 10,
            target_cpu_utilization: 0.7,
            target_memory_utilization: 0.8,
            cooldown_period_seconds: 300,
            scale_up_factor: 2.0,
            scale_down_factor: 0.5,
            policy: ScalingPolicy::CpuBased,
        };

        assert_eq!(config.min_instances, 1);
        assert_eq!(config.max_instances, 10);
        assert_eq!(config.target_cpu_utilization, 0.7);
        assert_eq!(config.target_memory_utilization, 0.8);
        assert_eq!(config.cooldown_period_seconds, 300);
        assert_eq!(config.scale_up_factor, 2.0);
        assert_eq!(config.scale_down_factor, 0.5);
        assert!(matches!(config.policy, ScalingPolicy::CpuBased));
    }

    #[test]
    fn test_network_config_creation() {
        let config = NetworkConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            timeout_seconds: 30,
            keep_alive: true,
            compression: true,
        };

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.keep_alive);
        assert!(config.compression);
    }

    #[test]
    fn test_domain_config_creation() {
        let config = DomainConfig {
            domain: "example.com".to_string(),
            subdomains: vec!["api".to_string(), "app".to_string()],
            redirect_rules: vec![
                RedirectRule {
                    from: "/old-path".to_string(),
                    to: "/new-path".to_string(),
                    status_code: 301,
                }
            ],
        };

        assert_eq!(config.domain, "example.com");
        assert_eq!(config.subdomains.len(), 2);
        assert_eq!(config.redirect_rules.len(), 1);
        assert_eq!(config.redirect_rules[0].from, "/old-path");
        assert_eq!(config.redirect_rules[0].to, "/new-path");
        assert_eq!(config.redirect_rules[0].status_code, 301);
    }

    #[test]
    fn test_ssl_config_creation() {
        let config = SslConfig {
            enabled: true,
            cert_type: CertType::LetsEncrypt,
            cert_path: Some("/etc/ssl/certs/example.crt".to_string()),
            key_path: Some("/etc/ssl/private/example.key".to_string()),
            hsts: HstsConfig {
                enabled: true,
                max_age_seconds: 31536000,
                include_subdomains: true,
                preload: false,
            },
        };

        assert!(config.enabled);
        assert!(matches!(config.cert_type, CertType::LetsEncrypt));
        assert_eq!(config.cert_path, Some("/etc/ssl/certs/example.crt".to_string()));
        assert_eq!(config.key_path, Some("/etc/ssl/private/example.key".to_string()));
        assert!(config.hsts.enabled);
        assert_eq!(config.hsts.max_age_seconds, 31536000);
        assert!(config.hsts.include_subdomains);
        assert!(!config.hsts.preload);
    }

    #[test]
    fn test_cdn_config_creation() {
        let config = CdnConfig {
            enabled: true,
            provider: CdnProvider::Cloudflare,
            regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
            cache_ttl_seconds: 3600,
            purge_on_deploy: true,
        };

        assert!(config.enabled);
        assert!(matches!(config.provider, CdnProvider::Cloudflare));
        assert_eq!(config.regions.len(), 2);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.purge_on_deploy);
    }

    #[test]
    fn test_region_config_creation() {
        let config = RegionConfig {
            name: "us-east-1".to_string(),
            provider: "aws".to_string(),
            location: "N. Virginia".to_string(),
            enabled: true,
            priority: 1,
        };

        assert_eq!(config.name, "us-east-1");
        assert_eq!(config.provider, "aws");
        assert_eq!(config.location, "N. Virginia");
        assert!(config.enabled);
        assert_eq!(config.priority, 1);
    }

    #[test]
    fn test_deploy_script_creation() {
        let script = DeployScript {
            name: "build".to_string(),
            command: "npm run build".to_string(),
            trigger: ScriptTrigger::PreBuild,
            timeout_seconds: Some(300),
            environment: HashMap::from([
                ("NODE_ENV".to_string(), "production".to_string()),
            ]),
        };

        assert_eq!(script.name, "build");
        assert_eq!(script.command, "npm run build");
        assert!(matches!(script.trigger, ScriptTrigger::PreBuild));
        assert_eq!(script.timeout_seconds, Some(300));
        assert_eq!(script.environment.len(), 1);
    }

    #[test]
    fn test_deployment_request_creation() {
        let request = DeploymentRequest {
            app_name: "my-app".to_string(),
            version: "1.0.0".to_string(),
            config: serde_json::json!({"runtime": "typescript"}),
            priority: DeploymentPriority::High,
            user_id: "user123".to_string(),
        };

        assert_eq!(request.app_name, "my-app");
        assert_eq!(request.version, "1.0.0");
        assert_eq!(request.priority, DeploymentPriority::High);
        assert_eq!(request.user_id, "user123");
    }

    #[test]
    fn test_running_deployment_creation() {
        let deployment = RunningDeployment {
            id: "deploy-123".to_string(),
            app_name: "my-app".to_string(),
            version: "1.0.0".to_string(),
            status: DeploymentStatus::Running,
            url: "https://my-app.example.com".to_string(),
            region: "us-east-1".to_string(),
            instances: 3,
            created_at: chrono::Utc::now(),
        };

        assert_eq!(deployment.id, "deploy-123");
        assert_eq!(deployment.app_name, "my-app");
        assert_eq!(deployment.version, "1.0.0");
        assert!(matches!(deployment.status, DeploymentStatus::Running));
        assert_eq!(deployment.url, "https://my-app.example.com");
        assert_eq!(deployment.region, "us-east-1");
        assert_eq!(deployment.instances, 3);
    }

    #[test]
    fn test_resource_usage_creation() {
        let usage = ResourceUsage {
            cpu_percent: 75.5,
            memory_mb: 512.0,
            network_in_bytes: 1024000,
            network_out_bytes: 2048000,
            requests_per_second: 150.0,
            response_time_ms: 45.0,
        };

        assert_eq!(usage.cpu_percent, 75.5);
        assert_eq!(usage.memory_mb, 512.0);
        assert_eq!(usage.network_in_bytes, 1024000);
        assert_eq!(usage.network_out_bytes, 2048000);
        assert_eq!(usage.requests_per_second, 150.0);
        assert_eq!(usage.response_time_ms, 45.0);
    }

    #[test]
    fn test_hosted_app_creation() {
        let app = HostedApp {
            id: "app-123".to_string(),
            name: "my-app".to_string(),
            version: "1.0.0".to_string(),
            runtime: RuntimeType::TypeScript,
            url: "https://my-app.example.com".to_string(),
            status: DeploymentStatus::Running,
            instances: 2,
            region: "us-east-1".to_string(),
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        };

        assert_eq!(app.id, "app-123");
        assert_eq!(app.name, "my-app");
        assert_eq!(app.version, "1.0.0");
        assert!(matches!(app.runtime, RuntimeType::TypeScript));
        assert_eq!(app.url, "https://my-app.example.com");
        assert!(matches!(app.status, DeploymentStatus::Running));
        assert_eq!(app.instances, 2);
        assert_eq!(app.region, "us-east-1");
    }

    #[test]
    fn test_hosting_stats_creation() {
        let stats = HostingStats {
            total_apps: 25,
            running_apps: 22,
            total_instances: 75,
            active_instances: 68,
            total_requests: 150000,
            avg_response_time_ms: 35.5,
            uptime_percentage: 99.9,
        };

        assert_eq!(stats.total_apps, 25);
        assert_eq!(stats.running_apps, 22);
        assert_eq!(stats.total_instances, 75);
        assert_eq!(stats.active_instances, 68);
        assert_eq!(stats.total_requests, 150000);
        assert_eq!(stats.avg_response_time_ms, 35.5);
        assert_eq!(stats.uptime_percentage, 99.9);
    }

    #[test]
    fn test_deployment_metrics_creation() {
        let metrics = DeploymentMetrics {
            deployment_time_seconds: 45.0,
            build_time_seconds: 120.0,
            startup_time_seconds: 5.0,
            memory_usage_mb: 256.0,
            cpu_usage_percent: 15.0,
        };

        assert_eq!(metrics.deployment_time_seconds, 45.0);
        assert_eq!(metrics.build_time_seconds, 120.0);
        assert_eq!(metrics.startup_time_seconds, 5.0);
        assert_eq!(metrics.memory_usage_mb, 256.0);
        assert_eq!(metrics.cpu_usage_percent, 15.0);
    }

    #[test]
    fn test_system_stats_creation() {
        let stats = SystemStats {
            total_memory_mb: 8192.0,
            used_memory_mb: 4096.0,
            total_cpu_cores: 8,
            used_cpu_percent: 65.0,
            disk_usage_gb: 500.0,
            network_in_mbps: 100.0,
            network_out_mbps: 50.0,
        };

        assert_eq!(stats.total_memory_mb, 8192.0);
        assert_eq!(stats.used_memory_mb, 4096.0);
        assert_eq!(stats.total_cpu_cores, 8);
        assert_eq!(stats.used_cpu_percent, 65.0);
        assert_eq!(stats.disk_usage_gb, 500.0);
        assert_eq!(stats.network_in_mbps, 100.0);
        assert_eq!(stats.network_out_mbps, 50.0);
    }

    #[test]
    fn test_instance_info_creation() {
        let info = InstanceInfo {
            id: "instance-123".to_string(),
            region: "us-east-1".to_string(),
            status: InstanceStatus::Running,
            cpu_usage: 75.0,
            memory_usage: 512.0,
            request_count: 1000,
            last_health_check: chrono::Utc::now(),
        };

        assert_eq!(info.id, "instance-123");
        assert_eq!(info.region, "us-east-1");
        assert!(matches!(info.status, InstanceStatus::Running));
        assert_eq!(info.cpu_usage, 75.0);
        assert_eq!(info.memory_usage, 512.0);
        assert_eq!(info.request_count, 1000);
    }

    #[test]
    fn test_github_event_creation() {
        let event = GitHubEvent {
            event_type: "push".to_string(),
            repository: "test/repo".to_string(),
            branch: "main".to_string(),
            commit_sha: "abc123".to_string(),
            author: "testuser".to_string(),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(event.event_type, "push");
        assert_eq!(event.repository, "test/repo");
        assert_eq!(event.branch, "main");
        assert_eq!(event.commit_sha, "abc123");
        assert_eq!(event.author, "testuser");
    }

    #[test]
    fn test_enum_debug_formatting() {
        let runtime = RuntimeType::JavaScript;
        let debug_str = format!("{:?}", runtime);
        assert!(debug_str.contains("JavaScript"));

        let status = DeploymentStatus::Running;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Running"));

        let policy = ScalingPolicy::CpuBased;
        let debug_str = format!("{:?}", policy);
        assert!(debug_str.contains("CpuBased"));
    }

    #[test]
    fn test_enum_clone() {
        let runtime = RuntimeType::WebAssembly;
        let cloned = runtime.clone();
        assert!(matches!(cloned, RuntimeType::WebAssembly));

        let status = DeploymentStatus::Failed;
        let cloned = status.clone();
        assert!(matches!(cloned, DeploymentStatus::Failed));

        let policy = ScalingPolicy::RequestBased;
        let cloned = policy.clone();
        assert!(matches!(cloned, ScalingPolicy::RequestBased));
    }

    #[test]
    fn test_config_with_edge_cases() {
        // Test with zero values
        let config = ScalingConfig {
            min_instances: 0,
            max_instances: 0,
            target_cpu_utilization: 0.0,
            target_memory_utilization: 0.0,
            cooldown_period_seconds: 0,
            scale_up_factor: 1.0,
            scale_down_factor: 1.0,
            policy: ScalingPolicy::Manual,
        };

        assert_eq!(config.min_instances, 0);
        assert_eq!(config.max_instances, 0);
        assert_eq!(config.target_cpu_utilization, 0.0);
        assert_eq!(config.cooldown_period_seconds, 0);

        // Test with maximum values
        let config = ScalingConfig {
            min_instances: usize::MAX,
            max_instances: usize::MAX,
            target_cpu_utilization: 1.0,
            target_memory_utilization: 1.0,
            cooldown_period_seconds: u64::MAX,
            scale_up_factor: f64::MAX,
            scale_down_factor: f64::MAX,
            policy: ScalingPolicy::ScheduleBased,
        };

        assert_eq!(config.min_instances, usize::MAX);
        assert_eq!(config.max_instances, usize::MAX);
        assert_eq!(config.target_cpu_utilization, 1.0);
        assert_eq!(config.cooldown_period_seconds, u64::MAX);
        assert_eq!(config.scale_up_factor, f64::MAX);
        assert_eq!(config.scale_down_factor, f64::MAX);
    }

    #[test]
    fn test_empty_configs() {
        let metadata = DeployMetadata {
            name: "".to_string(),
            version: "".to_string(),
            description: None,
            author: None,
            repository: None,
            homepage: None,
            license: None,
            tags: vec![],
        };

        assert!(metadata.name.is_empty());
        assert!(metadata.version.is_empty());
        assert!(metadata.tags.is_empty());

        let app_config = ApplicationConfig {
            runtime: RuntimeType::JavaScript,
            entry_point: "".to_string(),
            build_command: None,
            install_command: None,
            environment: HashMap::new(),
            secrets: vec![],
            dependencies: vec![],
        };

        assert!(app_config.entry_point.is_empty());
        assert!(app_config.environment.is_empty());
        assert!(app_config.secrets.is_empty());
        assert!(app_config.dependencies.is_empty());
    }

    #[test]
    fn test_complex_nested_configs() {
        let complex_config = DeployConfig {
            metadata: DeployMetadata {
                name: "complex-app".to_string(),
                version: "2.1.0".to_string(),
                description: Some("A complex deployment configuration".to_string()),
                author: Some("Complex Author".to_string()),
                repository: Some("https://github.com/complex/repo".to_string()),
                homepage: Some("https://complex-app.com".to_string()),
                license: Some("Apache-2.0".to_string()),
                tags: vec!["complex".to_string(), "advanced".to_string(), "production".to_string()],
            },
            application: ApplicationConfig {
                runtime: RuntimeType::TypeScript,
                entry_point: "src/main.ts".to_string(),
                build_command: Some("npm run build:prod".to_string()),
                install_command: Some("npm ci".to_string()),
                environment: HashMap::from([
                    ("NODE_ENV".to_string(), "production".to_string()),
                    ("LOG_LEVEL".to_string(), "info".to_string()),
                    ("DATABASE_URL".to_string(), "postgresql://...".to_string()),
                ]),
                secrets: vec!["API_KEY".to_string(), "JWT_SECRET".to_string(), "ENCRYPTION_KEY".to_string()],
                dependencies: vec!["typescript".to_string(), "express".to_string(), "prisma".to_string(), "redis".to_string()],
            },
            build: BuildConfig {
                output_dir: "dist/production".to_string(),
                assets_dir: Some("public/assets".to_string()),
                exclude_patterns: vec!["*.test.ts".to_string(), "*.spec.ts".to_string(), "src/dev/**".to_string()],
                include_patterns: vec!["src/**/*".to_string(), "public/**/*".to_string(), "package.json".to_string()],
                minify: true,
                source_map: false,
                target: Some("es2020".to_string()),
            },
            static_files: StaticFilesConfig {
                root_dir: "dist/static".to_string(),
                index_file: "index.html".to_string(),
                spa_fallback: true,
                cache_control: Some("public, max-age=31536000, immutable".to_string()),
                gzip: true,
                brotli: true,
            },
            scaling: ScalingConfig {
                min_instances: 3,
                max_instances: 50,
                target_cpu_utilization: 0.75,
                target_memory_utilization: 0.85,
                cooldown_period_seconds: 600,
                scale_up_factor: 1.5,
                scale_down_factor: 0.7,
                policy: ScalingPolicy::RequestBased,
            },
            network: NetworkConfig {
                port: 443,
                host: "0.0.0.0".to_string(),
                max_connections: 10000,
                timeout_seconds: 60,
                keep_alive: true,
                compression: true,
            },
            domain: DomainConfig {
                domain: "api.complex-app.com".to_string(),
                subdomains: vec!["v1".to_string(), "v2".to_string(), "staging".to_string()],
                redirect_rules: vec![
                    RedirectRule {
                        from: "/old-api".to_string(),
                        to: "/v2/api".to_string(),
                        status_code: 301,
                    },
                    RedirectRule {
                        from: "/deprecated".to_string(),
                        to: "/new-endpoint".to_string(),
                        status_code: 302,
                    },
                ],
            },
            ssl: SslConfig {
                enabled: true,
                cert_type: CertType::LetsEncrypt,
                cert_path: Some("/etc/ssl/certs/complex.crt".to_string()),
                key_path: Some("/etc/ssl/private/complex.key".to_string()),
                hsts: HstsConfig {
                    enabled: true,
                    max_age_seconds: 63072000,
                    include_subdomains: true,
                    preload: true,
                },
            },
            cdn: CdnConfig {
                enabled: true,
                provider: CdnProvider::Cloudflare,
                regions: vec!["us-east-1".to_string(), "eu-west-1".to_string(), "ap-southeast-1".to_string()],
                cache_ttl_seconds: 7200,
                purge_on_deploy: true,
            },
            regions: vec![
                RegionConfig {
                    name: "us-east-1".to_string(),
                    provider: "aws".to_string(),
                    location: "N. Virginia".to_string(),
                    enabled: true,
                    priority: 1,
                },
                RegionConfig {
                    name: "eu-west-1".to_string(),
                    provider: "aws".to_string(),
                    location: "Ireland".to_string(),
                    enabled: true,
                    priority: 2,
                },
            ],
            scripts: vec![
                DeployScript {
                    name: "lint".to_string(),
                    command: "npm run lint".to_string(),
                    trigger: ScriptTrigger::PreBuild,
                    timeout_seconds: Some(120),
                    environment: HashMap::from([("CI".to_string(), "true".to_string())]),
                },
                DeployScript {
                    name: "test".to_string(),
                    command: "npm run test:ci".to_string(),
                    trigger: ScriptTrigger::PreDeploy,
                    timeout_seconds: Some(300),
                    environment: HashMap::from([("NODE_ENV".to_string(), "test".to_string())]),
                },
            ],
        };

        // Verify the complex configuration
        assert_eq!(complex_config.metadata.name, "complex-app");
        assert_eq!(complex_config.metadata.version, "2.1.0");
        assert_eq!(complex_config.metadata.tags.len(), 3);
        assert!(matches!(complex_config.application.runtime, RuntimeType::TypeScript));
        assert_eq!(complex_config.application.environment.len(), 3);
        assert_eq!(complex_config.application.secrets.len(), 3);
        assert_eq!(complex_config.application.dependencies.len(), 4);
        assert_eq!(complex_config.build.exclude_patterns.len(), 3);
        assert_eq!(complex_config.build.include_patterns.len(), 3);
        assert_eq!(complex_config.scaling.min_instances, 3);
        assert_eq!(complex_config.scaling.max_instances, 50);
        assert_eq!(complex_config.network.port, 443);
        assert_eq!(complex_config.domain.subdomains.len(), 3);
        assert_eq!(complex_config.domain.redirect_rules.len(), 2);
        assert!(matches!(complex_config.ssl.cert_type, CertType::LetsEncrypt));
        assert_eq!(complex_config.ssl.hsts.max_age_seconds, 63072000);
        assert!(matches!(complex_config.cdn.provider, CdnProvider::Cloudflare));
        assert_eq!(complex_config.cdn.regions.len(), 3);
        assert_eq!(complex_config.regions.len(), 2);
        assert_eq!(complex_config.scripts.len(), 2);
    }

    #[test]
    fn test_prelude_exports() {
        // Test that prelude exports work correctly
        let _metadata: prelude::DeployMetadata = DeployMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            repository: None,
            homepage: None,
            license: None,
            tags: vec![],
        };

        let _config: prelude::DeployConfig = DeployConfig {
            metadata: DeployMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                repository: None,
                homepage: None,
                license: None,
                tags: vec![],
            },
            application: ApplicationConfig {
                runtime: RuntimeType::JavaScript,
                entry_point: "main.js".to_string(),
                build_command: None,
                install_command: None,
                environment: HashMap::new(),
                secrets: vec![],
                dependencies: vec![],
            },
            build: BuildConfig {
                output_dir: "dist".to_string(),
                assets_dir: None,
                exclude_patterns: vec![],
                include_patterns: vec![],
                minify: false,
                source_map: false,
                target: None,
            },
            static_files: StaticFilesConfig {
                root_dir: "public".to_string(),
                index_file: "index.html".to_string(),
                spa_fallback: false,
                cache_control: None,
                gzip: false,
                brotli: false,
            },
            scaling: ScalingConfig {
                min_instances: 1,
                max_instances: 1,
                target_cpu_utilization: 0.5,
                target_memory_utilization: 0.5,
                cooldown_period_seconds: 300,
                scale_up_factor: 1.0,
                scale_down_factor: 1.0,
                policy: ScalingPolicy::Manual,
            },
            network: NetworkConfig {
                port: 8080,
                host: "localhost".to_string(),
                max_connections: 100,
                timeout_seconds: 30,
                keep_alive: false,
                compression: false,
            },
            domain: DomainConfig {
                domain: "example.com".to_string(),
                subdomains: vec![],
                redirect_rules: vec![],
            },
            ssl: SslConfig {
                enabled: false,
                cert_type: CertType::SelfSigned,
                cert_path: None,
                key_path: None,
                hsts: HstsConfig {
                    enabled: false,
                    max_age_seconds: 0,
                    include_subdomains: false,
                    preload: false,
                },
            },
            cdn: CdnConfig {
                enabled: false,
                provider: CdnProvider::Cloudflare,
                regions: vec![],
                cache_ttl_seconds: 0,
                purge_on_deploy: false,
            },
            regions: vec![],
            scripts: vec![],
        };

        let _controller: prelude::DeployController = DeployController::default();
        let _runtime: prelude::DeployRuntime = DeployRuntime::default();
        let _scaling: prelude::ScalingEngine = ScalingEngine::default();
        let _network: prelude::NetworkManager = NetworkManager::default();
    }
}

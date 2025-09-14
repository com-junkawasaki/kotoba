//! Deploy Configuration for Kotoba deployment settings

use crate::{KotobaNetError, Result};
use kotoba_jsonnet::JsonnetValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    pub name: String,
    pub version: String,
    pub environment: DeploymentEnvironment,
    pub scaling: ScalingConfig,
    pub regions: Vec<RegionConfig>,
    pub networking: NetworkingConfig,
    pub resources: ResourceConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
}

/// Deployment environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentEnvironment {
    Development,
    Staging,
    Production,
    Custom(String),
}

/// Scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub min_instances: u32,
    pub max_instances: u32,
    pub target_cpu_utilization: f64,
    pub target_memory_utilization: f64,
    pub cooldown_period_seconds: u32,
    pub scaling_policies: Vec<ScalingPolicy>,
}

/// Scaling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub name: String,
    pub metric: String,
    pub target_value: f64,
    pub adjustment_type: ScalingAdjustmentType,
}

/// Scaling adjustment type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingAdjustmentType {
    ChangeInCapacity(i32),
    PercentChangeInCapacity(i32),
    SetToCapacity(u32),
}

/// Region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub name: String,
    pub provider: CloudProvider,
    pub zone: Option<String>,
    pub instance_type: String,
    pub spot_instances: bool,
    pub reserved_instances: bool,
}

/// Cloud provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
    DigitalOcean,
    Custom(String),
}

/// Networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    pub load_balancer: LoadBalancerConfig,
    pub cdn: Option<CdnConfig>,
    pub firewall_rules: Vec<FirewallRule>,
    pub dns_config: DnsConfig,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub enabled: bool,
    pub algorithm: LoadBalancingAlgorithm,
    pub health_check: HealthCheckConfig,
    pub ssl_config: Option<SslConfig>,
}

/// Load balancing algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    IPHash,
    WeightedRoundRobin,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub path: String,
    pub interval_seconds: u32,
    pub timeout_seconds: u32,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

/// SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub certificate_arn: Option<String>,
    pub certificate_file: Option<String>,
    pub key_file: Option<String>,
    pub redirect_http_to_https: bool,
}

/// CDN configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConfig {
    pub enabled: bool,
    pub provider: CdnProvider,
    pub origins: Vec<String>,
    pub cache_behaviors: Vec<CacheBehavior>,
}

/// CDN provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CdnProvider {
    CloudFront,
    CloudFlare,
    Fastly,
    Akamai,
}

/// Cache behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheBehavior {
    pub path_pattern: String,
    pub ttl_seconds: u32,
    pub compress: bool,
    pub forward_cookies: bool,
}

/// Firewall rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub name: String,
    pub source_ip: String,
    pub port_range: PortRange,
    pub protocol: NetworkProtocol,
    pub action: FirewallAction,
}

/// Port range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Network protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkProtocol {
    TCP,
    UDP,
    ICMP,
}

/// Firewall action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallAction {
    Allow,
    Deny,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub domain: String,
    pub records: Vec<DnsRecord>,
    pub provider: DnsProvider,
}

/// DNS record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub type_: DnsRecordType,
    pub value: String,
    pub ttl: u32,
}

/// DNS record type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsRecordType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    SRV,
}

/// DNS provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsProvider {
    Route53,
    CloudFlare,
    GoogleCloudDNS,
    AzureDNS,
}

/// Resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub cpu: CpuConfig,
    pub memory: MemoryConfig,
    pub storage: StorageConfig,
    pub gpu: Option<GpuConfig>,
}

/// CPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    pub cores: f64,
    pub architecture: CpuArchitecture,
}

/// CPU architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpuArchitecture {
    X86_64,
    ARM64,
    AMD64,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub size_gb: f64,
    pub type_: MemoryType,
}

/// Memory type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    DDR4,
    DDR5,
    GDDR6,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub size_gb: u32,
    pub type_: StorageType,
    pub iops: Option<u32>,
}

/// Storage type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    SSD,
    HDD,
    NVMe,
}

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub type_: String,
    pub count: u32,
    pub memory_gb: f64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics: Vec<String>,
    pub logs: LogConfig,
    pub alerts: Vec<AlertConfig>,
    pub dashboards: Vec<String>,
}

/// Log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub retention_days: u32,
    pub log_level: LogLevel,
    pub structured_logging: bool,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub name: String,
    pub metric: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub channels: Vec<String>,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption: EncryptionConfig,
    pub secrets: SecretsConfig,
    pub access_control: AccessControlConfig,
    pub compliance: Option<ComplianceConfig>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub at_rest: bool,
    pub in_transit: bool,
    pub key_management: KeyManagementType,
}

/// Key management type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyManagementType {
    AWSKMS,
    GCPKMS,
    AzureKeyVault,
    HashiCorpVault,
}

/// Secrets configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    pub provider: SecretsProvider,
    pub rotation_enabled: bool,
    pub rotation_days: u32,
}

/// Secrets provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretsProvider {
    AWS,
    GCP,
    Azure,
    HashiCorp,
    Doppler,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub enabled: bool,
    pub provider: AccessControlProvider,
    pub roles: Vec<String>,
    pub policies: Vec<String>,
}

/// Access control provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessControlProvider {
    AWSIAM,
    GCP,
    AzureRBAC,
    Keycloak,
}

/// Compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub standards: Vec<ComplianceStandard>,
    pub audit_enabled: bool,
    pub audit_retention_days: u32,
}

/// Compliance standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceStandard {
    SOC2,
    HIPAA,
    PCI_DSS,
    GDPR,
    ISO27001,
}

/// Deploy Parser for .kotoba-deploy files
#[derive(Debug)]
pub struct DeployParser;

impl DeployParser {
    /// Parse deployment configuration from Jsonnet
    pub fn parse(content: &str) -> Result<DeployConfig> {
        let evaluated = crate::evaluate_kotoba(content)?;
        Self::jsonnet_value_to_deploy_config(&evaluated)
    }

    /// Parse deploy config from file
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<DeployConfig> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Convert JsonnetValue to DeployConfig
    fn jsonnet_value_to_deploy_config(value: &JsonnetValue) -> Result<DeployConfig> {
        match value {
            JsonnetValue::Object(obj) => {
                let name = Self::extract_string(obj, "name")?;
                let version = Self::extract_string(obj, "version")?;
                let environment = Self::extract_environment(obj)?;
                let scaling = Self::extract_scaling(obj)?;
                let regions = Self::extract_regions(obj)?;
                let networking = Self::extract_networking(obj)?;
                let resources = Self::extract_resources(obj)?;
                let monitoring = Self::extract_monitoring(obj)?;
                let security = Self::extract_security(obj)?;

                Ok(DeployConfig {
                    name,
                    version,
                    environment,
                    scaling,
                    regions,
                    networking,
                    resources,
                    monitoring,
                    security,
                })
            }
            _ => Err(KotobaNetError::DeployConfig(
                "Deploy configuration must be an object".to_string(),
            )),
        }
    }

    /// Extract deployment environment
    fn extract_environment(obj: &HashMap<String, JsonnetValue>) -> Result<DeploymentEnvironment> {
        let env_str = Self::extract_string(obj, "environment")?;
        match env_str.to_lowercase().as_str() {
            "development" | "dev" => Ok(DeploymentEnvironment::Development),
            "staging" | "stage" => Ok(DeploymentEnvironment::Staging),
            "production" | "prod" => Ok(DeploymentEnvironment::Production),
            custom => Ok(DeploymentEnvironment::Custom(custom.to_string())),
        }
    }

    /// Extract scaling configuration
    fn extract_scaling(obj: &HashMap<String, JsonnetValue>) -> Result<ScalingConfig> {
        if let Some(JsonnetValue::Object(scale_obj)) = obj.get("scaling") {
            let min_instances = Self::extract_number(scale_obj, "minInstances")? as u32;
            let max_instances = Self::extract_number(scale_obj, "maxInstances")? as u32;
            let target_cpu_utilization = Self::extract_number(scale_obj, "targetCpuUtilization")?;
            let target_memory_utilization = Self::extract_number(scale_obj, "targetMemoryUtilization")?;
            let cooldown_period_seconds = Self::extract_number(scale_obj, "cooldownPeriodSeconds")? as u32;
            let scaling_policies = Self::extract_scaling_policies(scale_obj)?;

            Ok(ScalingConfig {
                min_instances,
                max_instances,
                target_cpu_utilization,
                target_memory_utilization,
                cooldown_period_seconds,
                scaling_policies,
            })
        } else {
            // Default scaling config
            Ok(ScalingConfig {
                min_instances: 1,
                max_instances: 10,
                target_cpu_utilization: 0.7,
                target_memory_utilization: 0.8,
                cooldown_period_seconds: 300,
                scaling_policies: Vec::new(),
            })
        }
    }

    /// Extract scaling policies
    fn extract_scaling_policies(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<ScalingPolicy>> {
        let mut policies = Vec::new();

        if let Some(JsonnetValue::Array(policy_array)) = obj.get("policies") {
            for policy_value in policy_array {
                if let JsonnetValue::Object(policy_obj) = policy_value {
                    let policy = Self::parse_scaling_policy(policy_obj)?;
                    policies.push(policy);
                }
            }
        }

        Ok(policies)
    }

    /// Parse scaling policy
    fn parse_scaling_policy(obj: &HashMap<String, JsonnetValue>) -> Result<ScalingPolicy> {
        let name = Self::extract_string(obj, "name")?;
        let metric = Self::extract_string(obj, "metric")?;
        let target_value = Self::extract_number(obj, "targetValue")?;
        let adjustment_type = Self::extract_adjustment_type(obj)?;

        Ok(ScalingPolicy {
            name,
            metric,
            target_value,
            adjustment_type,
        })
    }

    /// Extract adjustment type
    fn extract_adjustment_type(obj: &HashMap<String, JsonnetValue>) -> Result<ScalingAdjustmentType> {
        if let Some(JsonnetValue::Object(adj_obj)) = obj.get("adjustment") {
            let type_str = Self::extract_string(adj_obj, "type")?;
            match type_str.as_str() {
                "ChangeInCapacity" => {
                    let value = Self::extract_number(adj_obj, "value")? as i32;
                    Ok(ScalingAdjustmentType::ChangeInCapacity(value))
                }
                "PercentChangeInCapacity" => {
                    let value = Self::extract_number(adj_obj, "value")? as i32;
                    Ok(ScalingAdjustmentType::PercentChangeInCapacity(value))
                }
                "SetToCapacity" => {
                    let value = Self::extract_number(adj_obj, "value")? as u32;
                    Ok(ScalingAdjustmentType::SetToCapacity(value))
                }
                _ => Err(KotobaNetError::DeployConfig(format!("Invalid adjustment type: {}", type_str))),
            }
        } else {
            Ok(ScalingAdjustmentType::ChangeInCapacity(1))
        }
    }

    /// Extract regions configuration
    fn extract_regions(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<RegionConfig>> {
        let mut regions = Vec::new();

        if let Some(JsonnetValue::Array(region_array)) = obj.get("regions") {
            for region_value in region_array {
                if let JsonnetValue::Object(region_obj) = region_value {
                    let region = Self::parse_region(region_obj)?;
                    regions.push(region);
                }
            }
        }

        Ok(regions)
    }

    /// Parse region configuration
    fn parse_region(obj: &HashMap<String, JsonnetValue>) -> Result<RegionConfig> {
        let name = Self::extract_string(obj, "name")?;
        let provider = Self::extract_cloud_provider(obj)?;
        let zone = Self::extract_string(obj, "zone").ok();
        let instance_type = Self::extract_string(obj, "instanceType")?;
        let spot_instances = Self::extract_bool(obj, "spotInstances").unwrap_or(false);
        let reserved_instances = Self::extract_bool(obj, "reservedInstances").unwrap_or(false);

        Ok(RegionConfig {
            name,
            provider,
            zone,
            instance_type,
            spot_instances,
            reserved_instances,
        })
    }

    /// Extract cloud provider
    fn extract_cloud_provider(obj: &HashMap<String, JsonnetValue>) -> Result<CloudProvider> {
        let provider_str = Self::extract_string(obj, "provider")?;
        match provider_str.to_uppercase().as_str() {
            "AWS" => Ok(CloudProvider::AWS),
            "GCP" | "GOOGLE" => Ok(CloudProvider::GCP),
            "AZURE" => Ok(CloudProvider::Azure),
            "DIGITALOCEAN" | "DO" => Ok(CloudProvider::DigitalOcean),
            custom => Ok(CloudProvider::Custom(custom.to_string())),
        }
    }

    /// Extract networking configuration
    fn extract_networking(obj: &HashMap<String, JsonnetValue>) -> Result<NetworkingConfig> {
        if let Some(JsonnetValue::Object(net_obj)) = obj.get("networking") {
            let load_balancer = Self::extract_load_balancer(net_obj)?;
            let cdn = Self::extract_cdn(net_obj)?;
            let firewall_rules = Self::extract_firewall_rules(net_obj)?;
            let dns_config = Self::extract_dns_config(net_obj)?;

            Ok(NetworkingConfig {
                load_balancer,
                cdn,
                firewall_rules,
                dns_config,
            })
        } else {
            Ok(NetworkingConfig {
                load_balancer: LoadBalancerConfig {
                    enabled: true,
                    algorithm: LoadBalancingAlgorithm::RoundRobin,
                    health_check: HealthCheckConfig {
                        path: "/health".to_string(),
                        interval_seconds: 30,
                        timeout_seconds: 5,
                        healthy_threshold: 2,
                        unhealthy_threshold: 2,
                    },
                    ssl_config: None,
                },
                cdn: None,
                firewall_rules: Vec::new(),
                dns_config: DnsConfig {
                    domain: "example.com".to_string(),
                    records: Vec::new(),
                    provider: DnsProvider::Route53,
                },
            })
        }
    }

    /// Extract load balancer configuration
    fn extract_load_balancer(obj: &HashMap<String, JsonnetValue>) -> Result<LoadBalancerConfig> {
        if let Some(JsonnetValue::Object(lb_obj)) = obj.get("loadBalancer") {
            let enabled = Self::extract_bool(lb_obj, "enabled").unwrap_or(true);
            let algorithm = Self::extract_load_balancing_algorithm(lb_obj)?;
            let health_check = Self::extract_health_check(lb_obj)?;
            let ssl_config = Self::extract_ssl_config(lb_obj)?;

            Ok(LoadBalancerConfig {
                enabled,
                algorithm,
                health_check,
                ssl_config,
            })
        } else {
            Ok(LoadBalancerConfig {
                enabled: true,
                algorithm: LoadBalancingAlgorithm::RoundRobin,
                health_check: HealthCheckConfig {
                    path: "/health".to_string(),
                    interval_seconds: 30,
                    timeout_seconds: 5,
                    healthy_threshold: 2,
                    unhealthy_threshold: 2,
                },
                ssl_config: None,
            })
        }
    }

    /// Extract load balancing algorithm
    fn extract_load_balancing_algorithm(obj: &HashMap<String, JsonnetValue>) -> Result<LoadBalancingAlgorithm> {
        let alg_str = Self::extract_string(obj, "algorithm")
            .unwrap_or_else(|_| "RoundRobin".to_string());

        match alg_str.as_str() {
            "RoundRobin" => Ok(LoadBalancingAlgorithm::RoundRobin),
            "LeastConnections" => Ok(LoadBalancingAlgorithm::LeastConnections),
            "IPHash" => Ok(LoadBalancingAlgorithm::IPHash),
            "WeightedRoundRobin" => Ok(LoadBalancingAlgorithm::WeightedRoundRobin),
            _ => Ok(LoadBalancingAlgorithm::RoundRobin),
        }
    }

    /// Extract health check configuration
    fn extract_health_check(obj: &HashMap<String, JsonnetValue>) -> Result<HealthCheckConfig> {
        if let Some(JsonnetValue::Object(hc_obj)) = obj.get("healthCheck") {
            let path = Self::extract_string(hc_obj, "path").unwrap_or_else(|_| "/health".to_string());
            let interval_seconds = Self::extract_number(hc_obj, "intervalSeconds").unwrap_or(30.0) as u32;
            let timeout_seconds = Self::extract_number(hc_obj, "timeoutSeconds").unwrap_or(5.0) as u32;
            let healthy_threshold = Self::extract_number(hc_obj, "healthyThreshold").unwrap_or(2.0) as u32;
            let unhealthy_threshold = Self::extract_number(hc_obj, "unhealthyThreshold").unwrap_or(2.0) as u32;

            Ok(HealthCheckConfig {
                path,
                interval_seconds,
                timeout_seconds,
                healthy_threshold,
                unhealthy_threshold,
            })
        } else {
            Ok(HealthCheckConfig {
                path: "/health".to_string(),
                interval_seconds: 30,
                timeout_seconds: 5,
                healthy_threshold: 2,
                unhealthy_threshold: 2,
            })
        }
    }

    /// Extract SSL configuration
    fn extract_ssl_config(obj: &HashMap<String, JsonnetValue>) -> Result<Option<SslConfig>> {
        if let Some(JsonnetValue::Object(ssl_obj)) = obj.get("ssl") {
            let certificate_arn = Self::extract_string(ssl_obj, "certificateArn").ok();
            let certificate_file = Self::extract_string(ssl_obj, "certificateFile").ok();
            let key_file = Self::extract_string(ssl_obj, "keyFile").ok();
            let redirect_http_to_https = Self::extract_bool(ssl_obj, "redirectHttpToHttps").unwrap_or(true);

            Ok(Some(SslConfig {
                certificate_arn,
                certificate_file,
                key_file,
                redirect_http_to_https,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract CDN configuration
    fn extract_cdn(obj: &HashMap<String, JsonnetValue>) -> Result<Option<CdnConfig>> {
        if let Some(JsonnetValue::Object(cdn_obj)) = obj.get("cdn") {
            let enabled = Self::extract_bool(cdn_obj, "enabled").unwrap_or(true);
            let provider = Self::extract_cdn_provider(cdn_obj)?;
            let origins = Self::extract_string_array(cdn_obj, "origins")?;
            let cache_behaviors = Self::extract_cache_behaviors(cdn_obj)?;

            Ok(Some(CdnConfig {
                enabled,
                provider,
                origins,
                cache_behaviors,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract CDN provider
    fn extract_cdn_provider(obj: &HashMap<String, JsonnetValue>) -> Result<CdnProvider> {
        let provider_str = Self::extract_string(obj, "provider")
            .unwrap_or_else(|_| "CloudFront".to_string());

        match provider_str.as_str() {
            "CloudFront" => Ok(CdnProvider::CloudFront),
            "CloudFlare" => Ok(CdnProvider::CloudFlare),
            "Fastly" => Ok(CdnProvider::Fastly),
            "Akamai" => Ok(CdnProvider::Akamai),
            _ => Ok(CdnProvider::CloudFront),
        }
    }

    /// Extract cache behaviors
    fn extract_cache_behaviors(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<CacheBehavior>> {
        let mut behaviors = Vec::new();

        if let Some(JsonnetValue::Array(behavior_array)) = obj.get("cacheBehaviors") {
            for behavior_value in behavior_array {
                if let JsonnetValue::Object(behavior_obj) = behavior_value {
                    let behavior = Self::parse_cache_behavior(behavior_obj)?;
                    behaviors.push(behavior);
                }
            }
        }

        Ok(behaviors)
    }

    /// Parse cache behavior
    fn parse_cache_behavior(obj: &HashMap<String, JsonnetValue>) -> Result<CacheBehavior> {
        let path_pattern = Self::extract_string(obj, "pathPattern")?;
        let ttl_seconds = Self::extract_number(obj, "ttlSeconds").unwrap_or(3600.0) as u32;
        let compress = Self::extract_bool(obj, "compress").unwrap_or(true);
        let forward_cookies = Self::extract_bool(obj, "forwardCookies").unwrap_or(false);

        Ok(CacheBehavior {
            path_pattern,
            ttl_seconds,
            compress,
            forward_cookies,
        })
    }

    /// Extract firewall rules
    fn extract_firewall_rules(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<FirewallRule>> {
        let mut rules = Vec::new();

        if let Some(JsonnetValue::Array(rule_array)) = obj.get("firewallRules") {
            for rule_value in rule_array {
                if let JsonnetValue::Object(rule_obj) = rule_value {
                    let rule = Self::parse_firewall_rule(rule_obj)?;
                    rules.push(rule);
                }
            }
        }

        Ok(rules)
    }

    /// Parse firewall rule
    fn parse_firewall_rule(obj: &HashMap<String, JsonnetValue>) -> Result<FirewallRule> {
        let name = Self::extract_string(obj, "name")?;
        let source_ip = Self::extract_string(obj, "sourceIp")?;
        let port_range = Self::extract_port_range(obj)?;
        let protocol = Self::extract_network_protocol(obj)?;
        let action = Self::extract_firewall_action(obj)?;

        Ok(FirewallRule {
            name,
            source_ip,
            port_range,
            protocol,
            action,
        })
    }

    /// Extract port range
    fn extract_port_range(obj: &HashMap<String, JsonnetValue>) -> Result<PortRange> {
        if let Some(JsonnetValue::Object(port_obj)) = obj.get("portRange") {
            let start = Self::extract_number(port_obj, "start").unwrap_or(80.0) as u16;
            let end = Self::extract_number(port_obj, "end").unwrap_or(80.0) as u16;

            Ok(PortRange { start, end })
        } else {
            Ok(PortRange { start: 80, end: 80 })
        }
    }

    /// Extract network protocol
    fn extract_network_protocol(obj: &HashMap<String, JsonnetValue>) -> Result<NetworkProtocol> {
        let protocol_str = Self::extract_string(obj, "protocol")
            .unwrap_or_else(|_| "TCP".to_string());

        match protocol_str.to_uppercase().as_str() {
            "TCP" => Ok(NetworkProtocol::TCP),
            "UDP" => Ok(NetworkProtocol::UDP),
            "ICMP" => Ok(NetworkProtocol::ICMP),
            _ => Ok(NetworkProtocol::TCP),
        }
    }

    /// Extract firewall action
    fn extract_firewall_action(obj: &HashMap<String, JsonnetValue>) -> Result<FirewallAction> {
        let action_str = Self::extract_string(obj, "action")
            .unwrap_or_else(|_| "Allow".to_string());

        match action_str.as_str() {
            "Allow" => Ok(FirewallAction::Allow),
            "Deny" => Ok(FirewallAction::Deny),
            _ => Ok(FirewallAction::Allow),
        }
    }

    /// Extract DNS configuration
    fn extract_dns_config(obj: &HashMap<String, JsonnetValue>) -> Result<DnsConfig> {
        if let Some(JsonnetValue::Object(dns_obj)) = obj.get("dns") {
            let domain = Self::extract_string(dns_obj, "domain")?;
            let records = Self::extract_dns_records(dns_obj)?;
            let provider = Self::extract_dns_provider(dns_obj)?;

            Ok(DnsConfig {
                domain,
                records,
                provider,
            })
        } else {
            Ok(DnsConfig {
                domain: "example.com".to_string(),
                records: Vec::new(),
                provider: DnsProvider::Route53,
            })
        }
    }

    /// Extract DNS records
    fn extract_dns_records(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<DnsRecord>> {
        let mut records = Vec::new();

        if let Some(JsonnetValue::Array(record_array)) = obj.get("records") {
            for record_value in record_array {
                if let JsonnetValue::Object(record_obj) = record_value {
                    let record = Self::parse_dns_record(record_obj)?;
                    records.push(record);
                }
            }
        }

        Ok(records)
    }

    /// Parse DNS record
    fn parse_dns_record(obj: &HashMap<String, JsonnetValue>) -> Result<DnsRecord> {
        let name = Self::extract_string(obj, "name")?;
        let type_str = Self::extract_string(obj, "type")?;
        let type_ = Self::parse_dns_record_type(&type_str)?;
        let value = Self::extract_string(obj, "value")?;
        let ttl = Self::extract_number(obj, "ttl").unwrap_or(300.0) as u32;

        Ok(DnsRecord {
            name,
            type_,
            value,
            ttl,
        })
    }

    /// Parse DNS record type
    fn parse_dns_record_type(type_str: &str) -> Result<DnsRecordType> {
        match type_str.to_uppercase().as_str() {
            "A" => Ok(DnsRecordType::A),
            "AAAA" => Ok(DnsRecordType::AAAA),
            "CNAME" => Ok(DnsRecordType::CNAME),
            "MX" => Ok(DnsRecordType::MX),
            "TXT" => Ok(DnsRecordType::TXT),
            "SRV" => Ok(DnsRecordType::SRV),
            _ => Err(KotobaNetError::DeployConfig(format!("Invalid DNS record type: {}", type_str))),
        }
    }

    /// Extract DNS provider
    fn extract_dns_provider(obj: &HashMap<String, JsonnetValue>) -> Result<DnsProvider> {
        let provider_str = Self::extract_string(obj, "provider")
            .unwrap_or_else(|_| "Route53".to_string());

        match provider_str.as_str() {
            "Route53" => Ok(DnsProvider::Route53),
            "CloudFlare" => Ok(DnsProvider::CloudFlare),
            "GoogleCloudDNS" => Ok(DnsProvider::GoogleCloudDNS),
            "AzureDNS" => Ok(DnsProvider::AzureDNS),
            _ => Ok(DnsProvider::Route53),
        }
    }

    /// Extract resource configuration
    fn extract_resources(obj: &HashMap<String, JsonnetValue>) -> Result<ResourceConfig> {
        if let Some(JsonnetValue::Object(res_obj)) = obj.get("resources") {
            let cpu = Self::extract_cpu_config(res_obj)?;
            let memory = Self::extract_memory_config(res_obj)?;
            let storage = Self::extract_storage_config(res_obj)?;
            let gpu = Self::extract_gpu_config(res_obj)?;

            Ok(ResourceConfig {
                cpu,
                memory,
                storage,
                gpu,
            })
        } else {
            Ok(ResourceConfig {
                cpu: CpuConfig {
                    cores: 2.0,
                    architecture: CpuArchitecture::X86_64,
                },
                memory: MemoryConfig {
                    size_gb: 4.0,
                    type_: MemoryType::DDR4,
                },
                storage: StorageConfig {
                    size_gb: 20,
                    type_: StorageType::SSD,
                    iops: None,
                },
                gpu: None,
            })
        }
    }

    /// Extract CPU configuration
    fn extract_cpu_config(obj: &HashMap<String, JsonnetValue>) -> Result<CpuConfig> {
        if let Some(JsonnetValue::Object(cpu_obj)) = obj.get("cpu") {
            let cores = Self::extract_number(cpu_obj, "cores").unwrap_or(2.0);
            let architecture = Self::extract_cpu_architecture(cpu_obj)?;

            Ok(CpuConfig {
                cores,
                architecture,
            })
        } else {
            Ok(CpuConfig {
                cores: 2.0,
                architecture: CpuArchitecture::X86_64,
            })
        }
    }

    /// Extract CPU architecture
    fn extract_cpu_architecture(obj: &HashMap<String, JsonnetValue>) -> Result<CpuArchitecture> {
        let arch_str = Self::extract_string(obj, "architecture")
            .unwrap_or_else(|_| "x86_64".to_string());

        match arch_str.to_lowercase().as_str() {
            "x86_64" => Ok(CpuArchitecture::X86_64),
            "arm64" => Ok(CpuArchitecture::ARM64),
            "amd64" => Ok(CpuArchitecture::AMD64),
            _ => Ok(CpuArchitecture::X86_64),
        }
    }

    /// Extract memory configuration
    fn extract_memory_config(obj: &HashMap<String, JsonnetValue>) -> Result<MemoryConfig> {
        if let Some(JsonnetValue::Object(mem_obj)) = obj.get("memory") {
            let size_gb = Self::extract_number(mem_obj, "sizeGb").unwrap_or(4.0);
            let type_ = Self::extract_memory_type(mem_obj)?;

            Ok(MemoryConfig {
                size_gb,
                type_,
            })
        } else {
            Ok(MemoryConfig {
                size_gb: 4.0,
                type_: MemoryType::DDR4,
            })
        }
    }

    /// Extract memory type
    fn extract_memory_type(obj: &HashMap<String, JsonnetValue>) -> Result<MemoryType> {
        let type_str = Self::extract_string(obj, "type")
            .unwrap_or_else(|_| "DDR4".to_string());

        match type_str.as_str() {
            "DDR4" => Ok(MemoryType::DDR4),
            "DDR5" => Ok(MemoryType::DDR5),
            "GDDR6" => Ok(MemoryType::GDDR6),
            _ => Ok(MemoryType::DDR4),
        }
    }

    /// Extract storage configuration
    fn extract_storage_config(obj: &HashMap<String, JsonnetValue>) -> Result<StorageConfig> {
        if let Some(JsonnetValue::Object(storage_obj)) = obj.get("storage") {
            let size_gb = Self::extract_number(storage_obj, "sizeGb").unwrap_or(20.0) as u32;
            let type_ = Self::extract_storage_type(storage_obj)?;
            let iops = Self::extract_number(storage_obj, "iops").map(|n| n as u32).ok();

            Ok(StorageConfig {
                size_gb,
                type_,
                iops,
            })
        } else {
            Ok(StorageConfig {
                size_gb: 20,
                type_: StorageType::SSD,
                iops: None,
            })
        }
    }

    /// Extract storage type
    fn extract_storage_type(obj: &HashMap<String, JsonnetValue>) -> Result<StorageType> {
        let type_str = Self::extract_string(obj, "type")
            .unwrap_or_else(|_| "SSD".to_string());

        match type_str.as_str() {
            "SSD" => Ok(StorageType::SSD),
            "HDD" => Ok(StorageType::HDD),
            "NVMe" => Ok(StorageType::NVMe),
            _ => Ok(StorageType::SSD),
        }
    }

    /// Extract GPU configuration
    fn extract_gpu_config(obj: &HashMap<String, JsonnetValue>) -> Result<Option<GpuConfig>> {
        if let Some(JsonnetValue::Object(gpu_obj)) = obj.get("gpu") {
            let type_ = Self::extract_string(gpu_obj, "type")?;
            let count = Self::extract_number(gpu_obj, "count").unwrap_or(1.0) as u32;
            let memory_gb = Self::extract_number(gpu_obj, "memoryGb").unwrap_or(8.0);

            Ok(Some(GpuConfig {
                type_,
                count,
                memory_gb,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract monitoring configuration
    fn extract_monitoring(obj: &HashMap<String, JsonnetValue>) -> Result<MonitoringConfig> {
        if let Some(JsonnetValue::Object(mon_obj)) = obj.get("monitoring") {
            let enabled = Self::extract_bool(mon_obj, "enabled").unwrap_or(true);
            let metrics = Self::extract_string_array(mon_obj, "metrics")?;
            let logs = Self::extract_log_config(mon_obj)?;
            let alerts = Self::extract_alerts(mon_obj)?;
            let dashboards = Self::extract_string_array(mon_obj, "dashboards")?;

            Ok(MonitoringConfig {
                enabled,
                metrics,
                logs,
                alerts,
                dashboards,
            })
        } else {
            Ok(MonitoringConfig {
                enabled: true,
                metrics: vec!["cpu".to_string(), "memory".to_string()],
                logs: LogConfig {
                    retention_days: 30,
                    log_level: LogLevel::INFO,
                    structured_logging: true,
                },
                alerts: Vec::new(),
                dashboards: Vec::new(),
            })
        }
    }

    /// Extract log configuration
    fn extract_log_config(obj: &HashMap<String, JsonnetValue>) -> Result<LogConfig> {
        if let Some(JsonnetValue::Object(log_obj)) = obj.get("logs") {
            let retention_days = Self::extract_number(log_obj, "retentionDays").unwrap_or(30.0) as u32;
            let log_level = Self::extract_log_level(log_obj)?;
            let structured_logging = Self::extract_bool(log_obj, "structuredLogging").unwrap_or(true);

            Ok(LogConfig {
                retention_days,
                log_level,
                structured_logging,
            })
        } else {
            Ok(LogConfig {
                retention_days: 30,
                log_level: LogLevel::INFO,
                structured_logging: true,
            })
        }
    }

    /// Extract log level
    fn extract_log_level(obj: &HashMap<String, JsonnetValue>) -> Result<LogLevel> {
        let level_str = Self::extract_string(obj, "level")
            .unwrap_or_else(|_| "INFO".to_string());

        match level_str.to_uppercase().as_str() {
            "DEBUG" => Ok(LogLevel::DEBUG),
            "INFO" => Ok(LogLevel::INFO),
            "WARN" => Ok(LogLevel::WARN),
            "ERROR" => Ok(LogLevel::ERROR),
            _ => Ok(LogLevel::INFO),
        }
    }

    /// Extract alerts
    fn extract_alerts(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<AlertConfig>> {
        let mut alerts = Vec::new();

        if let Some(JsonnetValue::Array(alert_array)) = obj.get("alerts") {
            for alert_value in alert_array {
                if let JsonnetValue::Object(alert_obj) = alert_value {
                    let alert = Self::parse_alert(alert_obj)?;
                    alerts.push(alert);
                }
            }
        }

        Ok(alerts)
    }

    /// Parse alert configuration
    fn parse_alert(obj: &HashMap<String, JsonnetValue>) -> Result<AlertConfig> {
        let name = Self::extract_string(obj, "name")?;
        let metric = Self::extract_string(obj, "metric")?;
        let condition = Self::extract_alert_condition(obj)?;
        let threshold = Self::extract_number(obj, "threshold")?;
        let channels = Self::extract_string_array(obj, "channels")?;

        Ok(AlertConfig {
            name,
            metric,
            condition,
            threshold,
            channels,
        })
    }

    /// Extract alert condition
    fn extract_alert_condition(obj: &HashMap<String, JsonnetValue>) -> Result<AlertCondition> {
        let condition_str = Self::extract_string(obj, "condition")
            .unwrap_or_else(|_| "GreaterThan".to_string());

        match condition_str.as_str() {
            "GreaterThan" => Ok(AlertCondition::GreaterThan),
            "LessThan" => Ok(AlertCondition::LessThan),
            "Equal" => Ok(AlertCondition::Equal),
            "NotEqual" => Ok(AlertCondition::NotEqual),
            _ => Ok(AlertCondition::GreaterThan),
        }
    }

    /// Extract security configuration
    fn extract_security(obj: &HashMap<String, JsonnetValue>) -> Result<SecurityConfig> {
        if let Some(JsonnetValue::Object(sec_obj)) = obj.get("security") {
            let encryption = Self::extract_encryption(sec_obj)?;
            let secrets = Self::extract_secrets(sec_obj)?;
            let access_control = Self::extract_access_control(sec_obj)?;
            let compliance = Self::extract_compliance(sec_obj)?;

            Ok(SecurityConfig {
                encryption,
                secrets,
                access_control,
                compliance,
            })
        } else {
            Ok(SecurityConfig {
                encryption: EncryptionConfig {
                    at_rest: true,
                    in_transit: true,
                    key_management: KeyManagementType::AWSKMS,
                },
                secrets: SecretsConfig {
                    provider: SecretsProvider::AWS,
                    rotation_enabled: true,
                    rotation_days: 90,
                },
                access_control: AccessControlConfig {
                    enabled: true,
                    provider: AccessControlProvider::AWSIAM,
                    roles: Vec::new(),
                    policies: Vec::new(),
                },
                compliance: None,
            })
        }
    }

    /// Extract encryption configuration
    fn extract_encryption(obj: &HashMap<String, JsonnetValue>) -> Result<EncryptionConfig> {
        if let Some(JsonnetValue::Object(enc_obj)) = obj.get("encryption") {
            let at_rest = Self::extract_bool(enc_obj, "atRest").unwrap_or(true);
            let in_transit = Self::extract_bool(enc_obj, "inTransit").unwrap_or(true);
            let key_management = Self::extract_key_management(enc_obj)?;

            Ok(EncryptionConfig {
                at_rest,
                in_transit,
                key_management,
            })
        } else {
            Ok(EncryptionConfig {
                at_rest: true,
                in_transit: true,
                key_management: KeyManagementType::AWSKMS,
            })
        }
    }

    /// Extract key management type
    fn extract_key_management(obj: &HashMap<String, JsonnetValue>) -> Result<KeyManagementType> {
        let km_str = Self::extract_string(obj, "keyManagement")
            .unwrap_or_else(|_| "AWSKMS".to_string());

        match km_str.as_str() {
            "AWSKMS" => Ok(KeyManagementType::AWSKMS),
            "GCPKMS" => Ok(KeyManagementType::GCPKMS),
            "AzureKeyVault" => Ok(KeyManagementType::AzureKeyVault),
            "HashiCorpVault" => Ok(KeyManagementType::HashiCorpVault),
            _ => Ok(KeyManagementType::AWSKMS),
        }
    }

    /// Extract secrets configuration
    fn extract_secrets(obj: &HashMap<String, JsonnetValue>) -> Result<SecretsConfig> {
        if let Some(JsonnetValue::Object(sec_obj)) = obj.get("secrets") {
            let provider = Self::extract_secrets_provider(sec_obj)?;
            let rotation_enabled = Self::extract_bool(sec_obj, "rotationEnabled").unwrap_or(true);
            let rotation_days = Self::extract_number(sec_obj, "rotationDays").unwrap_or(90.0) as u32;

            Ok(SecretsConfig {
                provider,
                rotation_enabled,
                rotation_days,
            })
        } else {
            Ok(SecretsConfig {
                provider: SecretsProvider::AWS,
                rotation_enabled: true,
                rotation_days: 90,
            })
        }
    }

    /// Extract secrets provider
    fn extract_secrets_provider(obj: &HashMap<String, JsonnetValue>) -> Result<SecretsProvider> {
        let provider_str = Self::extract_string(obj, "provider")
            .unwrap_or_else(|_| "AWS".to_string());

        match provider_str.as_str() {
            "AWS" => Ok(SecretsProvider::AWS),
            "GCP" => Ok(SecretsProvider::GCP),
            "Azure" => Ok(SecretsProvider::Azure),
            "HashiCorp" => Ok(SecretsProvider::HashiCorp),
            "Doppler" => Ok(SecretsProvider::Doppler),
            _ => Ok(SecretsProvider::AWS),
        }
    }

    /// Extract access control configuration
    fn extract_access_control(obj: &HashMap<String, JsonnetValue>) -> Result<AccessControlConfig> {
        if let Some(JsonnetValue::Object(ac_obj)) = obj.get("accessControl") {
            let enabled = Self::extract_bool(ac_obj, "enabled").unwrap_or(true);
            let provider = Self::extract_access_control_provider(ac_obj)?;
            let roles = Self::extract_string_array(ac_obj, "roles")?;
            let policies = Self::extract_string_array(ac_obj, "policies")?;

            Ok(AccessControlConfig {
                enabled,
                provider,
                roles,
                policies,
            })
        } else {
            Ok(AccessControlConfig {
                enabled: true,
                provider: AccessControlProvider::AWSIAM,
                roles: Vec::new(),
                policies: Vec::new(),
            })
        }
    }

    /// Extract access control provider
    fn extract_access_control_provider(obj: &HashMap<String, JsonnetValue>) -> Result<AccessControlProvider> {
        let provider_str = Self::extract_string(obj, "provider")
            .unwrap_or_else(|_| "AWSIAM".to_string());

        match provider_str.as_str() {
            "AWSIAM" => Ok(AccessControlProvider::AWSIAM),
            "GCP" => Ok(AccessControlProvider::GCP),
            "AzureRBAC" => Ok(AccessControlProvider::AzureRBAC),
            "Keycloak" => Ok(AccessControlProvider::Keycloak),
            _ => Ok(AccessControlProvider::AWSIAM),
        }
    }

    /// Extract compliance configuration
    fn extract_compliance(obj: &HashMap<String, JsonnetValue>) -> Result<Option<ComplianceConfig>> {
        if let Some(JsonnetValue::Object(comp_obj)) = obj.get("compliance") {
            let standards = Self::extract_compliance_standards(comp_obj)?;
            let audit_enabled = Self::extract_bool(comp_obj, "auditEnabled").unwrap_or(true);
            let audit_retention_days = Self::extract_number(comp_obj, "auditRetentionDays").unwrap_or(365.0 * 7.0) as u32;

            Ok(Some(ComplianceConfig {
                standards,
                audit_enabled,
                audit_retention_days,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract compliance standards
    fn extract_compliance_standards(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<ComplianceStandard>> {
        let mut standards = Vec::new();

        if let Some(JsonnetValue::Array(std_array)) = obj.get("standards") {
            for std_value in std_array {
                if let JsonnetValue::String(std_str) = std_value {
                    let standard = Self::parse_compliance_standard(&std_str)?;
                    standards.push(standard);
                }
            }
        }

        Ok(standards)
    }

    /// Parse compliance standard
    fn parse_compliance_standard(std_str: &str) -> Result<ComplianceStandard> {
        match std_str {
            "SOC2" => Ok(ComplianceStandard::SOC2),
            "HIPAA" => Ok(ComplianceStandard::HIPAA),
            "PCI_DSS" => Ok(ComplianceStandard::PCI_DSS),
            "GDPR" => Ok(ComplianceStandard::GDPR),
            "ISO27001" => Ok(ComplianceStandard::ISO27001),
            _ => Err(KotobaNetError::DeployConfig(format!("Invalid compliance standard: {}", std_str))),
        }
    }

    // Helper methods

    fn extract_string(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<String> {
        match obj.get(key) {
            Some(JsonnetValue::String(s)) => Ok(s.clone()),
            _ => Err(KotobaNetError::DeployConfig(format!("Expected string for key '{}'", key))),
        }
    }

    fn extract_bool(obj: &HashMap<String, JsonnetValue>, key: &str) -> Option<bool> {
        match obj.get(key) {
            Some(JsonnetValue::Boolean(b)) => Some(*b),
            _ => None,
        }
    }

    fn extract_number(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<f64> {
        match obj.get(key) {
            Some(JsonnetValue::Number(n)) => Ok(*n),
            _ => Err(KotobaNetError::DeployConfig(format!("Expected number for key '{}'", key))),
        }
    }

    fn extract_string_array(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<Vec<String>> {
        match obj.get(key) {
            Some(JsonnetValue::Array(arr)) => {
                let mut strings = Vec::new();
                for item in arr {
                    if let JsonnetValue::String(s) = item {
                        strings.push(s.clone());
                    }
                }
                Ok(strings)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn jsonnet_object_to_hashmap(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        let mut map = serde_json::Map::new();
        for (key, value) in obj {
            let json_value = Self::jsonnet_value_to_json_value(value)?;
            map.insert(key.clone(), json_value);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn jsonnet_value_to_json_value(value: &JsonnetValue) -> Result<serde_json::Value> {
        match value {
            JsonnetValue::Null => Ok(serde_json::Value::Null),
            JsonnetValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            JsonnetValue::Number(n) => Ok(serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap())),
            JsonnetValue::String(s) => Ok(serde_json::Value::String(s.clone())),
            JsonnetValue::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(Self::jsonnet_value_to_json_value(item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            }
            JsonnetValue::Object(obj) => Self::jsonnet_object_to_hashmap(obj),
            JsonnetValue::Function(_) => Err(KotobaNetError::DeployConfig("Functions cannot be converted to JSON".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_deploy_config() {
        let config = r#"
        {
            name: "my-app",
            version: "1.0.0",
            environment: "production",
            scaling: {
                minInstances: 2,
                maxInstances: 10,
                targetCpuUtilization: 0.7,
            },
            regions: [
                {
                    name: "us-east-1",
                    provider: "AWS",
                    instanceType: "t3.medium",
                }
            ]
        }
        "#;

        let result = DeployParser::parse(config);
        assert!(result.is_ok());

        let deploy_config = result.unwrap();
        assert_eq!(deploy_config.name, "my-app");
        assert_eq!(deploy_config.version, "1.0.0");
        assert!(matches!(deploy_config.environment, DeploymentEnvironment::Production));
        assert_eq!(deploy_config.scaling.min_instances, 2);
        assert_eq!(deploy_config.scaling.max_instances, 10);
    }
}

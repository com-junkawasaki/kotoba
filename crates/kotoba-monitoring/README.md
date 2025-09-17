# KotobaDB Monitoring & Metrics

**Comprehensive monitoring and metrics collection system for KotobaDB** with health checks, performance monitoring, and Prometheus integration.

## Features

- **Metrics Collection**: Automatic collection of database, system, and application metrics
- **Health Checks**: Comprehensive health monitoring with configurable checks
- **Performance Monitoring**: Real-time performance tracking and analysis
- **Prometheus Integration**: Native Prometheus metrics export and scraping
- **Alerting System**: Configurable alerting rules and notifications
- **Custom Metrics**: Extensible metrics collection framework
- **Historical Data**: Time-series metrics storage and querying

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kotoba-monitoring = "0.1.0"
```

### Basic Monitoring Setup

```rust
use kotoba_monitoring::*;
use std::sync::Arc;

// Create monitoring configuration
let monitoring_config = MonitoringConfig {
    enable_metrics: true,
    enable_health_checks: true,
    collection_interval: Duration::from_secs(15),
    health_check_interval: Duration::from_secs(30),
    prometheus_config: PrometheusConfig {
        enabled: true,
        address: "127.0.0.1".to_string(),
        port: 9090,
        path: "/metrics".to_string(),
        global_labels: HashMap::new(),
    },
    ..Default::default()
};

// Create metrics collector (assuming you have a KotobaDB instance)
let collector = Arc::new(MetricsCollector::new(db_instance, monitoring_config.clone()));

// Create health checker
let health_checker = HealthChecker::new(monitoring_config.clone());
health_checker.add_default_checks().await?;

// Create Prometheus exporter
let exporter = PrometheusExporter::new(Arc::clone(&collector), monitoring_config.prometheus_config)?;

// Start monitoring
collector.start().await?;
health_checker.start().await?;
exporter.start().await?;

println!("Monitoring system started");

// Monitor health
let health = health_checker.check_health().await?;
println!("System health: {:?}", health.overall_status);

// Get performance metrics
let performance = collector.get_performance_metrics().await?;
println!("Queries per second: {}", performance.query_metrics.queries_per_second);
```

### Custom Metrics

```rust
// Record custom metrics
collector.record_metric(
    "custom_operation_duration",
    1.5,
    hashmap! {
        "operation".to_string() => "user_registration".to_string(),
        "region".to_string() => "us-east".to_string(),
    }
).await?;

// Record using Prometheus helpers
use kotoba_monitoring::prometheus_exporter::*;

record_counter("user_registrations_total", 1, &[("region", "us-east")]);
record_gauge("active_users", 150.0, &[("service", "auth")]);
record_histogram("request_duration", 0.25, &[("endpoint", "/api/users")]);
```

### Health Checks

```rust
// Create custom health check
struct CustomHealthCheck;

#[async_trait::async_trait]
impl HealthCheck for CustomHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        // Your custom health check logic
        let status = HealthStatus::Healthy;
        let message = "Custom service is healthy".to_string();

        HealthCheckResult {
            name: "custom_service".to_string(),
            status,
            message,
            duration: Duration::from_millis(50),
            details: hashmap! {
                "version".to_string() => "1.2.3".to_string(),
                "uptime".to_string() => "2h 30m".to_string(),
            },
        }
    }
}

// Register custom health check
health_checker.register_check("custom".to_string(), Box::new(CustomHealthCheck)).await?;
```

## Metrics Categories

### Database Metrics

```rust
// Automatically collected database metrics
let db_metrics = collector.get_metrics(
    "database_connections_active",
    Utc::now() - Duration::hours(1),
    Utc::now()
).await?;
```

Available database metrics:
- `kotoba_db_connections_active`: Active database connections
- `kotoba_db_connections_total`: Total database connections
- `kotoba_db_queries_total`: Total number of queries
- `kotoba_db_query_latency_seconds`: Query latency histogram
- `kotoba_db_storage_size_bytes`: Total storage size
- `kotoba_db_storage_used_bytes`: Used storage size

### System Metrics (with `system` feature)

```toml
[dependencies]
kotoba-monitoring = { version = "0.1.0", features = ["system"] }
```

Available system metrics:
- `system_cpu_usage_percent`: CPU usage percentage
- `system_memory_usage_bytes`: Memory usage in bytes
- `system_memory_usage_percent`: Memory usage percentage
- `system_disk_usage_bytes`: Disk usage in bytes
- `system_disk_usage_percent`: Disk usage percentage

### Cluster Metrics (with `cluster` feature)

```toml
[dependencies]
kotoba-monitoring = { version = "0.1.0", features = ["cluster"] }
```

Available cluster metrics:
- `kotoba_cluster_nodes_total`: Total cluster nodes
- `kotoba_cluster_nodes_active`: Active cluster nodes
- `kotoba_cluster_leader_changes_total`: Leader change events

## Prometheus Integration

### Configuration

```rust
let prometheus_config = PrometheusConfig {
    enabled: true,
    address: "0.0.0.0".to_string(),  // Listen on all interfaces
    port: 9090,
    path: "/metrics".to_string(),
    global_labels: hashmap! {
        "service".to_string() => "kotoba-db".to_string(),
        "environment".to_string() => "production".to_string(),
    },
};
```

### Accessing Metrics

Once started, metrics are available at: `http://localhost:9090/metrics`

```bash
# View metrics
curl http://localhost:9090/metrics

# Example output
# HELP kotoba_db_connections_active Number of active database connections
# TYPE kotoba_db_connections_active gauge
# kotoba_db_connections_active{service="kotoba-db",environment="production"} 5

# HELP kotoba_db_query_latency_seconds Database query latency in seconds
# TYPE kotoba_db_query_latency_seconds histogram
# ...
```

### Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'kotoba-db'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

### Grafana Dashboards

Use the exported metrics to create dashboards in Grafana:

- **Database Performance**: Query latency, throughput, connection counts
- **Storage Usage**: Disk usage, I/O operations, cache hit rates
- **System Resources**: CPU, memory, network usage
- **Health Status**: Service health indicators and alerts

## Alerting System

### Alert Rules

```rust
let alerting_config = AlertingConfig {
    enabled: true,
    rules: vec![
        AlertRule {
            name: "High CPU Usage".to_string(),
            description: "CPU usage is above 80%".to_string(),
            query: "system_cpu_usage_percent > 80".to_string(),
            threshold: AlertThreshold::GreaterThan(80.0),
            evaluation_interval: Duration::from_secs(60),
            severity: AlertSeverity::Warning,
            labels: hashmap! {
                "team".to_string() => "infrastructure".to_string(),
            },
        },
        AlertRule {
            name: "Database Down".to_string(),
            description: "Database health check is failing".to_string(),
            query: "health_check_status{check_name=\"database\"} == 0".to_string(),
            threshold: AlertThreshold::Equal(0.0),
            evaluation_interval: Duration::from_secs(30),
            severity: AlertSeverity::Critical,
            labels: hashmap! {
                "service".to_string() => "database".to_string(),
            },
        },
    ],
    notifications: vec![
        NotificationConfig {
            notification_type: NotificationType::Slack,
            config: hashmap! {
                "webhook_url".to_string() => "https://hooks.slack.com/...".to_string(),
                "channel".to_string() => "#alerts".to_string(),
            },
        },
    ],
};
```

### Alert Severities

- **Info**: Informational alerts (e.g., version updates)
- **Warning**: Warning conditions (e.g., high resource usage)
- **Error**: Error conditions (e.g., service degradation)
- **Critical**: Critical conditions (e.g., service down)

### Notification Channels

- **Email**: SMTP-based email notifications
- **Slack**: Slack webhook notifications
- **Webhook**: HTTP webhook notifications
- **PagerDuty**: PagerDuty integration

## Health Checks

### Built-in Health Checks

The system includes several built-in health checks:

- **Database**: Database connectivity and responsiveness
- **Memory**: Memory usage monitoring
- **Disk**: Disk space availability
- **CPU**: CPU usage monitoring
- **Network**: Network connectivity (cluster mode)

### Health Status Levels

- **Healthy**: All systems operational
- **Degraded**: Some non-critical issues detected
- **Unhealthy**: Critical issues requiring attention
- **Unknown**: Health status cannot be determined

### Custom Health Checks

```rust
struct ExternalServiceHealthCheck {
    service_url: String,
}

#[async_trait::async_trait]
impl HealthCheck for ExternalServiceHealthCheck {
    async fn check_health(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Check external service
        let client = reqwest::Client::new();
        let response = client
            .get(&self.service_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        let duration = start.elapsed();

        match response {
            Ok(resp) if resp.status().is_success() => HealthCheckResult {
                name: "external_service".to_string(),
                status: HealthStatus::Healthy,
                message: "External service is responding".to_string(),
                duration,
                details: hashmap! {
                    "response_time_ms".to_string() => duration.as_millis().to_string(),
                    "status_code".to_string() => resp.status().as_u16().to_string(),
                },
            },
            Ok(resp) => HealthCheckResult {
                name: "external_service".to_string(),
                status: HealthStatus::Degraded,
                message: format!("External service returned status {}", resp.status()),
                duration,
                details: HashMap::new(),
            },
            Err(e) => HealthCheckResult {
                name: "external_service".to_string(),
                status: HealthStatus::Unhealthy,
                message: format!("External service unreachable: {}", e),
                duration,
                details: HashMap::new(),
            },
        }
    }
}
```

## Performance Monitoring

### Real-time Metrics

```rust
// Get current performance snapshot
let performance = collector.get_performance_metrics().await?;

println!("Query Performance:");
println!("  Total queries: {}", performance.query_metrics.total_queries);
println!("  Queries/sec: {:.2}", performance.query_metrics.queries_per_second);
println!("  Avg latency: {:.2}ms", performance.query_metrics.avg_query_latency_ms);
println!("  P95 latency: {:.2}ms", performance.query_metrics.p95_query_latency_ms);

println!("Storage Performance:");
println!("  Total size: {} bytes", performance.storage_metrics.total_size_bytes);
println!("  Used size: {} bytes", performance.storage_metrics.used_size_bytes);
println!("  Cache hit rate: {:.2}%", performance.storage_metrics.cache_hit_rate * 100.0);
```

### Historical Analysis

```rust
// Get metrics for the last hour
let from = Utc::now() - Duration::hours(1);
let to = Utc::now();

let query_latencies = collector.get_metrics("query_latency", from, to).await?;
let avg_latency = query_latencies.iter()
    .map(|p| p.value)
    .sum::<f64>() / query_latencies.len() as f64;

println!("Average query latency over last hour: {:.2}ms", avg_latency);
```

## Configuration

### Advanced Configuration

```rust
let config = MonitoringConfig {
    enable_metrics: true,
    enable_health_checks: true,
    collection_interval: Duration::from_secs(10),  // More frequent collection
    health_check_interval: Duration::from_secs(20),
    retention_period: Duration::from_secs(7200),  // 2 hours retention
    max_metrics_points: 50000,                    // Higher limit
    prometheus_config: PrometheusConfig {
        enabled: true,
        address: "127.0.0.1".to_string(),
        port: 9090,
        path: "/metrics".to_string(),
        global_labels: hashmap! {
            "cluster".to_string() => "production".to_string(),
            "region".to_string() => "us-east-1".to_string(),
        },
    },
    alerting_config: AlertingConfig {
        enabled: true,
        rules: vec![/* alert rules */],
        notifications: vec![/* notification configs */],
    },
};
```

### Environment Variables

```bash
# Prometheus configuration
export KOTOBA_METRICS_PORT=9090
export KOTOBA_METRICS_PATH=/metrics

# Alerting configuration
export KOTOBA_ALERT_SLACK_WEBHOOK=https://hooks.slack.com/...
export KOTOBA_ALERT_EMAIL_SMTP=smtp.gmail.com:587
```

## Architecture

```
┌─────────────────────────────────────────┐
│            Application Layer            │
├─────────────────────────────────────────┤
│    ┌─────────────┬─────────────┬─────┐  │
│    │Metrics Coll│Health Check │Alert│  │ ← Monitoring Components
│    │ector       │er          │ing │  │
│    └─────────────┴─────────────┴─────┘  │
├─────────────────────────────────────────┤
│    ┌─────────────────────────────────┐  │
│    │    Prometheus HTTP Server       │  │ ← Metrics Export
│    └─────────────────────────────────┘  │
├─────────────────────────────────────────┤
│    ┌─────────────────────────────────┐  │
│    │      Metrics Storage            │  │ ← Time-series Storage
│    └─────────────────────────────────┘  │
├─────────────────────────────────────────┤
│         Database Integration          │ ← KotobaDB Integration
└─────────────────────────────────────────┘
```

## Integration Examples

### With KotobaDB

```rust
use kotoba_db::DB;
use kotoba_monitoring::*;

// Create database
let db = DB::open_lsm("./database").await?;

// Wrap database for monitoring
struct MonitoredKotobaDB {
    db: DB,
}

#[async_trait::async_trait]
impl MonitoredDatabase for MonitoredKotobaDB {
    async fn get_database_metrics(&self) -> Result<DatabaseMetrics, MonitoringError> {
        // Implement database metrics collection
        Ok(DatabaseMetrics {
            active_connections: 10,
            total_connections: 15,
            uptime_seconds: 3600,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    async fn get_query_metrics(&self) -> Result<QueryMetrics, MonitoringError> {
        // Implement query metrics collection
        Ok(QueryMetrics {
            total_queries: 1000,
            queries_per_second: 50.0,
            avg_query_latency_ms: 25.0,
            p95_query_latency_ms: 50.0,
            p99_query_latency_ms: 100.0,
            slow_queries: 5,
            failed_queries: 1,
        })
    }

    async fn get_storage_metrics(&self) -> Result<StorageMetrics, MonitoringError> {
        // Implement storage metrics collection
        Ok(StorageMetrics {
            total_size_bytes: 1_000_000_000,
            used_size_bytes: 500_000_000,
            read_operations: 10000,
            write_operations: 5000,
            read_bytes_per_sec: 100_000.0,
            write_bytes_per_sec: 50_000.0,
            cache_hit_rate: 0.95,
            io_latency_ms: 10.0,
        })
    }
}

let monitored_db = Arc::new(MonitoredKotobaDB { db });
let collector = Arc::new(MetricsCollector::new(monitored_db, config));
```

### With Custom Metrics

```rust
// Custom application metrics
async fn record_business_metrics(collector: &MetricsCollector) {
    // Business logic metrics
    collector.record_metric(
        "orders_total",
        150.0,
        hashmap! {
            "status".to_string() => "completed".to_string(),
            "region".to_string() => "us-east".to_string(),
        }
    ).await?;

    collector.record_metric(
        "revenue_total",
        25000.0,
        hashmap! {
            "currency".to_string() => "USD".to_string(),
            "period".to_string() => "daily".to_string(),
        }
    ).await?;
}
```

## Best Practices

### Monitoring Setup

1. **Start Simple**: Begin with basic health checks and essential metrics
2. **Define SLOs**: Set Service Level Objectives before configuring alerts
3. **Use Labels**: Properly label metrics for effective querying and aggregation
4. **Monitor Trends**: Focus on trends rather than absolute values
5. **Test Alerts**: Regularly test alerting rules to avoid alert fatigue

### Alert Configuration

1. **Start with Critical**: Configure alerts for truly critical conditions first
2. **Use Appropriate Thresholds**: Set thresholds based on historical data
3. **Avoid Noise**: Use aggregation and filtering to reduce false positives
4. **Escalation Paths**: Define clear escalation procedures for different alert severities
5. **Regular Review**: Regularly review and adjust alerting rules

### Performance Considerations

1. **Metrics Overhead**: Monitor the performance impact of metrics collection
2. **Storage Limits**: Configure appropriate retention periods and limits
3. **Network Usage**: Consider network overhead for distributed deployments
4. **Resource Usage**: Allocate sufficient resources for monitoring components

## Troubleshooting

### Common Issues

#### Metrics Not Appearing in Prometheus

```bash
# Check if metrics endpoint is accessible
curl http://localhost:9090/metrics

# Verify Prometheus configuration
# Check scrape target status in Prometheus UI
```

#### High Memory Usage

```rust
// Reduce metrics retention
let config = MonitoringConfig {
    retention_period: Duration::from_secs(1800), // 30 minutes
    max_metrics_points: 10000,
    ..Default::default()
};
```

#### Slow Health Checks

```rust
// Increase health check intervals
let config = MonitoringConfig {
    health_check_interval: Duration::from_secs(60), // Less frequent
    ..Default::default()
};
```

#### Alert Spam

```rust
// Add hysteresis to alert rules
// Use rate limiting for notifications
// Implement alert aggregation
```

## Future Enhancements

- **Distributed Tracing**: Request tracing across services
- **Anomaly Detection**: ML-based anomaly detection in metrics
- **Predictive Alerting**: Predictive maintenance alerts
- **Custom Dashboards**: Built-in dashboard generation
- **Metrics Federation**: Cross-cluster metrics aggregation

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new metrics/checks
4. Update documentation
5. Submit a pull request

## License

Licensed under the MIT License.

---

**KotobaDB Monitoring & Metrics** - *Comprehensive observability for modern databases* 📊📈

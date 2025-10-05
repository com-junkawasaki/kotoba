# Kubernetes Deployment Configuration

This directory contains Kubernetes manifests and deployment configurations for running Kotoba in a cloud-native environment.

## Directory Structure

```
k8s/
├── configmap.yaml         # Configuration data and environment variables
├── deploy.sh              # Deployment script for automated setup
├── ingress.yaml           # Ingress configuration for external access
├── namespace.yaml         # Namespace definition
├── services.yaml          # Service definitions (ClusterIP, LoadBalancer)
├── statefulset.yaml       # StatefulSet for database persistence
├── storage.yaml           # Persistent volume and storage class definitions
├── templates/             # Helm chart templates
├── values.yaml            # Helm chart default values
├── kind/                  # Kind (Kubernetes in Docker) configurations
│   ├── config.yaml        # Kind cluster configuration
│   └── deploy.sh          # Kind-specific deployment script
└── README.md              # This file
```

## Components

### Core Infrastructure

#### `namespace.yaml`
**Purpose**: Defines the `kotoba-system` namespace
- **Labels**: Environment and team identification
- **Resource Quotas**: CPU and memory limits
- **Network Policies**: Pod-to-pod communication rules

#### `configmap.yaml`
**Purpose**: Centralized configuration management
- **Database Configuration**: Connection strings and credentials
- **Application Settings**: Environment-specific parameters
- **Feature Flags**: Runtime configuration toggles
- **Secrets**: Base64 encoded sensitive data references

#### `secrets.yaml` (not shown)
**Purpose**: Secure credential management
- **Database Credentials**: Encrypted database access
- **API Keys**: External service authentication
- **TLS Certificates**: HTTPS certificate storage
- **Service Account Tokens**: Kubernetes authentication

### Storage Layer

#### `storage.yaml`
**Purpose**: Persistent storage configuration
- **PersistentVolumeClaims**: Dynamic volume provisioning
- **StorageClasses**: Performance and redundancy options
- **Volume Snapshots**: Backup and restore capabilities
- **Multi-zone Replication**: Cross-region data durability

#### `statefulset.yaml`
**Purpose**: Stateful application deployment
- **Database Pods**: Persistent data management
- **Ordered Deployment**: Sequential pod startup
- **PVC Binding**: Stable volume attachment
- **Rolling Updates**: Zero-downtime updates

### Network Layer

#### `services.yaml`
**Purpose**: Service discovery and load balancing
- **ClusterIP Services**: Internal pod communication
- **LoadBalancer Services**: External traffic ingress
- **Headless Services**: Direct pod access for stateful sets
- **Service Mesh Integration**: Istio/Traffic management

#### `ingress.yaml`
**Purpose**: External traffic routing
- **HTTP/HTTPS Routing**: Path and host-based routing
- **SSL/TLS Termination**: Certificate management
- **Rate Limiting**: DDoS protection and traffic shaping
- **Authentication**: JWT validation and access control

### Deployment Automation

#### `deploy.sh`
**Purpose**: Automated deployment pipeline
- **Environment Setup**: Namespace and prerequisites
- **Rolling Deployment**: Zero-downtime application updates
- **Health Checks**: Post-deployment verification
- **Rollback Procedures**: Automated failure recovery

## Deployment Scenarios

### Development Environment

```bash
# Quick local deployment with Kind
cd k8s/kind
./deploy.sh dev

# Features:
# - Single-node cluster
# - Local storage
# - Development configurations
# - Hot reload capabilities
```

### Production Environment

```bash
# Production deployment
./deploy.sh prod

# Features:
# - Multi-zone deployment
# - Persistent storage
# - Load balancing
# - Monitoring integration
# - Backup automation
```

### High Availability Setup

```bash
# HA deployment with redundancy
./deploy.sh ha

# Features:
# - Multi-node clusters
# - Cross-zone replication
- Load balancer configuration
- Automatic failover
- Performance monitoring
```

## Configuration Management

### Helm Integration

#### `values.yaml`
Default configuration values for Helm deployments:

```yaml
# Application configuration
app:
  name: kotoba
  version: "0.1.0"
  replicas: 3

# Database configuration
database:
  type: postgresql
  host: kotoba-db
  port: 5432

# Ingress configuration
ingress:
  enabled: true
  className: nginx
  hosts:
    - host: kotoba.example.com
      paths:
        - path: /
          pathType: Prefix
```

### Environment-Specific Overrides

```yaml
# production-values.yaml
app:
  replicas: 5
  resources:
    requests:
      cpu: 1000m
      memory: 2Gi
    limits:
      cpu: 2000m
      memory: 4Gi

ingress:
  tls:
    - secretName: kotoba-tls
      hosts:
        - kotoba.example.com
```

## Monitoring and Observability

### Integrated Monitoring

```yaml
# Prometheus metrics collection
monitoring:
  prometheus:
    enabled: true
    scrapeInterval: 30s

  grafana:
    enabled: true
    dashboards:
      - kotoba-overview
      - database-metrics
      - application-performance

  alertmanager:
    enabled: true
    rules:
      - kotoba-pod-restart
      - database-connection-failure
      - high-memory-usage
```

### Logging Integration

```yaml
# Centralized logging
logging:
  fluentd:
    enabled: true
    configMap: kotoba-logging-config

  elasticsearch:
    enabled: true
    replicas: 3

  kibana:
    enabled: true
    dashboards:
      - kotoba-application-logs
      - error-analysis
      - performance-metrics
```

## Security Configuration

### Network Policies

```yaml
# Pod-to-pod communication rules
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: kotoba-network-policy
spec:
  podSelector:
    matchLabels:
      app: kotoba
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: kotoba
    ports:
    - protocol: TCP
      port: 8080
```

### RBAC Configuration

```yaml
# Service account permissions
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: kotoba-cluster-role
rules:
- apiGroups: [""]
  resources: ["pods", "services", "configmaps"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
```

## Scaling and Performance

### Horizontal Pod Autoscaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: kotoba-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: kotoba
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Vertical Pod Autoscaling

```yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: kotoba-vpa
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: kotoba
  updatePolicy:
    updateMode: "Auto"
```

## Disaster Recovery

### Backup Configuration

```yaml
# Automated backup schedule
backup:
  schedule: "0 2 * * *"  # Daily at 2 AM
  retention: 30d
  storage:
    type: s3
    bucket: kotoba-backups
    region: us-west-2
```

### Restore Procedures

```bash
# Database restore
kubectl apply -f k8s/restore-job.yaml

# Application rollback
kubectl rollout undo deployment/kotoba --to-revision=2

# Complete cluster restore
./deploy.sh restore
```

## Development Workflow

### Local Development

```bash
# Start local Kind cluster
kind create cluster --config k8s/kind/config.yaml

# Deploy application
./k8s/deploy.sh dev

# Port forward for local access
kubectl port-forward svc/kotoba 8080:80

# View logs
kubectl logs -f deployment/kotoba
```

### CI/CD Integration

```yaml
# .github/workflows/deploy.yml
name: Deploy to Kubernetes
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    - name: Deploy to K8s
      run: |
        ./k8s/deploy.sh prod
```

## Troubleshooting

### Common Issues

#### Pod Startup Failures
```bash
# Check pod status
kubectl get pods -n kotoba-system

# View detailed logs
kubectl describe pod <pod-name>

# Check resource constraints
kubectl top pods
```

#### Network Connectivity
```bash
# Test service connectivity
kubectl exec -it <pod-name> -- curl http://localhost:8080/health

# Check network policies
kubectl get networkpolicies

# Verify ingress configuration
kubectl describe ingress kotoba-ingress
```

#### Storage Issues
```bash
# Check PVC status
kubectl get pvc

# Verify storage class
kubectl get storageclass

# Check volume mounts
kubectl describe pod <pod-name>
```

## Integration with Process Network

This directory is part of the Kubernetes deployment process network:

- **Node**: `kubernetes_deployment`
- **Type**: `infrastructure`
- **Dependencies**: `docker_infrastructure`
- **Provides**: K8s manifests, services, ingress, storage
- **Build Order**: 2

## Best Practices

1. **Use Helm Charts**: For complex deployments and configuration management
2. **Implement Health Checks**: Ensure proper liveness and readiness probes
3. **Configure Resource Limits**: Prevent resource exhaustion
4. **Use Network Policies**: Secure pod-to-pod communication
5. **Implement Monitoring**: Set up comprehensive observability
6. **Automate Backups**: Regular data protection and disaster recovery
7. **Test Deployments**: Use staging environments before production
8. **Document Changes**: Keep deployment configurations versioned and documented

## Related Components

- **Docker Configuration**: `Dockerfile` (container build)
- **Helm Charts**: `packages/kotoba-workflow-designer/` (application packaging)
- **CI/CD**: `.github/workflows/` (automated deployment)
- **Monitoring**: `crates/kotoba-monitoring/` (observability)
- **Security**: `crates/kotoba-security/` (authentication and authorization)

---

## Quick Start

### Deploy to Local Kind Cluster

```bash
# Install Kind and kubectl
# Then run:
cd k8s/kind
./deploy.sh

# Access the application
kubectl port-forward svc/kotoba 8080:80
# Visit http://localhost:8080
```

### Deploy to Production Cluster

```bash
# Ensure kubectl context is set to production cluster
kubectl config current-context

# Deploy with production configuration
./k8s/deploy.sh prod

# Verify deployment
kubectl get all -n kotoba-system
```

This Kubernetes configuration provides a complete, production-ready deployment setup for the Kotoba system with high availability, security, and monitoring capabilities.
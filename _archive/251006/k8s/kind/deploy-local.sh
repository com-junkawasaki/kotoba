#!/bin/bash
set -e

# Local deployment script for Kotoba on Kind
# Usage: ./deploy-local.sh [cluster-name] [namespace]

CLUSTER_NAME=${1:-"kotoba-local"}
NAMESPACE=${2:-"kotoba-system"}
IMAGE_TAG=${3:-"latest"}

echo "🚀 Deploying Kotoba locally with Helm + Kind"
echo "Cluster: $CLUSTER_NAME"
echo "Namespace: $NAMESPACE"
echo "Image Tag: $IMAGE_TAG"

# Check prerequisites
echo "🔍 Checking prerequisites..."

# Check if kind is installed
if ! command -v kind &> /dev/null; then
    echo "❌ kind is not installed. Please install kind first:"
    echo "   https://kind.sigs.k8s.io/docs/user/quick-start/"
    exit 1
fi

# Check if helm is installed
if ! command -v helm &> /dev/null; then
    echo "❌ helm is not installed. Please install helm first:"
    echo "   https://helm.sh/docs/intro/install/"
    exit 1
fi

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    echo "❌ kubectl is not installed. Please install kubectl first."
    exit 1
fi

# Check if docker is running
if ! docker info &> /dev/null; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Build Docker image
echo "🐳 Building Kotoba Docker image..."
docker build -t kotoba:$IMAGE_TAG .

# Load image into Kind cluster (if cluster exists)
if kind get clusters | grep -q "^${CLUSTER_NAME}$"; then
    echo "📦 Loading image into existing Kind cluster..."
    kind load docker-image kotoba:$IMAGE_TAG --name $CLUSTER_NAME
else
    echo "🏗️  Creating Kind cluster..."
    kind create cluster --name $CLUSTER_NAME --config kind-config.yaml
fi

# Set kubectl context to kind cluster
kubectl cluster-info --context kind-$CLUSTER_NAME

# Create namespace
echo "📁 Creating namespace..."
kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -

# Add Helm repositories if needed
echo "📚 Adding Helm repositories..."
helm repo add stable https://charts.helm.sh/stable --force-update 2>/dev/null || true

# Install local-path provisioner for storage (if not exists)
if ! kubectl get storageclass local-path &>/dev/null; then
    echo "💾 Installing local-path provisioner..."
    kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/master/deploy/local-path-storage.yaml
    kubectl patch storageclass local-path -p '{"metadata":{"annotations":{"storageclass.kubernetes.io/is-default-class":"true"}}}'
fi

# Deploy Kotoba using Helm
echo "🚀 Deploying Kotoba with Helm..."
cd ../

# Package the chart
helm package k8s --version $IMAGE_TAG

# Install/upgrade the chart
helm upgrade --install $CLUSTER_NAME \
    kotoba-$IMAGE_TAG.tgz \
    --namespace $NAMESPACE \
    --values k8s/kind/values-local.yaml \
    --set image.tag=$IMAGE_TAG \
    --wait \
    --timeout 600s

# Wait for deployment to be ready
echo "⏳ Waiting for deployment to be ready..."
kubectl wait --for=condition=ready pod --selector=app.kubernetes.io/name=kotoba --timeout=300s -n $NAMESPACE

# Show deployment status
echo "✅ Deployment completed!"
echo ""
echo "📊 Cluster Status:"
kubectl get pods -n $NAMESPACE
kubectl get svc -n $NAMESPACE
kubectl get pvc -n $NAMESPACE

echo ""
echo "🔗 Access Information:"
echo "  # Port forward for HTTP access:"
echo "  kubectl port-forward svc/$CLUSTER_NAME-external 3000:80 -n $NAMESPACE"
echo ""
echo "  # Port forward for gRPC access:"
echo "  kubectl port-forward svc/$CLUSTER_NAME-external 8080:8080 -n $NAMESPACE"
echo ""
echo "  # Access URLs:"
echo "  HTTP: http://localhost:3000"
echo "  gRPC: localhost:8080"
echo "  Health: http://localhost:3000/health"
echo "  Metrics: http://localhost:3000/metrics"

echo ""
echo "🛠️  Useful commands:"
echo "  # View logs:"
echo "  kubectl logs -f statefulset/$CLUSTER_NAME -n $NAMESPACE"
echo ""
echo "  # Scale the cluster:"
echo "  kubectl scale statefulset $CLUSTER_NAME --replicas=5 -n $NAMESPACE"
echo ""
echo "  # Access pod shell:"
echo "  kubectl exec -it $CLUSTER_NAME-0 -n $NAMESPACE -- /bin/bash"
echo ""
echo "  # Clean up:"
echo "  helm uninstall $CLUSTER_NAME -n $NAMESPACE"
echo "  kind delete cluster --name $CLUSTER_NAME"

# Test the deployment
echo ""
echo "🧪 Testing deployment..."
sleep 5

# Test health endpoint
echo "Testing health endpoint..."
kubectl port-forward svc/$CLUSTER_NAME-external 3000:80 -n $NAMESPACE &
PORT_FORWARD_PID=$!
sleep 3

if curl -f http://localhost:3000/health &>/dev/null; then
    echo "✅ Health check passed!"
else
    echo "⚠️  Health check failed - check logs for details"
fi

# Kill port forward
kill $PORT_FORWARD_PID 2>/dev/null || true

echo ""
echo "🎉 Kotoba local deployment completed successfully!"
echo "You can now access your Kotoba cluster at http://localhost:3000"

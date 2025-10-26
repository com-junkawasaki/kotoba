#!/bin/bash
set -e

# GKE deployment script for Kotoba
# Usage: ./deploy.sh [project-id] [cluster-name] [region]

PROJECT_ID=${1:-"your-gcp-project"}
CLUSTER_NAME=${2:-"kotoba-cluster"}
REGION=${3:-"us-central1"}

echo "🚀 Deploying Kotoba to GKE"
echo "Project: $PROJECT_ID"
echo "Cluster: $CLUSTER_NAME"
echo "Region: $REGION"

# Check if gcloud is authenticated
echo "🔐 Checking GCP authentication..."
gcloud auth list --filter=status:ACTIVE --format="value(account)" > /dev/null

# Set project
echo "🔧 Setting GCP project..."
gcloud config set project $PROJECT_ID

# Build and push Docker image
echo "🐳 Building and pushing Docker image..."
IMAGE_NAME="gcr.io/$PROJECT_ID/kotoba:latest"

# Update the StatefulSet with the correct image
sed -i "s|gcr.io/YOUR_PROJECT_ID/kotoba:latest|$IMAGE_NAME|g" k8s/statefulset.yaml

docker build -t $IMAGE_NAME .
docker push $IMAGE_NAME

# Create GKE cluster if it doesn't exist
echo "☸️  Checking GKE cluster..."
if ! gcloud container clusters describe $CLUSTER_NAME --region=$REGION &>/dev/null; then
    echo "Creating GKE cluster..."
    gcloud container clusters create $CLUSTER_NAME \
        --region=$REGION \
        --num-nodes=3 \
        --machine-type=e2-standard-4 \
        --disk-size=100GB \
        --enable-autoscaling \
        --min-nodes=3 \
        --max-nodes=10 \
        --enable-ip-alias \
        --enable-stackdriver-kubernetes
else
    echo "GKE cluster already exists"
fi

# Get cluster credentials
echo "🔑 Getting cluster credentials..."
gcloud container clusters get-credentials $CLUSTER_NAME --region=$REGION

# Create namespace
echo "📁 Creating namespace..."
kubectl apply -f k8s/namespace.yaml

# Deploy ConfigMap
echo "⚙️  Deploying configuration..."
kubectl apply -f k8s/configmap.yaml

# Deploy storage
echo "💾 Deploying persistent storage..."
kubectl apply -f k8s/storage.yaml

# Deploy StatefulSet
echo "🚀 Deploying Kotoba cluster..."
kubectl apply -f k8s/statefulset.yaml

# Wait for pods to be ready
echo "⏳ Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod --selector=app=kotoba --timeout=300s -n kotoba-system

# Deploy services
echo "🌐 Deploying services..."
kubectl apply -f k8s/services.yaml

# Deploy autoscaling
echo "📈 Deploying autoscaling..."
kubectl apply -f k8s/autoscaling.yaml

# Deploy ingress (optional)
echo "🌍 Deploying ingress..."
kubectl apply -f k8s/ingress.yaml

# Verify deployment
echo "✅ Verifying deployment..."
kubectl get pods -n kotoba-system
kubectl get svc -n kotoba-system
kubectl get pvc -n kotoba-system

echo "🎉 Kotoba deployment completed!"
echo ""
echo "📋 Useful commands:"
echo "  # Check pod status"
echo "  kubectl get pods -n kotoba-system"
echo ""
echo "  # Check service endpoints"
echo "  kubectl get svc -n kotoba-system"
echo ""
echo "  # View logs"
echo "  kubectl logs -f statefulset/kotoba-cluster -n kotoba-system"
echo ""
echo "  # Scale the cluster"
echo "  kubectl scale statefulset kotoba-cluster --replicas=5 -n kotoba-system"
echo ""
echo "  # Access the cluster"
echo "  kubectl port-forward svc/kotoba-external 8080:80 -n kotoba-system"

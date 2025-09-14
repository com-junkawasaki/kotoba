# Kotoba GKE Deployment Guide

このガイドでは、KotobaをGoogle Kubernetes Engine (GKE)にデプロイする方法を説明します。

## 📋 前提条件

- Google Cloud Platform アカウント
- `gcloud` CLI がインストール済み
- `kubectl` がインストール済み
- Docker がインストール済み
- GCP プロジェクトの作成

## 🚀 クイックスタート

### 1. 環境準備

```bash
# GCP プロジェクトを設定
export PROJECT_ID="your-gcp-project-id"
export CLUSTER_NAME="kotoba-cluster"
export REGION="us-central1"

# gcloud 認証
gcloud auth login
gcloud config set project $PROJECT_ID
```

### 2. デプロイスクリプト実行

```bash
# デプロイスクリプトを実行
./k8s/deploy.sh $PROJECT_ID $CLUSTER_NAME $REGION
```

スクリプトは以下の処理を自動的に実行します：
- Dockerイメージのビルドとプッシュ
- GKEクラスタの作成（存在しない場合）
- Kubernetesリソースのデプロイ
- サービスの起動確認

## 📁 デプロイメント構成

### アーキテクチャ

```
Internet
    ↓
[GKE Ingress]
    ↓
[LoadBalancer Service]
    ↓
[Kotoba StatefulSet] (3 replicas)
    ↓
[Persistent Disk] (GCP PD)
```

### コンポーネント

- **StatefulSet**: Kotoba分散ストレージクラスタ（3ノード）
- **PersistentVolume**: GKE Persistent Diskを使用
- **Service**: クラスタ内通信と外部アクセス
- **Ingress**: HTTP/HTTPSアクセス
- **HPA**: 自動スケーリング
- **PDB**: 障害耐性保証

## ⚙️ 設定カスタマイズ

### クラスタサイズの変更

```bash
# StatefulSetのレプリカ数を変更
kubectl scale statefulset kotoba-cluster --replicas=5 -n kotoba-system
```

### リソース制限の調整

`k8s/statefulset.yaml` のリソース設定を変更：

```yaml
resources:
  requests:
    memory: "4Gi"    # メモリ要求量を増加
    cpu: "2000m"     # CPU要求量を増加
  limits:
    memory: "8Gi"    # メモリ上限を増加
    cpu: "4000m"     # CPU上限を増加
```

### ストレージサイズの調整

`k8s/storage.yaml` のストレージサイズを変更：

```yaml
resources:
  requests:
    storage: 500Gi  # ストレージサイズを500GBに増加
```

## 🔍 監視と運用

### クラスタ状態確認

```bash
# Pod状態確認
kubectl get pods -n kotoba-system

# サービス状態確認
kubectl get svc -n kotoba-system

# PersistentVolume状態確認
kubectl get pvc -n kotoba-system
```

### ログ確認

```bash
# 全Podのログを表示
kubectl logs -f statefulset/kotoba-cluster -n kotoba-system

# 特定Podのログを表示
kubectl logs -f kotoba-cluster-0 -n kotoba-system
```

### メトリクス監視

KotobaはPrometheusメトリクスを `/metrics` エンドポイントで提供します。

```bash
# メトリクス取得
kubectl port-forward svc/kotoba-external 9090:80 -n kotoba-system
curl http://localhost:9090/metrics
```

## 🔧 トラブルシューティング

### よくある問題

#### 1. Podが起動しない

```bash
# 詳細なPod状態確認
kubectl describe pod kotoba-cluster-0 -n kotoba-system

# ログ確認
kubectl logs kotoba-cluster-0 -n kotoba-system --previous
```

#### 2. 永続ボリュームが作成されない

```bash
# PVC状態確認
kubectl get pvc -n kotoba-system
kubectl describe pvc data-kotoba-cluster-0 -n kotoba-system
```

#### 3. サービスにアクセスできない

```bash
# LoadBalancer IP確認
kubectl get svc kotoba-external -n kotoba-system

# ポートフォワーディングでテスト
kubectl port-forward svc/kotoba-external 8080:80 -n kotoba-system
curl http://localhost:8080/health
```

### デバッグコマンド

```bash
# Pod内でのデバッグ
kubectl exec -it kotoba-cluster-0 -n kotoba-system -- /bin/bash

# ネットワーク接続テスト
kubectl exec -it kotoba-cluster-0 -n kotoba-system -- \
  curl -f http://kotoba-cluster-1.kotoba-cluster.kotoba-system.svc.cluster.local:3000/health
```

## 🔄 更新とアップグレード

### ローリングアップデート

```bash
# 新しいイメージで更新
kubectl set image statefulset/kotoba-cluster kotoba=gcr.io/$PROJECT_ID/kotoba:v2.0.0 -n kotoba-system

# 更新状況確認
kubectl rollout status statefulset/kotoba-cluster -n kotoba-system
```

### 設定更新

```bash
# ConfigMap更新
kubectl apply -f k8s/configmap.yaml

# Pod再起動
kubectl rollout restart statefulset/kotoba-cluster -n kotoba-system
```

## 🛡️ セキュリティ

### ネットワークポリシー

クラスタ内通信のみを許可するNetworkPolicyを作成：

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: kotoba-network-policy
  namespace: kotoba-system
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
      port: 8080  # gRPC port
    - protocol: TCP
      port: 3000  # HTTP port
```

### サービスアカウント

GKE Workload Identityを使用した安全な認証：

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: kotoba-sa
  namespace: kotoba-system
  annotations:
    iam.gke.io/gcp-service-account: kotoba-service@$PROJECT_ID.iam.gserviceaccount.com
```

## 📊 パフォーマンスチューニング

### ノードプール設定

高性能ノードプールを作成：

```bash
gcloud container node-pools create high-mem-pool \
  --cluster=$CLUSTER_NAME \
  --region=$REGION \
  --machine-type=n2-highmem-8 \
  --num-nodes=3 \
  --enable-autoscaling \
  --min-nodes=3 \
  --max-nodes=10
```

### ストレージクラス

高性能SSDストレージを使用：

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: pd.csi.storage.gke.io
parameters:
  type: pd-ssd
  replication-type: regional-pd
reclaimPolicy: Retain
```

## 🚀 高度な構成

### マルチゾーン配置

可用性を高めるためのマルチゾーン配置：

```bash
gcloud container clusters create $CLUSTER_NAME \
  --region=$REGION \
  --node-locations=$REGION-a,$REGION-b,$REGION-c \
  --enable-autoscaling \
  --min-nodes=9 \
  --max-nodes=30
```

### 外部ロードバランサー

外部アクセス用のロードバランサー設定：

```yaml
apiVersion: v1
kind: Service
metadata:
  name: kotoba-external-lb
  namespace: kotoba-system
  annotations:
    cloud.google.com/load-balancer-type: "External"
spec:
  type: LoadBalancer
  loadBalancerIP: "YOUR_STATIC_IP"
  # ... 他の設定
```

## 📞 サポート

問題が発生した場合：

1. [Kotoba GitHub Issues](https://github.com/jun784/kotoba/issues) を確認
2. GKE の[トラブルシューティングガイド](https://cloud.google.com/kubernetes-engine/docs/troubleshooting) を参照
3. 以下の情報を含めてIssueを作成：
   - `kubectl get pods -n kotoba-system`
   - `kubectl logs [pod-name] -n kotoba-system`
   - GKEクラスタの設定情報

---

**Kotoba on GKE** - クラウドネイティブな分散グラフデータベースを実現

# Kotoba Local Development with Helm + Kind

このガイドでは、Kotobaをローカル環境でHelm + Kindを使ってデプロイ・検証する方法を説明します。

## 📋 前提条件

- **Docker**: コンテナランタイム
- **Kind**: Kubernetes in Docker (https://kind.sigs.k8s.io/)
- **Helm**: Kubernetesパッケージマネージャー (https://helm.sh/)
- **kubectl**: Kubernetes CLI
- **curl**: HTTPテスト用

## 🚀 クイックスタート

### 1. 必要なツールのインストール

```bash
# Kindのインストール (macOS)
brew install kind

# Helmのインストール (macOS)
brew install helm

# kubectlのインストール (macOS)
brew install kubectl

# または、公式インストーラーを使用
# https://kind.sigs.k8s.io/docs/user/quick-start/
# https://helm.sh/docs/intro/install/
# https://kubernetes.io/docs/tasks/tools/
```

### 2. 一括デプロイ実行

```bash
# デプロイスクリプトを実行
./k8s/kind/deploy-local.sh

# または、カスタム設定で実行
./k8s/kind/deploy-local.sh my-cluster kotoba-dev v1.0.0
```

スクリプトは以下の処理を自動実行：
- ✅ Kindクラスタの作成（4ノード）
- ✅ Dockerイメージのビルド・ロード
- ✅ Helmチャートのデプロイ
- ✅ サービスの起動確認
- ✅ ヘルスチェックテスト

## 🏗️ アーキテクチャ

### Kindクラスタ構成

```
Control Plane (1 node)
├── Ingress-ready label
├── Port mappings: 80→80, 443→443, 3000→30000, 8080→30001

Worker Nodes (3 nodes)
├── Storage-enabled labels
├── Individual port mappings for each pod
└── Local storage provisioner
```

### Kotobaデプロイメント

```
Namespace: kotoba-system
├── StatefulSet: 3 replicas (distributed storage)
├── Services: ClusterIP + headless
├── PVCs: 10Gi each (local-path storage)
├── ConfigMap: Cluster configuration
└── Health checks: HTTP probes
```

## ⚙️ 設定ファイル

### `kind-config.yaml`
- 4ノードクラスタ構成
- ポートマッピング設定
- ストレージノードラベル設定

### `values-local.yaml`
- ローカル開発向けリソース制限
- デバッグログ有効化
- 簡易ストレージ設定
- ヘルスチェック間隔短縮

### `deploy-local.sh`
- 完全自動デプロイスクリプト
- 前提条件チェック
- エラーハンドリング
- デプロイ検証

## 🔧 使用方法

### 基本的なデプロイ

```bash
# デフォルト設定でデプロイ
./k8s/kind/deploy-local.sh

# カスタムクラスタ名でデプロイ
./k8s/kind/deploy-local.sh my-kotoba-cluster

# カスタム名前空間でデプロイ
./k8s/kind/deploy-local.sh kotoba-cluster kotoba-dev

# 特定のイメージタグでデプロイ
./k8s/kind/deploy-local.sh kotoba-cluster default v1.0.0
```

### アクセス方法

デプロイ完了後、以下の方法でアクセスできます：

```bash
# HTTP APIアクセス
kubectl port-forward svc/kotoba-local-external 3000:80 -n kotoba-system
curl http://localhost:3000/health

# gRPCアクセス
kubectl port-forward svc/kotoba-local-external 8080:8080 -n kotoba-system

# ブラウザアクセス
open http://localhost:3000
```

### クラスタ操作

```bash
# クラスタ状態確認
kubectl get pods -n kotoba-system
kubectl get svc -n kotoba-system
kubectl get pvc -n kotoba-system

# ログ監視
kubectl logs -f statefulset/kotoba-local -n kotoba-system

# 特定のPodログ
kubectl logs -f kotoba-local-0 -n kotoba-system

# クラスタスケーリング
kubectl scale statefulset kotoba-local --replicas=5 -n kotoba-system

# Podシェルアクセス
kubectl exec -it kotoba-local-0 -n kotoba-system -- /bin/bash
```

## 🔍 テストと検証

### 自動テスト

デプロイスクリプトは自動的に以下のテストを実行します：

```bash
# ヘルスチェック
curl http://localhost:3000/health

# メトリクス確認
curl http://localhost:3000/metrics

# クラスタステータス
kubectl get nodes
kubectl cluster-info
```

### 手動テスト

```bash
# Kotoba APIテスト
curl -X GET "http://localhost:3000/api/status"

# 分散ストレージテスト
# 各ノードのデータを確認
for i in {0..2}; do
  kubectl exec kotoba-local-$i -n kotoba-system -- df -h /data
done

# ネットワーク接続テスト
kubectl exec kotoba-local-0 -n kotoba-system -- \
  curl -f http://kotoba-local-1.kotoba-local.kotoba-system.svc.cluster.local:3000/health
```

## 🛠️ 開発ワークフロー

### 1. コード変更時

```bash
# Dockerイメージ再ビルド
docker build -t kotoba:dev .

# Kindクラスタにイメージロード
kind load docker-image kotoba:dev --name kotoba-local

# Helmアップグレード
helm upgrade kotoba-local ./k8s \
  --namespace kotoba-system \
  --values ./k8s/kind/values-local.yaml \
  --set image.tag=dev

# 変更確認
kubectl rollout status statefulset/kotoba-local -n kotoba-system
```

### 2. 設定変更時

```bash
# valuesファイル編集
vim k8s/kind/values-local.yaml

# 設定反映
helm upgrade kotoba-local ./k8s \
  --namespace kotoba-system \
  --values ./k8s/kind/values-local.yaml

# ポッド再起動確認
kubectl get pods -n kotoba-system
```

### 3. ログ分析

```bash
# 全ポッドログ
kubectl logs -f -l app.kubernetes.io/name=kotoba -n kotoba-system

# 特定のログレベル
kubectl logs -f kotoba-local-0 -n kotoba-system | grep ERROR

# ログをファイルに保存
kubectl logs kotoba-local-0 -n kotoba-system > debug.log
```

## 🔄 更新とクリーンアップ

### 更新

```bash
# 既存クラスタの更新
./k8s/kind/deploy-local.sh kotoba-local kotoba-system new-tag

# または手動更新
helm upgrade kotoba-local ./k8s --namespace kotoba-system
```

### クリーンアップ

```bash
# Helmリリース削除
helm uninstall kotoba-local -n kotoba-system

# 名前空間削除
kubectl delete namespace kotoba-system

# Kindクラスタ削除
kind delete cluster --name kotoba-local

# Dockerイメージ削除
docker image rm kotoba:latest
```

## 🐛 トラブルシューティング

### よくある問題

#### 1. Podが起動しない

```bash
# 詳細なPod情報確認
kubectl describe pod kotoba-local-0 -n kotoba-system

# ログ確認
kubectl logs kotoba-local-0 -n kotoba-system --previous

# リソース確認
kubectl get nodes --show-labels
kubectl describe node
```

#### 2. イメージが見つからない

```bash
# イメージ確認
docker images | grep kotoba

# イメージ再ビルド
docker build -t kotoba:latest .

# Kindにロード
kind load docker-image kotoba:latest --name kotoba-local
```

#### 3. PVCが作成されない

```bash
# StorageClass確認
kubectl get storageclass

# PVC状態確認
kubectl get pvc -n kotoba-system
kubectl describe pvc data-kotoba-local-0 -n kotoba-system
```

#### 4. ネットワーク接続エラー

```bash
# DNS解決テスト
kubectl exec -it kotoba-local-0 -n kotoba-system -- nslookup kotoba-local-1

# サービス確認
kubectl get endpoints -n kotoba-system

# ポート開放確認
kubectl get svc kotoba-local -n kotoba-system -o yaml
```

### デバッグコマンド

```bash
# クラスタイベント確認
kubectl get events -n kotoba-system --sort-by=.metadata.creationTimestamp

# ノード情報
kubectl describe nodes

# システムログ
kubectl logs -f -n kube-system deployment/coredns

# リソース使用量
kubectl top pods -n kotoba-system
kubectl top nodes
```

## 📊 パフォーマンスチューニング

### リソース調整

```yaml
# values-local.yaml で調整
resources:
  requests:
    memory: "1Gi"  # 必要に応じて増加
    cpu: "1000m"
  limits:
    memory: "2Gi"
    cpu: "2000m"
```

### ストレージ調整

```yaml
# values-local.yaml で調整
storage:
  size: "50Gi"  # 大容量が必要な場合
  className: "local-path"
```

### レプリカ数調整

```bash
# クラスタサイズ変更
kubectl scale statefulset kotoba-local --replicas=5 -n kotoba-system

# valuesファイル更新
cluster:
  replicas: 5
```

## 🔗 統合開発

### IDE統合

```bash
# VS Codeでの開発
code .
# または
cursor .

# kubectl連携
# VS Code拡張: ms-kubernetes-tools.vscode-kubernetes-tools
```

### CI/CD統合

```yaml
# .github/workflows/local-test.yml
name: Local Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Kind
        uses: helm/kind-action@v1.5.0
      - name: Deploy Kotoba
        run: ./k8s/kind/deploy-local.sh
      - name: Run Tests
        run: |
          kubectl port-forward svc/kotoba-local-external 3000:80 -n kotoba-system &
          sleep 10
          curl -f http://localhost:3000/health
```

## 📚 参考リンク

- [Kind Documentation](https://kind.sigs.k8s.io/)
- [Helm Documentation](https://helm.sh/docs/)
- [Kotoba GitHub](https://github.com/com-junkawasaki/kotoba)
- [Kubernetes Documentation](https://kubernetes.io/docs/)

---

**Helm + KindでのローカルKotoba開発** - 分散グラフデータベースをローカルで簡単に検証

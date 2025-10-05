# Kotoba Deploy Examples

Kotoba Deployは、Deno Deployと同等の機能をKotoba上で実現するデプロイメントプラットフォームです。ISO GQLプロトコルとLive Graph Modelを使用して、グローバル分散ネットワーク、自動スケーリング、GitHub連携を提供します。

## 特徴

- **ISO GQLベース**: 標準化されたグラフクエリ言語を使用したデプロイメント管理
- **グローバル分散**: 世界中のエッジロケーションに自動分散配置
- **自動スケーリング**: CPU/メモリ使用率に基づくインテリジェントスケーリング
- **GitHub連携**: プッシュ時に自動デプロイ
- **Live Graph Model**: リアルタイムなデプロイメント状態管理
- **マイクロサービス対応**: 複雑なサービス間依存関係の管理

## クイックスタート

### 1. シンプルなデプロイメント

```bash
# 設定ファイルからデプロイ
kotoba-deploy deploy --config examples/deploy/simple.kotoba-deploy

# コマンドライン引数からデプロイ
kotoba-deploy deploy --name my-app --entry-point src/main.rs --runtime http_server

# デプロイメント状態を確認
kotoba-deploy status --all
```

### 2. マイクロサービスデプロイメント

```bash
# マイクロサービス設定からデプロイ
kotoba-deploy deploy --config examples/deploy/microservices.kotoba-deploy

# 特定のサービスの状態を確認
kotoba-deploy status api-gateway
```

### 3. ISO GQLクエリを使用した管理

```bash
# デプロイメント一覧を取得
kotoba-deploy query "LIST DEPLOYMENTS RETURN id, name, status, instance_count"

# デプロイメントをスケーリング
kotoba-deploy query "SCALE DEPLOYMENT WHERE id = 'my-app' SET instances = 3"

# デプロイメントグラフをクエリ
kotoba-deploy graph --query "MATCH (d:Deployment) WHERE d.status = 'running' RETURN d"
```

## 設定ファイル形式

Kotoba DeployはJsonnetベースの設定ファイルを使用します。これにより、動的な設定生成と再利用が可能です。

### 基本構造

```jsonnet
{
  metadata: {
    name: "my-app",
    version: "1.0.0",
    description: "My awesome application",
  },

  application: {
    entry_point: "src/main.rs",
    runtime: "http_server",
  },

  scaling: {
    min_instances: 1,
    max_instances: 10,
    cpu_threshold: 70.0,
  },

  network: {
    domains: ["my-app.kotoba.dev"],
    regions: ["us-east-1", "eu-west-1"],
  },

  environment: {
    NODE_ENV: "production",
    DATABASE_URL: "...",
  },
}
```

### 高度な設定

```jsonnet
{
  // CDN設定
  network: {
    cdn: {
      enabled: true,
      provider: "cloudflare",
      cache_settings: {
        default_ttl: 3600,
        cache_key: ["host", "path"],
      },
    },
  },

  // カスタム設定
  custom: {
    monitoring: {
      enabled: true,
      metrics_endpoint: "/metrics",
    },
    security: {
      cors_enabled: true,
      rate_limiting: true,
    },
  },
}
```

## ISO GQLデプロイメントクエリ

Kotoba DeployはISO GQLの拡張を使用してデプロイメントを管理します。

### 基本的なクエリ

```sql
-- デプロイメント作成
CREATE DEPLOYMENT
SET name = "my-app",
    entry_point = "src/main.rs"

-- デプロイメント取得
GET DEPLOYMENT
WHERE id = "my-app"
RETURN id, name, status, instance_count

-- デプロイメント一覧
LIST DEPLOYMENTS
RETURN id, name, version, status

-- デプロイメント更新
UPDATE DEPLOYMENT
WHERE id = "my-app"
SET version = "2.0.0"

-- デプロイメント削除
DELETE DEPLOYMENT
WHERE id = "my-app"
```

### スケーリングクエリ

```sql
-- 手動スケーリング
SCALE DEPLOYMENT
WHERE id = "my-app"
SET instances = 5

-- 条件付きスケーリング
SCALE DEPLOYMENT
WHERE id = "my-app" AND cpu_usage > 80
SET instances = instances + 2
```

### ロールバッククエリ

```sql
-- バージョン指定ロールバック
ROLLBACK DEPLOYMENT
WHERE id = "my-app"
TO version = "1.9.0"

-- 自動ロールバック
ROLLBACK DEPLOYMENT
WHERE id = "my-app"
WHEN error_rate > 5.0
```

### グラフクエリ

```sql
-- デプロイメントグラフ
MATCH (d:Deployment)
RETURN d.id, d.name, d.status

-- サービス依存関係
MATCH (s1:Service)-[:DEPENDS_ON]->(s2:Service)
RETURN s1.name, s2.name

-- メトリクス集計
MATCH (d:Deployment)
RETURN
  count(d) as total_deployments,
  avg(d.cpu_usage) as avg_cpu,
  sum(d.request_count) as total_requests
```

## CLIコマンド

### デプロイメント管理

```bash
# デプロイ
kotoba-deploy deploy [OPTIONS]

# アンデプロイ
kotoba-deploy undeploy <name>

# 状態確認
kotoba-deploy status [name] [--all]

# スケーリング
kotoba-deploy scale <name> <instances>

# ロールバック
kotoba-deploy rollback <name> <version>

# ログ表示
kotoba-deploy logs <name> [--follow] [--lines N]
```

### クエリ実行

```bash
# GQLクエリ実行
kotoba-deploy query <query> [--params <file>]

# グラフ表示
kotoba-deploy graph [--query <query>] [--format <format>]
```

### 設定管理

```bash
# GitHub連携設定
kotoba-deploy setup-git <owner> <repo> [--token <token>] [--secret <secret>]

# 設定ファイル検証
kotoba-deploy validate <config-file>
```

## 高度な機能

### GitHub連携

```bash
# GitHub連携を設定
kotoba-deploy setup-git myorg myrepo --token ghp_xxx

# プッシュ時の自動デプロイ設定
# .github/workflows/deploy.yml
name: Deploy to Kotoba
on:
  push:
    branches: [ main ]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Deploy to Kotoba
        run: |
          kotoba-deploy deploy --config .kotoba-deploy
```

### 監視とメトリクス

```bash
# メトリクス取得
kotoba-deploy query "
  MATCH (d:Deployment)
  RETURN d.name, d.cpu_usage, d.memory_usage, d.request_count
"

# アラート設定
# 設定ファイルで監視設定
custom: {
  monitoring: {
    alerts: {
      slack: {
        webhook: "https://hooks.slack.com/...",
        channels: ["#alerts"],
      },
    },
    thresholds: {
      cpu_usage: 80,
      error_rate: 5,
    },
  },
}
```

### セキュリティ

```bash
# SSL/TLS設定
network: {
  domains: [{
    domain: "my-app.com",
    ssl: {
      cert_type: "lets_encrypt",
      hsts: {
        enabled: true,
        max_age: 31536000,
      },
    },
  }],
}

# アクセス制御
custom: {
  security: {
    ip_whitelist: ["203.0.113.0/24"],
    rate_limiting: {
      requests_per_minute: 1000,
    },
    authentication: {
      provider: "oauth2",
      jwt_secret: "...",
    },
  },
}
```

## アーキテクチャ

Kotoba Deployのアーキテクチャは以下のコンポーネントで構成されます：

1. **Deploy Controller**: ISO GQLクエリを処理し、デプロイメントを管理
2. **Scaling Engine**: 自動スケーリングと負荷分散を実行
3. **Network Manager**: グローバル分散ネットワークを管理
4. **Git Integration**: GitHubとの連携を管理
5. **Configuration Parser**: Jsonnet設定ファイルをパース

### データフロー

```
GitHub Push → Webhook → Git Integration → Deploy Controller
                                               ↓
Configuration Parser ← Jsonnet Config ← Deploy Config
                                               ↓
Scaling Engine ← Auto Scaling ← Metrics Collector
                                               ↓
Network Manager ← Global Network ← Edge Locations
```

## ベストプラクティス

### 設定管理

- 環境固有の設定は別ファイルに分離
- 秘密情報は環境変数またはシークレット管理サービスを使用
- 設定のバージョン管理を徹底

### スケーリング

- 適切なmin/maxインスタンス数を設定
- CPU/メモリしきい値をアプリケーション特性に合わせて調整
- クールダウン期間を適切に設定

### セキュリティ

- 常にHTTPSを使用
- IP制限とレート制限を設定
- 定期的な証明書更新

### 監視

- 主要メトリクスを継続監視
- アラートしきい値を適切に設定
- ログを構造化して収集

## トラブルシューティング

### よくある問題

1. **デプロイ失敗**: 設定ファイルの構文エラーを確認
2. **スケーリング遅延**: クールダウン期間を見直し
3. **ネットワーク遅延**: エッジロケーション設定を確認

### デバッグ

```bash
# 詳細ログ表示
RUST_LOG=debug kotoba-deploy deploy --config config.jsonnet

# 設定ファイル検証
kotoba-deploy validate config.jsonnet

# デプロイメント状態詳細
kotoba-deploy status my-app --verbose
```

## 拡張性

Kotoba Deployは拡張可能なアーキテクチャを採用しています：

- **カスタムランタイム**: 新しいランタイムタイプを追加可能
- **サードパーティ統合**: CI/CD、監視、セキュリティツールとの連携
- **プラグインシステム**: デプロイメントライフサイクルを拡張

詳細なAPIドキュメントと拡張ガイドは[公式ドキュメント](https://docs.kotoba.dev/deploy)をご参照ください。

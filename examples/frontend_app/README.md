# Kotoba Web Framework Example

JsonnetベースのフルスタックWebフレームワークの使用例です。Next.js風のApp Router、REST/GraphQL API、データベース統合、認証・認可システムを実装しています。

## プロジェクト構造

```
app/
├── layout.libsonnet           # ルートレイアウト (Jsonnet)
├── page.libsonnet             # ホームページ
├── dashboard/
│   ├── layout.libsonnet       # ダッシュボードレイアウト
│   └── page.libsonnet         # ダッシュボードページ
├── blog/
│   └── [slug]/
│       └── page.libsonnet     # 動的ブログ記事ページ
├── (auth)/
│   └── login/
│       └── page.libsonnet     # 認証ページ（ルートグループ）
└── api/
    └── users.libsonnet        # REST API定義

lib/
├── database.libsonnet         # データベース設定ライブラリ
├── auth.libsonnet             # 認証設定ライブラリ
└── api.libsonnet              # API設定ライブラリ

kotoba.libsonnet               # メイン設定ファイル
README.md                       # このファイル
```

## 特徴

### 🏗️ フルスタックアーキテクチャ
- **フロントエンド**: Next.js風App Router + React Components
- **バックエンド**: REST/GraphQL API + ミドルウェアシステム
- **データベース**: PostgreSQL/MySQL/SQLite + MongoDB/Redis
- **認証**: JWT/OAuth2/LDAP/SAML + RBAC/ABAC
- **デプロイ**: 静的ホスティング/CDN/サーバーサイドレンダリング

### 📁 ファイルベースルーティング
- `app/page.libsonnet` → `/` ルート
- `app/dashboard/page.libsonnet` → `/dashboard` ルート
- `app/blog/[slug]/page.libsonnet` → `/blog/:slug` 動的ルート
- `app/api/users.libsonnet` → `/api/users` REST API

### 🎨 コンポーネントシステム
- **Server Components**: デフォルト、サーバーサイドレンダリング
- **Client Components**: インタラクティブなコンポーネント
- **Layout Components**: 共有レイアウト
- **API Components**: サーバーサイドAPIハンドラー

### 🔧 Jsonnet設定システム
- **型安全**: Jsonnetの型システムによる設定検証
- **再利用性**: ライブラリ関数による設定共有
- **動的生成**: プログラムによる設定生成
- **環境対応**: 環境別の設定切り替え

### 📦 API機能
- **REST API**: 完全なCRUD操作 + バリデーション
- **GraphQL**: スキーマ定義 + リゾルバー
- **WebSocket**: リアルタイム通信
- **ミドルウェア**: CORS/認証/レート制限/キャッシュ

### 🗄️ データベース統合
- **ORM**: モデル定義 + リレーションシップ
- **マイグレーション**: スキーマバージョン管理
- **クエリビルダ**: 型安全なクエリ構築
- **コネクションプール**: 高性能接続管理

## ビルドと実行

```bash
# Web Frameworkビルド
cargo run --example frontend_app -- --build

# 開発サーバー起動 (デフォルトポート: 3000)
cargo run --example frontend_app -- --dev

# 特定のルートをレンダリング
cargo run --example frontend_app -- /dashboard

# APIテスト
curl http://localhost:3000/api/users
```

## Jsonnet設定例

### コンポーネント定義 (layout.libsonnet)
```jsonnet
{
  component: 'RootLayout',
  type: 'layout',
  environment: 'server',

  props: {
    title: 'Kotoba App',
    lang: 'ja',
  },

  children: [
    {
      component: 'Navigation',
      type: 'client',
      props: {
        items: [
          { label: 'Home', href: '/' },
          { label: 'Dashboard', href: '/dashboard' },
          { label: 'Blog', href: '/blog' },
        ],
      },
    },
    {
      component: 'Content',
      type: 'layout',
      children: [],  // ページコンポーネントがここに挿入される
    },
  ],

  imports: [
    {
      module: 'react',
      specifiers: ['useState', 'useEffect'],
    },
  ],

  metadata: {
    description: 'Root layout for the entire application',
  },
}
```

### API定義 (api/users.libsonnet)
```jsonnet
{
  // GET /api/users - ユーザーリスト取得
  get: {
    path: '/api/users',
    method: 'GET',
    handler: {
      function_name: 'getUsers',
      is_async: true,
      timeout_ms: 5000,
    },
    middlewares: ['auth', 'cors'],
    response_format: 'JSON',
    parameters: {
      query_params: [
        {
          name: 'limit',
          param_type: 'Integer',
          required: false,
          default_value: 10,
          validation: {
            min_value: 1,
            max_value: 100,
          },
        },
      ],
    },
    metadata: {
      description: 'Get users list with pagination',
      tags: ['users'],
      rate_limit: {
        requests: 100,
        window_seconds: 60,
        strategy: 'SlidingWindow',
      },
    },
  },
}
```

### データベース設定 (kotoba.libsonnet)
```jsonnet
local db = import './lib/database.libsonnet';

{
  database: db.postgres {
    connection_string: 'postgresql://user:pass@localhost/kotoba_app',
    models: [
      db.model {
        name: 'User',
        table_name: 'users',
        fields: [
          db.field.id('id'),
          db.field.string('email', 255) {
            unique: true,
            nullable: false,
          },
          db.field.string('password_hash', 255) {
            nullable: false,
          },
          db.field.timestamps(),
        ],
      },
    ],
  },
}
```

## ルーティング例

| ファイルパス | URL | 説明 |
|-------------|-----|------|
| `app/page.libsonnet` | `/` | ホームページ |
| `app/dashboard/page.libsonnet` | `/dashboard` | ダッシュボード |
| `app/blog/[slug]/page.libsonnet` | `/blog/hello` | 動的ルート |
| `app/api/users.libsonnet` | `/api/users` | REST API |

## Jsonnetライブラリ

### database.libsonnet
データベース設定用のユーティリティ関数を提供：
- `db.postgres`, `db.mysql`, `db.sqlite` - データベース設定
- `db.model` - モデル定義
- `db.field.*` - フィールドタイプヘルパー
- `db.migration` - マイグレーション定義

### auth.libsonnet
認証設定用のユーティリティ関数を提供：
- `auth.jwt`, `auth.oauth2` - 認証プロバイダー設定
- `auth.permissions` - 権限定義
- `auth.policies` - アクセスポリシー

### api.libsonnet
API設定用のユーティリティ関数を提供：
- `api.rest.crud()` - RESTful CRUD API生成
- `api.graphql.*` - GraphQL設定
- `api.websocket.*` - WebSocket設定

## 環境別設定

Jsonnetの機能を活用して環境別の設定を管理：

```jsonnet
// 環境変数に基づく設定切り替え
local env = std.extVar('ENVIRONMENT');

{
  database: if env == 'production' then
    db.postgres { connection_string: 'prod-db-url' }
  else
    db.sqlite { connection_string: ':memory:' },

  server: {
    port: if env == 'production' then 80 else 3000,
    debug: env != 'production',
  },
}
```
| `app/(auth)/login/page.kotoba` | `/login` | ルートグループ |

## レイアウト継承

```
app/layout.kotoba (ルートレイアウト)
├── app/page.kotoba (ホームページ)
├── app/dashboard/layout.kotoba (ダッシュボードレイアウト)
│   └── app/dashboard/page.kotoba (ダッシュボードページ)
└── app/blog/[slug]/page.kotoba (ブログ記事)
```

各ページは親レイアウトを自動的に継承します。

## ビルド最適化

- **コード分割**: 自動的なチャンク分割
- **Tree Shaking**: 未使用コードの除去
- **ミニファイ**: バンドルサイズ最適化
- **圧縮**: Gzip/Brotli圧縮

## デプロイメント

```yaml
deployment:
  strategy: static_hosting
  cdn:
    provider: vercel
    distribution_id: "dist_kotoba_app"
```

Vercel、Netlify、CloudFlareなどに対応。

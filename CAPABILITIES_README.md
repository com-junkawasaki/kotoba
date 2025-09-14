# Kotoba Capabilities - .kotoba 言語での機能ベースセキュリティ

Kotobaの `.kotoba` ファイルでDenoに似た**capability-based security**（機能ベースセキュリティ）を定義できます。このシステムは、伝統的なロールベースアクセス制御（RBAC）よりも細かい権限管理を提供します。

## 🎯 概要

Capabilities（機能）は、特定のアクションを特定のリソースに対して実行できるという**明示的な権限**を表します。RBACとは異なり、機能は以下のような特徴を持ちます：

- **明示的な付与**: 権限は明示的に付与される必要があります
- **最小権限の原則**: 必要な権限のみを付与
- **機能減衰**: より安全な操作のために権限を制限可能
- **細かい制御**: リソースタイプ、アクション、スコープによる詳細制御

## 📄 .kotobaファイルでの定義

### 基本構造

```jsonnet
{
  config: {
    type: "config",
    name: "SecureApp",
  },

  // セキュリティ設定
  security: {
    capabilities: {
      enable_logging: true,
      enable_auditing: true,
    }
  },

  // プリンシパル定義（機能を持つ主体）
  principals: [
    {
      id: "user_123",
      name: "John Doe",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "posts:*"
        },
        {
          resource_type: "Graph",
          action: "Write",
          scope: "posts:owned"
        }
      ]
    }
  ],

  // リソース定義
  resources: [
    {
      type: "graph",
      id: "posts",
      actions: ["Read", "Write", "Create"],
      scopes: ["*", "owned:*", "public:*"]
    }
  ]
}
```

### リソースタイプ

- `"Graph"`: グラフデータベース操作
- `"FileSystem"`: ファイルシステムアクセス
- `"Network"`: ネットワークアクセス
- `"Environment"`: 環境変数アクセス
- `"System"`: システム操作
- `"Plugin"`: プラグイン操作
- `"Query"`: クエリ実行
- `"Admin"`: 管理者操作
- `"User"`: ユーザー管理

### アクション

- `"Read"`: 読み取りアクセス
- `"Write"`: 書き込みアクセス
- `"Execute"`: 実行アクセス
- `"Delete"`: 削除アクセス
- `"Create"`: 作成アクセス
- `"Update"`: 更新アクセス
- `"Admin"`: 管理者アクセス

## 🚀 使用例

### 1. 基本的な機能定義

**basic_capabilities.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "BasicCapabilities",
  },

  security: {
    capabilities: {
      enable_logging: true,
    }
  },

  principals: [
    {
      id: "analyst",
      name: "Data Analyst",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "analytics:*"
        },
        {
          resource_type: "Query",
          action: "Execute",
          scope: "read_only"
        }
      ]
    },
    {
      id: "content_writer",
      name: "Content Writer",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "posts:*"
        },
        {
          resource_type: "Graph",
          action: "Write",
          scope: "posts:owned"
        },
        {
          resource_type: "Graph",
          action: "Create",
          scope: "posts:*"
        }
      ]
    }
  ],

  resources: [
    {
      type: "graph",
      id: "analytics",
      actions: ["Read"],
      scopes: ["*"]
    },
    {
      type: "graph",
      id: "posts",
      actions: ["Read", "Write", "Create"],
      scopes: ["*", "owned:*"]
    }
  ]
}
```

### 2. HTTP API での機能ベース認可

**api_with_capabilities.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "APIServer",
    server: { host: "127.0.0.1", port: 3000 }
  },

  security: {
    capabilities: {
      enable_logging: true,
      enable_auditing: true,
    }
  },

  principals: [
    {
      id: "api_user",
      name: "API User",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "public:*"
        }
      ]
    }
  ],

  routes: [
    {
      method: "GET",
      pattern: "/api/posts",
      handler: "list_posts",
      required_capabilities: ["Graph:Read:public:*"]
    },
    {
      method: "POST",
      pattern: "/api/posts",
      handler: "create_post",
      required_capabilities: ["Graph:Write:owned:*", "Graph:Create:*"]
    }
  ],

  handlers: [
    {
      name: "list_posts",
      function: "execute_gql",
      parameters: {
        query: "MATCH (p:Post) WHERE p.public = true RETURN p.title, p.content",
        required_capabilities: ["Graph:Read:public:*"]
      }
    },
    {
      name: "create_post",
      function: "create_graph_node",
      parameters: {
        type: "Post",
        properties: ["title", "content", "author_id", "public"],
        required_capabilities: ["Graph:Create:*"]
      }
    }
  ]
}
```

### 3. 機能減衰（Attenuation）の使用

**attenuated_capabilities.kotoba**
```jsonnet
{
  config: {
    type: "config",
    name: "AttenuatedCapabilities",
  },

  security: {
    capabilities: {
      enable_logging: true,
    }
  },

  // プリセット機能セット
  capability_presets: {
    // 広範な管理者権限
    full_admin: [
      {
        resource_type: "Graph",
        action: "Read",
        scope: "*"
      },
      {
        resource_type: "Graph",
        action: "Write",
        scope: "*"
      },
      {
        resource_type: "System",
        action: "Admin",
        scope: "*"
      }
    ],

    // 制限された管理者権限（減衰）
    limited_admin: [
      {
        resource_type: "Graph",
        action: "Read",
        scope: "*"
      },
      {
        resource_type: "Graph",
        action: "Write",
        scope: "safe:*"  // 制限されたスコープ
      }
      // System:Admin は除外（減衰）
    ]
  },

  // 減衰ルール
  attenuation_rules: [
    {
      name: "safe_admin",
      source_preset: "full_admin",
      restrictions: [
        {
          resource_type: "Graph",
          action: "Write",
          scope: "safe:*"  // より制限されたスコープ
        },
        // System:Admin を完全に除外
      ]
    }
  ],

  principals: [
    {
      id: "safe_admin",
      name: "Safe Administrator",
      capabilities: [
        // 減衰された機能を使用
        {
          resource_type: "Graph",
          action: "Read",
          scope: "*"
        },
        {
          resource_type: "Graph",
          action: "Write",
          scope: "safe:*"
        }
      ]
    }
  ]
}
```

## 🔧 実行方法

```bash
# 機能ベースセキュリティを有効にして実行
kotoba run app.kotoba

# サーバーモードで起動
kotoba server --config secure_app.kotoba --port 3000

# 機能を検証
kotoba check app.kotoba --capabilities
```

## 🔒 セキュリティの利点

### 1. **明示的な権限付与**
- 必要な権限のみを明示的に宣言
- 暗黙的な権限付与を排除

### 2. **機能減衰**
- 広範な権限から安全な制限版を作成
- 信頼できない操作に制限された権限を提供

### 3. **スコープベース制御**
- パターン matching で詳細なアクセス制御
- `"users:*"`, `"posts:owned"`, `"public:*"` などの柔軟なスコープ

### 4. **監査可能性**
- すべての権限チェックをログに記録
- セキュリティイベントの追跡

## 🛡️ Deno との比較

| 特徴 | Deno | Kotoba Capabilities |
|------|------|-------------------|
| ファイルアクセス | `--allow-read` | `FileSystem:Read:*` |
| ネットワーク | `--allow-net` | `Network:*:*` |
| 環境変数 | `--allow-env` | `Environment:Read:*` |
| 実行権限 | `--allow-run` | `System:Execute:*` |
| スコープ | パス/ホスト制限 | 柔軟なパターンマッチ |

## 📚 関連ドキュメント

- [Deno Permissions](https://deno.land/manual/basics/permissions)
- [Capability-based Security](https://en.wikipedia.org/wiki/Capability-based_security)
- [Principle of Least Privilege](https://en.wikipedia.org/wiki/Principle_of_least_privilege)

---

**Kotoba Capabilities** - Denoに似たセキュリティで、より安全な.kotobaアプリケーションを実現
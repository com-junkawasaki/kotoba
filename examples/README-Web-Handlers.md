# 🌐 Kotoba Web Handlers - Rustコード不要のウェブ開発

KotobaのWeb Handlerモジュールを活用して、**Rustコードを書かずに**完全なウェブアプリケーションを構築できます！

## 🎯 概要

### 実装されたWeb Handlerモジュール

1. **`web.rs`** - HTTPリクエスト/レスポンス処理、ルーティング、ミドルウェア
2. **`templates.rs`** - Tera/Handlebarsテンプレートエンジン統合
3. **`database.rs`** - PostgreSQL/MySQL/SQLite/Redisデータベース統合
4. **`auth.rs`** - JWT認証、パスワードハッシュ、セッション管理、RBAC
5. **`dev_server.rs`** - ホットリロード、ファイル監視、開発ツール

## 🚀 クイックスタート

### 1. シンプルなウェブアプリケーション

```jsonnet
{
  config: {
    name: "My Web App",
    port: 3000,
    database_url: "sqlite://app.db"
  },

  routes: {
    "GET /": {
      handler: "render_template",
      template: "<h1>Hello World!</h1>",
    },

    "GET /api/users": {
      handler: "json_response",
      data: { users: [] },
    }
  }
}
```

### 2. 高度なウェブアプリケーション

```jsonnet
{
  // アプリケーション設定
  config: {
    name: "Advanced App",
    port: 3000,
    database_url: "postgresql://user:pass@localhost/app",
    jwt_secret: "your-secret-key",
    cors_enabled: true,
  },

  // データベースモデル
  models: {
    users: {
      fields: {
        id: "integer primary key",
        username: "varchar(50) unique",
        email: "varchar(255) unique",
        password_hash: "varchar(255)",
        role: "varchar(20) default 'user'",
      }
    }
  },

  // ルート定義
  routes: {
    "GET /": {
      handler: "render_template",
      template: "home.html",
      middlewares: ["auth_optional"],
    },

    "POST /auth/login": {
      handler: "auth_login",
      middlewares: ["rate_limit"],
    },

    "GET /api/users": {
      handler: "database_query",
      query: "SELECT * FROM users",
      middlewares: ["auth_required"],
    }
  },

  // ミドルウェア定義
  middlewares: {
    auth_required: { type: "auth", required: true },
    rate_limit: { type: "rate_limit", requests: 100, window_seconds: 60 },
  }
}
```

## 🔧 利用可能な機能

### HTTPハンドラー

#### 基本HTTPメソッド
```jsonnet
routes: {
  "GET /users": { handler: "list_users" },
  "POST /users": { handler: "create_user" },
  "PUT /users/:id": { handler: "update_user", params: { id: "integer" } },
  "DELETE /users/:id": { handler: "delete_user" },
}
```

#### パラメータ処理
```jsonnet
"GET /users/:id/posts/:postId": {
  handler: "get_user_post",
  params: {
    id: "integer",
    postId: "integer"
  }
}
```

#### クエリパラメータ
```jsonnet
"GET /posts": {
  handler: "list_posts",
  query_params: {
    page: "integer default 1",
    limit: "integer default 10",
    published: "boolean default true"
  }
}
```

### レスポンスタイプ

#### HTMLテンプレート
```jsonnet
"GET /": {
  handler: "render_template",
  template: "home.html",
  context: {
    title: "Welcome",
    user: session.user
  }
}
```

#### JSON API
```jsonnet
"GET /api/users": {
  handler: "json_response",
  data: {
    users: database.query("SELECT * FROM users"),
    total: database.count("users")
  }
}
```

#### 静的ファイル
```jsonnet
"GET /static/*": {
  handler: "serve_static",
  root_dir: "public"
}
```

### データベース統合

#### SQLクエリ実行
```jsonnet
handlers: {
  list_users: {
    implementation: |||
      local users = database.query("SELECT * FROM users WHERE active = $1", [true]);
      return { success: true, users: users };
    |||
  }
}
```

#### CRUD操作
```jsonnet
create_user: {
  implementation: |||
    local user_data = {
      username: request.body.username,
      email: request.body.email,
      password_hash: auth.hash_password(request.body.password)
    };

    local new_user = database.insert("users", user_data);
    return { success: true, user: new_user };
  |||
}
```

#### トランザクション
```jsonnet
transfer_money: {
  implementation: |||
    local tx = database.begin_transaction();

    // 金額チェック
    local balance = database.query_single("SELECT balance FROM accounts WHERE id = $1", [from_account]);
    if balance.amount < amount then
      tx.rollback();
      return { success: false, message: "Insufficient funds" };
    end;

    // 金額移動
    database.execute_in_transaction(tx, "UPDATE accounts SET balance = balance - $1 WHERE id = $2", [amount, from_account]);
    database.execute_in_transaction(tx, "UPDATE accounts SET balance = balance + $1 WHERE id = $2", [amount, to_account]);

    tx.commit();
    return { success: true, message: "Transfer completed" };
  |||
}
```

### 認証・認可

#### JWT認証
```jsonnet
auth_login: {
  implementation: |||
    local user = database.query_single("SELECT * FROM users WHERE email = $1", [request.body.email]);

    if user == null then
      return { success: false, message: "User not found" };
    end;

    local valid = auth.verify_password(request.body.password, user.password_hash);
    if not valid then
      return { success: false, message: "Invalid password" };
    end;

    local token = auth.generate_jwt(user.id, user.role);
    return { success: true, token: token, user: user };
  |||
}
```

#### ミドルウェアベース認可
```jsonnet
middlewares: {
  admin_only: {
    type: "authorization",
    role: "admin",
    redirect_to: "/unauthorized"
  },

  ownership_required: {
    type: "ownership",
    resource_type: "post",
    user_field: "author_id"
  }
}
```

### テンプレートエンジン

#### Teraテンプレート
```jsonnet
<!-- templates/user_profile.html -->
{% extends "base.html" %}

{% block title %}{{ user.username }}'s Profile{% endblock %}

{% block content %}
<div class="profile">
  <h1>{{ user.username }}</h1>
  <p>Email: {{ user.email }}</p>
  <p>Joined: {{ user.created_at | date:"M j, Y" }}</p>

  {% if user.role == "admin" %}
    <div class="admin-badge">Admin</div>
  {% endif %}
</div>
{% endblock %}
```

#### テンプレートコンテキスト
```jsonnet
"GET /profile": {
  handler: "render_template",
  template: "user_profile.html",
  context: {
    user: session.user,
    posts: database.query("SELECT * FROM posts WHERE author_id = $1", [session.user.id]),
    is_admin: session.user.role == "admin"
  }
}
```

### 開発サーバー機能

#### ホットリロード
```jsonnet
dev_server: {
  watch_paths: ["templates", "static", "src"],
  ignored_paths: ["node_modules", ".git"],
  enable_hot_reload: true,
  livereload_port: 35729
}
```

#### 開発ツール
```jsonnet
// 自動的に利用可能
// - ファイル変更検知
// - 自動ブラウザリロード
// - デバッグログ
// - パフォーマンス監視
// - エラートラッキング
```

## 📊 アーキテクチャ

### リクエストフロー

```
HTTP Request → Middleware Chain → Route Handler → Response
     ↓               ↓               ↓              ↓
   Parsing      Authentication   Business Logic   Rendering
                                    ↓
                               Database/External APIs
```

### ミドルウェアチェーン

```jsonnet
middlewares: {
  cors: { type: "cors" },
  logging: { type: "logging", level: "info" },
  auth: { type: "auth", required: true },
  rate_limit: { type: "rate_limit", requests: 100 },
  cache: { type: "cache", strategy: "public" }
}
```

### エラーハンドリング

```jsonnet
error_handlers: {
  404: {
    handler: "render_template",
    template: "errors/404.html"
  },

  500: {
    handler: "render_template",
    template: "errors/500.html",
    context: {
      error: error.message,
      stack_trace: error.stack
    }
  }
}
```

## 🔒 セキュリティ機能

### 組み込みセキュリティ
- **CSRF対策**: 自動トークン生成・検証
- **XSS対策**: テンプレート自動エスケープ
- **SQLインジェクション対策**: パラメータ化クエリ
- **レート制限**: リクエスト頻度制御
- **セッション管理**: 安全なセッション処理

### 設定例
```jsonnet
security: {
  csrf_protection: true,
  secure_headers: true,
  rate_limiting: {
    enabled: true,
    requests_per_minute: 60
  },
  session: {
    secure: true,
    http_only: true,
    same_site: "strict"
  }
}
```

## 🚀 デプロイメント

### ローカル開発
```bash
# 開発サーバー起動
kotoba dev examples/comprehensive-web-app.kotoba

# ブラウザでアクセス
open http://localhost:3000
```

### 本番デプロイ
```jsonnet
deployment: {
  production: {
    provider: "docker",
    image: "my-web-app:latest",
    environment: {
      DATABASE_URL: "postgresql://prod:prod@db.example.com/app",
      JWT_SECRET: "production-secret-key"
    }
  }
}
```

## 📈 パフォーマンス最適化

### キャッシュ戦略
```jsonnet
caching: {
  static_files: {
    max_age: 31536000, // 1年
    immutable: true
  },

  api_responses: {
    strategy: "time_based",
    ttl: 300 // 5分
  },

  database_queries: {
    enabled: true,
    ttl: 60 // 1分
  }
}
```

### データベース最適化
```jsonnet
database: {
  connection_pool: {
    max_size: 20,
    min_idle: 5
  },

  query_optimization: {
    enable_indexes: true,
    enable_query_cache: true
  }
}
```

## 🧪 テスト機能

### ユニットテスト
```jsonnet
tests: {
  unit: [
    {
      name: "User registration validation",
      test_cases: [
        {
          input: { username: "test", password: "Password123!" },
          expected: { success: true }
        }
      ]
    }
  ]
}
```

### 統合テスト
```jsonnet
integration_tests: [
  {
    name: "Full user registration flow",
    steps: [
      "POST /auth/register with valid data",
      "Verify email confirmation",
      "POST /auth/login",
      "GET /protected-resource with token"
    ]
  }
]
```

## 🎨 拡張性

### カスタムハンドラー
```jsonnet
handlers: {
  custom_logic: {
    implementation: |||
      // カスタムビジネスロジック
      local result = external_api.call("https://api.example.com/data");
      local processed = process_data(result);

      return { success: true, data: processed };
    |||
  }
}
```

### カスタムミドルウェア
```jsonnet
middlewares: {
  custom_auth: {
    type: "custom",
    implementation: |||
      if not request.headers.authorization then
        return { status: 401, message: "Unauthorized" };
      end;

      // カスタム認証ロジック
      local user = validate_token(request.headers.authorization);
      request.user = user;
    |||
  }
}
```

## 📚 使用例集

### 1. ブログアプリケーション
```jsonnet
{
  models: {
    posts: { fields: { title: "string", content: "text", author_id: "integer" } },
    comments: { fields: { post_id: "integer", content: "text", author_id: "integer" } }
  },

  routes: {
    "GET /posts": { handler: "list_posts" },
    "POST /posts": { handler: "create_post", middlewares: ["auth_required"] },
    "GET /posts/:id": { handler: "get_post" },
    "POST /posts/:id/comments": { handler: "add_comment" }
  }
}
```

### 2. Eコマースサイト
```jsonnet
{
  models: {
    products: { fields: { name: "string", price: "decimal", category: "string" } },
    orders: { fields: { user_id: "integer", total: "decimal", status: "string" } },
    cart_items: { fields: { user_id: "integer", product_id: "integer", quantity: "integer" } }
  },

  routes: {
    "GET /products": { handler: "list_products" },
    "POST /cart": { handler: "add_to_cart" },
    "POST /checkout": { handler: "create_order" },
    "GET /orders": { handler: "list_orders", middlewares: ["auth_required"] }
  }
}
```

### 3. ダッシュボードアプリケーション
```jsonnet
{
  routes: {
    "GET /dashboard": {
      handler: "dashboard_data",
      middlewares: ["auth_required"],
      template: "dashboard.html"
    },

    "GET /api/stats": {
      handler: "get_stats",
      response_type: "json",
      middlewares: ["auth_required"]
    }
  },

  realtime: {
    enabled: true,
    endpoints: ["/ws/dashboard"]
  }
}
```

---

**Kotoba Web Handlers**を使用することで、**Rustコードを書かずに**Jsonnetだけで本格的なウェブアプリケーションを構築できます。従来のフレームワークの複雑さを排除し、直感的で強力なウェブ開発体験を提供します！ 🎉

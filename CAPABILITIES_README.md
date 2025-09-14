# Kotoba Capabilities - 機能ベースセキュリティ

KotobaにDenoに似た**capability-based security**（機能ベースセキュリティ）を統合しました。このシステムは、伝統的なロールベースアクセス制御（RBAC）よりも細かい権限管理を提供します。

## 🎯 概要

Capabilities（機能）は、特定のアクションを特定のリソースに対して実行できるという**明示的な権限**を表します。RBACとは異なり、機能は以下のような特徴を持ちます：

- **明示的な付与**: 権限は明示的に付与される必要があります
- **最小権限の原則**: 必要な権限のみを付与
- **機能減衰**: より安全な操作のために権限を制限可能
- **細かい制御**: リソースタイプ、アクション、スコープによる詳細制御

## 🏗️ アーキテクチャ

### 主要コンポーネント

```rust
// 機能の定義
pub struct Capability {
    pub resource_type: ResourceType,  // リソースタイプ
    pub action: Action,              // アクション
    pub scope: Option<String>,       // スコープ（制限）
    pub conditions: Option<HashMap<String, Value>>,  // 追加条件
}

// 機能セット
pub struct CapabilitySet {
    pub capabilities: Vec<Capability>,
    pub metadata: Option<HashMap<String, Value>>,
}

// 機能サービス
pub struct CapabilityService {
    // 機能の管理と検証
}
```

### リソースタイプ

```rust
pub enum ResourceType {
    Graph,          // グラフデータベース操作
    FileSystem,     // ファイルシステムアクセス
    Network,        // ネットワークアクセス
    Environment,    // 環境変数
    System,         // システム操作
    Plugin,         // プラグイン操作
    Query,          // クエリ実行
    Admin,          // 管理者操作
    User,           // ユーザー管理
    Custom(String), // カスタムリソース
}
```

### アクション

```rust
pub enum Action {
    Read,           // 読み取り
    Write,          // 書き込み
    Execute,        // 実行
    Delete,         // 削除
    Create,         // 作成
    Update,         // 更新
    Admin,          // 管理者アクセス
    Custom(String), // カスタムアクション
}
```

## 🚀 使用方法

### 基本的な使用例

```rust
use kotoba_security::capabilities::*;

// 機能サービスを作成
let service = CapabilityService::new();

// 機能を作成
let read_users = Capability::new(
    ResourceType::Graph,
    Action::Read,
    Some("users:*".to_string())
);

// 機能セットを作成
let mut cap_set = CapabilitySet::new();
cap_set.add_capability(read_users);

// 権限チェック
let allowed = service.check_capability(
    &cap_set,
    &ResourceType::Graph,
    &Action::Read,
    Some("users:123")
);
assert!(allowed);  // 許可される
```

### プリンシパルと認可

```rust
// セキュリティサービスでプリンシパルを作成
let principal = security_service.create_principal_with_capabilities(
    "user-123".to_string(),
    cap_set,
    vec!["user".to_string()],
    vec!["read:*".to_string()],
    HashMap::new(),
);

// リソースを作成
let resource = security_service.create_resource(
    ResourceType::Graph,
    Action::Read,
    Some("users:123".to_string()),
    HashMap::new(),
);

// 認可チェック
let result = security_service.check_authorization(&principal, &resource);
assert!(result.allowed);
```

### 機能減衰（Attenuation）

```rust
// 広範な機能を減衰させて安全にする
let broad_cap = Capability::new(ResourceType::Graph, Action::Write, None);
let attenuated = broad_cap.attenuate(Some("owned:*".to_string()));

// 元の機能: すべてのグラフに書き込み可能
// 減衰後の機能: 所有するデータのみ書き込み可能
```

### プリセット機能セット

```rust
// 一般的なユースケース用のプリセット
let readonly = CapabilityService::create_preset_capability_set(
    PresetCapabilitySet::ReadOnly
);

let admin = CapabilityService::create_preset_capability_set(
    PresetCapabilitySet::Admin
);
```

## 📄 .kotobaファイルでの設定

機能ベースセキュリティを.kotobaファイルで設定できます：

```jsonnet
{
  // セキュリティ設定
  security: {
    capabilities: {
      enable_logging: true,
      enable_auditing: true,
    }
  },

  // プリンシパル定義
  principals: [
    {
      id: "admin-user",
      capabilities: [
        {
          resource_type: "Graph",
          action: "Read",
          scope: "*"
        },
        {
          resource_type: "Admin",
          action: "Admin",
          scope: "*"
        }
      ]
    }
  ],

  // リソース定義
  resources: [
    {
      type: "graph",
      id: "users",
      actions: ["Read", "Write"],
      scopes: ["*", "owned:*"]
    }
  ]
}
```

## 🔒 セキュリティの利点

### 1. **最小権限の原則**
- 必要な権限のみを明示的に付与
- 過剰な権限を避ける

### 2. **機能減衰**
- 広範な権限から安全な制限版を作成
- 信頼できないコードに制限された権限を提供

### 3. **細かい制御**
- リソースタイプ、アクション、スコープによる詳細制御
- 複雑なアクセスパターンを表現可能

### 4. **監査可能性**
- すべての権限チェックをログに記録
- セキュリティイベントの追跡

## 🛡️ Denoとの比較

KotobaのcapabilityシステムはDenoの権限モデルに着想を得ています：

| 特徴 | Deno | Kotoba Capabilities |
|------|------|-------------------|
| ファイルアクセス | `--allow-read` | `FileSystem::Read` |
| ネットワーク | `--allow-net` | `Network::*` |
| 環境変数 | `--allow-env` | `Environment::*` |
| 実行権限 | `--allow-run` | `System::Execute` |
| スコープ | パス/ホスト制限 | 柔軟なスコープパターン |

## 🧪 デモと例

### 実行方法

```bash
# 機能デモを実行
cargo run --example capabilities_demo

# .kotoba設定をテスト
jsonnet eval examples/capabilities_example.kotoba
```

### デモ内容

1. **基本機能**: 機能の作成と検証
2. **機能セット**: 機能の集合操作
3. **プリンシパル**: ユーザー/サービスと権限
4. **機能減衰**: 権限の安全な制限
5. **プリセット**: 一般的な権限セット

## 🔧 API リファレンス

### CapabilityService

- `check_capability()`: 権限チェック
- `grant_capabilities()`: 機能付与
- `revoke_capabilities()`: 機能剥奪
- `attenuate_capabilities()`: 機能減衰

### CapabilitySet

- `add_capability()`: 機能追加
- `remove_capability()`: 機能削除
- `allows()`: 権限確認
- `union()` / `intersection()`: 集合演算

### Capability

- `matches()`: リソース/アクションとの一致チェック
- `attenuate()`: 機能の制限

## 🚀 次のステップ

1. **ポリシーベースアクセス制御**: 属性ベースのアクセス制御との統合
2. **機能委譲**: 機能の安全な委譲メカニズム
3. **動的機能**: 実行時の機能付与/剥奪
4. **機能証明**: 機能の暗号的証明

## 📚 関連ドキュメント

- [Deno Permissions](https://deno.land/manual/basics/permissions)
- [Capability-based Security](https://en.wikipedia.org/wiki/Capability-based_security)
- [Principle of Least Privilege](https://en.wikipedia.org/wiki/Principle_of_least_privilege)

---

**Kotoba Capabilities** - Denoに似たセキュリティで、より安全なグラフ処理を実現

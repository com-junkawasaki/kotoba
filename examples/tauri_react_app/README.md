# Kotoba Tauri React App Example

Kotobaのプロセスネットワークグラフモデルに基づいた、Tauri + React + TypeScriptのデスクトップアプリケーション例です。

## 特徴

- **.kotobaファイルベースのUI定義**: UIコンポーネントをJSON Lines形式の.kotobaファイルで定義
- **宣言的なUI**: Reactコンポーネントを.kotobaファイルから動的に生成
- **Kotoba Graph Engine統合**: Rustベースのグラフデータベースエンジンを直接操作
- **モダンなデスクトップアプリ**: Tauriによるクロスプラットフォーム対応

## アーキテクチャ

```
.kotobaファイル (UI定義)
    ↓
KotobaParser (パース)
    ↓
ComponentBuilder (React生成)
    ↓
React Components (レンダリング)
    ↓
Tauri (デスクトップアプリ)
    ↓
Kotoba Engine (グラフ操作)
```

## ファイル構造

```
tauri_react_app/
├── app.kotoba              # メインUI定義ファイル
├── src/
│   ├── main.tsx           # Reactエントリーポイント
│   ├── App.tsx            # メインアプリコンポーネント
│   ├── styles.css         # グローバルスタイル
│   └── utils/
│       ├── kotobaParser.ts    # .kotobaファイルパーサー
│       └── componentBuilder.tsx # Reactコンポーネントビルダー
├── src-tauri/
│   ├── src/main.rs        # Tauriバックエンド (Rust)
│   ├── Cargo.toml         # Rust依存関係
│   └── tauri.conf.json    # Tauri設定
└── package.json           # Node.js依存関係
```

## .kotobaファイルの構造

UIコンポーネントはJSON Lines形式で定義されます：

```json
{"type": "component", "name": "Header", "component_type": "header", "props": {"title": "My App"}}
{"type": "component", "name": "Button", "component_type": "button", "props": {"text": "Click me", "onClick": "handleClick"}}
{"type": "handler", "name": "handleClick", "function": "handleClick"}
{"type": "state", "name": "counter", "initial": 0}
```

## ビルドと実行

### 前提条件

- Rust (最新安定版)
- Node.js (v16以上)
- npm または yarn

### インストール

```bash
# 依存関係のインストール
npm install

# Tauri CLIのインストール (まだインストールしていない場合)
npm install -g @tauri-apps/cli
```

### 開発モード

```bash
# React開発サーバーの起動
npm run dev

# Tauriアプリの起動 (別ターミナル)
cd src-tauri && cargo tauri dev
```

### 本番ビルド

```bash
# Tauriアプリのビルド
cd src-tauri && cargo tauri build
```

## UIコンポーネント

### サポートされるコンポーネントタイプ

- `layout`: アプリ全体のレイアウト
- `header`: ヘッダーセクション
- `main`: メインコンテンツエリア
- `section`: セクション分割
- `button`: ボタン
- `input`: テキスト入力
- `textarea`: 複数行テキスト入力
- `form`: フォーム
- `div`: 汎用コンテナ
- `aside`: サイドバー
- `nav`: ナビゲーション
- `nav-item`: ナビゲーション項目
- `stat`: 統計情報表示
- `footer`: フッター
- `status`: ステータス表示

### イベント処理

イベントハンドラーはTauriコマンドを通じてRustバックエンドと通信します：

```typescript
builder.registerHandler('createGraph', async () => {
  const result = await window.__TAURI__.invoke('create_graph');
  // 結果処理
});
```

## Kotoba Engine統合

このアプリは以下のKotoba機能を活用しています：

- **Graph Operations**: グラフの作成、頂点/辺の追加
- **Query Execution**: GQLクエリの実行
- **Statistics**: グラフ統計情報の取得
- **MVCC**: マルチバージョン同時実行制御
- **Merkle DAG**: 永続化とバージョン管理

## カスタマイズ

### 新しいUIコンポーネントの追加

1. `app.kotoba`ファイルにコンポーネント定義を追加
2. `componentBuilder.tsx`の`ComponentTypeMap`にマッピングを追加
3. 必要に応じてCSSスタイルを追加

### 新しいTauriコマンドの追加

1. `src-tauri/src/main.rs`にコマンドを実装
2. `invoke_handler`にコマンドを登録
3. React側から`window.__TAURI__.invoke()`で呼び出し

## ライセンス

このプロジェクトはKotobaプロジェクトの一部です。

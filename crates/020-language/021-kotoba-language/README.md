# Kotoba Language - Unified Language Processing

Kotoba Languageは、すべての言語機能を統合的に提供するクレートです。graphをプログラミング言語として扱うための統一APIを提供します。

## Overview

Kotoba Languageは、以下の言語機能を統一的なAPIで提供します：

- **Kotobas**: HTTP設定言語のパース
- **Jsonnet**: 設定言語の評価
- **TypeScript変換**: .kotoba から TypeScriptへの変換
- **Formatter**: コードフォーマット
- **Linter**: コード解析
- **REPL**: 対話型実行環境
- **WASM**: WebAssembly対応

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Kotoba Language                          │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Unified Language Processor                             │ │
│  │  - LanguageType enum                                    │ │
│  │  - LanguageProcessor trait                              │ │
│  │  - KotobaLanguageProcessor impl                        │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Language Features (Optional)                           │ │
│  │  - kotoba-kotobas (HTTP config parser)                  │ │
│  │  - kotoba-jsonnet (Jsonnet evaluator)                   │ │
│  │  - kotoba2tsx (TypeScript converter)                   │ │
│  │  - kotoba-formatter (code formatter)                   │ │
│  │  - kotoba-linter (code analyzer)                       │ │
│  │  - kotoba-repl (REPL environment)                      │ │
│  │  - kotobas-wasm (WASM support)                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Usage

### Basic Usage

```rust
use kotoba_language::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create language processor
    let processor = KotobaLanguageProcessor::new();

    // Process Kotobas HTTP configuration
    let http_config = processor.process(
        LanguageType::Kotobas,
        "server { listen 8080; }"
    ).await?;

    // Evaluate Jsonnet configuration
    let jsonnet_result = processor.process(
        LanguageType::Jsonnet,
        "{ server: { port: 8080 } }"
    ).await?;

    // Format code
    let formatted = processor.format(
        LanguageType::Formatter,
        "fn main() { println!(\"hello\") }"
    ).await?;

    Ok(())
}
```

### Custom Configuration

```rust
use kotoba_language::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom configuration
    let config = LanguageConfig {
        features: vec![LanguageType::Kotobas, LanguageType::Jsonnet],
        options: HashMap::new(),
    };

    let processor = KotobaLanguageProcessor::with_config(config);

    // Only Kotobas and Jsonnet are available
    let result = processor.process(LanguageType::Kotobas, "content").await?;

    Ok(())
}
```

## Features

### Feature Flags

- `formatter`: Enable code formatting
- `jsonnet`: Enable Jsonnet evaluation
- `kotobas`: Enable HTTP configuration parsing
- `linter`: Enable code analysis
- `repl`: Enable REPL environment
- `tsx`: Enable TypeScript conversion
- `wasm`: Enable WASM support
- `full`: Enable all features

### Language Types

- `LanguageType::Kotobas`: HTTP設定言語
- `LanguageType::Jsonnet`: 設定言語
- `LanguageType::TypeScript`: TypeScript変換
- `LanguageType::Formatter`: コードフォーマット
- `LanguageType::Linter`: コード解析
- `LanguageType::Repl`: REPL環境
- `LanguageType::Wasm`: WASM対応

## Integration

Kotoba Languageは、既存の言語機能クレートを統合的に提供します：

- `kotoba-kotobas`: HTTP設定パーサー
- `kotoba-jsonnet`: Jsonnet評価器
- `kotoba2tsx`: TypeScript変換器
- `kotoba-formatter`: コードフォーマッター
- `kotoba-linter`: コードリンター
- `kotoba-repl`: REPL環境
- `kotobas-wasm`: WASM対応

## Roadmap

### Phase 1: Integration Foundation ✅
- [x] Unified API design
- [x] LanguageProcessor trait
- [x] Feature flag system
- [x] Basic implementation

### Phase 2: Feature Integration
- [ ] Full kotobas integration
- [ ] Full jsonnet integration
- [ ] Full tsx integration
- [ ] Full formatter integration
- [ ] Full linter integration

### Phase 3: Advanced Features
- [ ] Language server integration
- [ ] REPL integration
- [ ] WASM integration
- [ ] Performance optimization

## License

Licensed under MIT OR Apache-2.0

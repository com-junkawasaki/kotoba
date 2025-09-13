{
  // Kotobaライブラリ設定 - 実行可能なコンポーネント定義
  // dag.jsonnetのプロセスネットワークを実際に実行可能な形で構成

  // ==========================================
  // ライブラリ設定
  // ==========================================

  library: {
    name: 'kotoba',
    version: '0.1.0',
    description: 'GP2系グラフ書換え言語 - ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム',
    authors: ['jun784'],
    license: 'MIT',
    repository: 'https://github.com/jun784/kotoba',
    keywords: ['graph', 'database', 'query', 'rewrite', 'gql', 'gp2'],
  },

  // ==========================================
  // 実行可能コンポーネント定義
  // ==========================================

  executables: {
    // メインCLIツール
    'kotoba-cli': {
      name: 'kotoba-cli',
      description: 'Kotobaコマンドラインインターフェース',
      source: 'src/main.rs',
      dependencies: ['kotoba-lib'],
      features: ['cli', 'serde', 'tokio'],
    },

    // テスト実行可能ファイル
    'kotoba-test': {
      name: 'kotoba-test',
      description: 'Kotoba統合テスト実行ツール',
      source: 'src/bin/test.rs',
      dependencies: ['kotoba-lib'],
      features: ['test', 'serde', 'tokio'],
    },

    // ベンチマークツール
    'kotoba-bench': {
      name: 'kotoba-bench',
      description: 'Kotobaベンチマーク実行ツール',
      source: 'src/bin/bench.rs',
      dependencies: ['kotoba-lib'],
      features: ['bench', 'serde', 'tokio', 'criterion'],
    },
  },

  // ==========================================
  // ライブラリコンポーネント定義
  // ==========================================

  components: {
    // コアライブラリ
    'kotoba-core': {
      name: 'kotoba-core',
      description: 'Kotobaコアライブラリ',
      source: 'src/lib.rs',
      type: 'rlib',
      features: {
        default: ['types', 'ir', 'graph'],
        full: ['types', 'ir', 'graph', 'storage', 'execution', 'planner', 'rewrite'],
        types: [],
        ir: ['types'],
        graph: ['types', 'ir'],
        storage: ['types', 'graph'],
        execution: ['types', 'ir', 'graph', 'storage'],
        planner: ['types', 'ir', 'graph'],
        rewrite: ['types', 'ir', 'graph', 'storage'],
      },
    },

    // プラグインシステム
    'kotoba-plugins': {
      name: 'kotoba-plugins',
      description: 'Kotobaプラグインシステム',
      source: 'src/plugins/mod.rs',
      type: 'rlib',
      dependencies: ['kotoba-core'],
      features: ['plugin-loader'],
    },

    // FFIバインディング
    'kotoba-ffi': {
      name: 'kotoba-ffi',
      description: 'Kotoba FFIバインディング',
      source: 'src/ffi/mod.rs',
      type: 'cdylib',
      dependencies: ['kotoba-core'],
      features: ['ffi'],
    },

    // WASMバインディング
    'kotoba-wasm': {
      name: 'kotoba-wasm',
      description: 'Kotoba WebAssemblyバインディング',
      source: 'src/wasm/mod.rs',
      type: 'cdylib',
      dependencies: ['kotoba-core'],
      features: ['wasm'],
      target: 'wasm32-unknown-unknown',
    },
  },

  // ==========================================
  // 依存関係設定
  // ==========================================

  dependencies: {
    // ランタイム依存
    runtime: {
      'serde': { version: '1.0', features: ['derive'] },
      'serde_json': '1.0',
      'uuid': { version: '1.0', features: ['v4', 'serde'] },
      'sha2': '0.10',
      'thiserror': '2.0',
      'tokio': { version: '1.0', features: ['full'] },
      'parking_lot': '0.12',
      'dashmap': '6.0',
      'anyhow': '1.0',
      'log': '0.4',
      'env_logger': '0.11',
    },

    // 開発時依存
    dev: {
      'criterion': { version: '0.5', features: ['html_reports'] },
      'proptest': '1.0',
      'quickcheck': '1.0',
      'rand': '0.8',
    },

    // ビルド時依存
    build: {
      'cc': '1.0',
      'bindgen': '0.69',
    },

    // ドキュメント生成依存
    docs: {
      'rustdoc': 'nightly',
    },
  },

  // ==========================================
  // ビルド設定
  // ==========================================

  build: {
    // Rust設定
    rust: {
      edition: '2021',
      version: '>=1.70.0',
    },

    // ターゲット設定
    targets: [
      'x86_64-unknown-linux-gnu',
      'x86_64-apple-darwin',
      'x86_64-pc-windows-msvc',
      'aarch64-apple-darwin',
      'wasm32-unknown-unknown',
    ],

    // 最適化設定
    optimization: {
      release: {
        opt_level: 3,
        debug: false,
        lto: true,
        codegen_units: 1,
        panic: 'abort',
      },
      debug: {
        opt_level: 0,
        debug: true,
        overflow_checks: true,
      },
    },

    // リンカー設定
    linker: {
      rpath: true,
      strip: true,
    },
  },

  // ==========================================
  // テスト設定
  // ==========================================

  tests: {
    // ユニットテスト
    unit: {
      pattern: 'src/**/*.rs',
      exclude: ['src/bin/**', 'src/main.rs'],
      features: ['test'],
    },

    // 統合テスト
    integration: {
      pattern: 'tests/**/*.rs',
      features: ['test', 'integration'],
    },

    // ベンチマーク
    benchmarks: {
      pattern: 'benches/**/*.rs',
      features: ['bench'],
    },

    // ドクトest
    doctest: {
      enabled: true,
      features: ['doc'],
    },
  },

  // ==========================================
  // パッケージング設定
  // ==========================================

  packaging: {
    // Debianパッケージ
    debian: {
      name: 'kotoba',
      version: '0.1.0',
      architecture: 'amd64',
      description: 'GP2系グラフ書換え言語',
      maintainer: 'jun784 <jun784@example.com>',
      depends: ['libc6 (>= 2.15)'],
      files: {
        '/usr/bin/kotoba-cli': 'target/release/kotoba-cli',
        '/usr/lib/libkotoba.so': 'target/release/libkotoba.so',
        '/usr/share/doc/kotoba/': 'README.md LICENSE',
      },
    },

    // Dockerイメージ
    docker: {
      base_image: 'rust:1.70-slim',
      build_stages: [
        {
          name: 'build',
          commands: [
            'apt-get update',
            'apt-get install -y build-essential pkg-config',
            'cargo build --release',
          ],
        },
        {
          name: 'runtime',
          from: 'debian:stable-slim',
          commands: [
            'apt-get update',
            'apt-get install -y ca-certificates',
            'useradd -r -s /bin/false kotoba',
            'mkdir -p /var/lib/kotoba',
            'chown kotoba:kotoba /var/lib/kotoba',
          ],
        },
      ],
      exposed_ports: [8080, 8443],
      volumes: ['/var/lib/kotoba'],
      healthcheck: {
        test: ['CMD', 'curl', '-f', 'http://localhost:8080/health'],
        interval: '30s',
        timeout: '10s',
        retries: 3,
      },
    },

    // Homebrew Formula
    homebrew: {
      name: 'kotoba',
      desc: 'GP2系グラフ書換え言語',
      homepage: 'https://github.com/jun784/kotoba',
      url: 'https://github.com/jun784/kotoba/releases/download/v0.1.0/kotoba-0.1.0.tar.gz',
      sha256: 'PLACEHOLDER_SHA256',
      install: [
        'system "cargo", "install", "--root", prefix, "--path", "."',
        'bin.install "target/release/kotoba-cli"',
        'lib.install "target/release/libkotoba.dylib"',
      ],
      depends_on: [
        'rust' => ':build',
      ],
    },
  },

  // ==========================================
  // CI/CD設定
  // ==========================================

  ci: {
    // GitHub Actions
    github_actions: {
      workflows: {
        'ci.yml': {
          triggers: ['push', 'pull_request'],
          jobs: {
            test: {
              'runs-on': 'ubuntu-latest',
              steps: [
                { uses: 'actions/checkout@v3' },
                { uses: 'dtolnay/rust-toolchain@stable' },
                { run: 'cargo test' },
                { run: 'cargo test --doc' },
              ],
            },
            build: {
              'runs-on': 'ubuntu-latest',
              steps: [
                { uses: 'actions/checkout@v3' },
                { uses: 'dtolnay/rust-toolchain@stable' },
                { run: 'cargo build --release' },
                { run: 'cargo build --release --target wasm32-unknown-unknown' },
              ],
            },
            lint: {
              'runs-on': 'ubuntu-latest',
              steps: [
                { uses: 'actions/checkout@v3' },
                { uses: 'dtolnay/rust-toolchain@stable' },
                { run: 'cargo clippy -- -D warnings' },
                { run: 'cargo fmt --check' },
              ],
            },
          },
        },
        'release.yml': {
          triggers: [{ release: { types: ['published'] } }],
          jobs: {
            release: {
              'runs-on': 'ubuntu-latest',
              steps: [
                { uses: 'actions/checkout@v3' },
                { uses: 'dtolnay/rust-toolchain@stable' },
                { run: 'cargo build --release' },
                { uses: 'softprops/action-gh-release@v1', with: { files: 'target/release/kotoba-cli' } },
              ],
            },
          },
        },
      },
    },

    // GitLab CI
    gitlab_ci: {
      stages: ['test', 'build', 'deploy'],
      jobs: {
        test: {
          stage: 'test',
          image: 'rust:latest',
          script: ['cargo test', 'cargo test --doc'],
        },
        build: {
          stage: 'build',
          image: 'rust:latest',
          script: ['cargo build --release'],
          artifacts: {
            paths: ['target/release/kotoba-cli'],
            expire_in: '1 week',
          },
        },
        deploy: {
          stage: 'deploy',
          image: 'alpine:latest',
          script: ['echo "Deploying..."'],
          only: ['tags'],
        },
      },
    },
  },

  // ==========================================
  // ドキュメント設定
  // ==========================================

  documentation: {
    // Rustdoc設定
    rustdoc: {
      output_dir: 'target/doc',
      features: ['doc'],
      additional_args: [
        '--document-private-items',
        '--enable-index-page',
        '-Zunstable-options',
        '--generate-link-to-definition',
      ],
    },

    // MkDocs設定
    mkdocs: {
      site_name: 'Kotoba',
      site_description: 'GP2系グラフ書換え言語',
      site_author: 'jun784',
      site_url: 'https://kotoba.jun784.dev',
      theme: 'material',
      pages: [
        { Home: 'index.md' },
        { 'Getting Started': 'getting-started.md' },
        { API: 'api.md' },
        { Examples: 'examples.md' },
        { 'Contributing': 'contributing.md' },
      ],
      plugins: ['search', 'minify'],
    },
  },

  // ==========================================
  // ユーティリティ関数
  // ==========================================

  // 特定のターゲットの設定を取得
  get_target_config(target)::
    if std.objectHas(self.build.targets, target) then
      self.build.targets[target]
    else
      error 'Unsupported target: ' + target,

  // 特定のコンポーネントの設定を取得
  get_component_config(name)::
    if std.objectHas(self.components, name) then
      self.components[name]
    else
      error 'Component not found: ' + name,

  // 依存関係を解決
  resolve_dependencies(component, features)::
    local comp = self.get_component_config(component);
    local deps = if std.objectHas(comp, 'dependencies') then comp.dependencies else [];
    local resolved_features = if std.objectHas(comp.features, 'default') then
      comp.features.default
    else
      [];
    local all_features = std.set(features + resolved_features);
    {
      dependencies: deps,
      features: all_features,
    },

  // 設定の検証
  validate_config()::
    local components = std.objectValues(self.components);
    local executables = std.objectValues(self.executables);
    local all_names = [c.name for c in components] + [e.name for e in executables];
    local unique_names = std.set(all_names);
    if std.length(all_names) != std.length(unique_names) then
      error 'Duplicate component/executable names found'
    else
      'Configuration is valid',
}

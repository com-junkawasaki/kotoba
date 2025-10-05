{
  nodes: {
    // ==========================================
    // Rust 高速化ワークフロー
    // ==========================================
    'rust_workflow_category_compile_avoidance': {
      name: 'コンパイルしない',
      type: 'workflow_category',
      description: '差分だけ＆早い検査系に寄せることで、そもそもコンパイルを実行しない戦略。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_check': {
      name: 'cargo check',
      type: 'build_step',
      description: '型検査までで止めるので速い。普段使いに推奨。',
      command: 'cargo check --workspace --all-targets',
      dependencies: ['rust_workflow_category_compile_avoidance'],
      provides: ['fast_type_check'],
      status: 'recommended_practice',
    },
    'rust_workflow_lint_on_save': {
      name: 'clippy + rustfmt on save',
      type: 'dev_practice',
      description: 'IDEやpre-commitフックで保存時に実行。構文/スタイル段階で問題を検出し、ビルド前に修正を促す。',
      command: 'cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check',
      dependencies: ['rust_workflow_category_compile_avoidance'],
      provides: ['code_quality_gate'],
      status: 'recommended_practice',
    },
    'rust_workflow_warnings_as_errors': {
      name: 'Warnings as Errors',
      type: 'build_config',
      description: '警告をエラーとして扱うことで、潜在的な問題を早期に修正させる。',
      command: 'RUSTFLAGS="-D warnings" cargo check',
      dependencies: ['rust_workflow_category_compile_avoidance'],
      provides: ['strict_compilation'],
      status: 'recommended_practice',
    },
    'rust_workflow_trybuild': {
      name: 'trybuild for proc-macros',
      type: 'testing_framework',
      description: '"コンパイル失敗すべき"コードをテストとして固定化し、回帰を早期検出。proc-macro開発で特に有効。',
      dependencies: ['rust_workflow_category_compile_avoidance'],
      provides: ['compile_fail_testing'],
      status: 'specialized_tool',
    },

    'rust_workflow_category_build_slimming': {
      name: 'やらないことを減らす',
      type: 'workflow_category',
      description: '依存・ビルド単位を見直し、不要なコンパイル作業を削減する戦略。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_workspace_division': {
      name: 'Workspace Division',
      type: 'architecture_practice',
      description: '変更頻度の低いクレートを分割し、キャッシュの再利用率を向上させる。',
      dependencies: ['rust_workflow_category_build_slimming'],
      status: 'architectural_decision',
    },
    'rust_workflow_minimal_features': {
      name: 'Minimal Features',
      type: 'build_config',
      description: 'デフォルトfeatureを軽くし、必要な組み合わせだけをビルドする。',
      command: 'cargo build --no-default-features --features foo,bar',
      dependencies: ['rust_workflow_category_build_slimming'],
      status: 'recommended_practice',
    },
    'rust_workflow_build_rs_optimization': {
      name: 'build.rs Optimization',
      type: 'build_script_practice',
      description: '`rerun-if-changed`を厳密に記述し、不要な再ビルドを防ぐ。',
      dependencies: ['rust_workflow_category_build_slimming'],
      status: 'recommended_practice',
    },
    'rust_workflow_dependency_audit': {
      name: 'Dependency Audit',
      type: 'maintenance_task',
      description: '`cargo tree -d` や `cargo udeps` で重複・不要な依存を整理する。',
      command: 'cargo tree -d && cargo +nightly udeps',
      dependencies: ['rust_workflow_category_build_slimming'],
      status: 'periodic_task',
    },

    'rust_workflow_category_compilation_acceleration': {
      name: 'やったら速くする',
      type: 'workflow_category',
      description: 'コンパイル・リンク・キャッシュを最適化してビルドプロセス自体を高速化する戦略。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_incremental_dev': {
      name: 'Incremental Compilation (dev)',
      type: 'build_config',
      description: 'devプロファイルでインクリメンタルビルドを有効にし、codegen-unitsを増やして並列性を高める。',
      config: {
        'profile.dev': {
          incremental: true,
          'codegen-units': 16,
        },
      },
      dependencies: ['rust_workflow_category_compilation_acceleration'],
      status: 'recommended_practice',
    },
    'rust_workflow_sccache': {
      name: 'sccache',
      type: 'caching_tool',
      description: 'ローカル/リモートにコンパイル成果をキャッシュするラッパー。',
      command: 'export RUSTC_WRAPPER=$(which sccache) && cargo build',
      dependencies: ['rust_workflow_category_compilation_acceleration'],
      status: 'tooling_setup',
    },
    'rust_workflow_fast_linker': {
      name: 'Fast Linker',
      type: 'linking_tool',
      description: 'リンク時間を短縮するために高速なリンカ (lld, mold, zld) を使用する。',
      config: {
        '.cargo/config.toml': '[build]\nrustflags = ["-C", "link-arg=-fuse-ld=lld"]',
      },
      dependencies: ['rust_workflow_category_compilation_acceleration'],
      status: 'tooling_setup',
    },
    'rust_workflow_native_cpu': {
      name: 'Target CPU Optimization',
      type: 'build_config',
      description: 'ローカル開発時にCPUネイティブな命令セットを使いコンパイルを高速化する。',
      config: {
        '.cargo/config.toml': '[build]\nrustflags = ["-C", "target-cpu=native"]',
      },
      dependencies: ['rust_workflow_category_compilation_acceleration'],
      status: 'local_optimization',
    },
    'rust_workflow_shared_target': {
      name: 'Shared Target Directory',
      type: 'build_config',
      description: '複数プロジェクトでキャッシュを共有するために共通のtargetディレクトリを指定する。',
      config: {
        '.cargo/config.toml': '[build]\ntarget-dir = "/path/to/.cargo-target"',
      },
      dependencies: ['rust_workflow_category_compilation_acceleration'],
      status: 'environment_setup',
    },
    'rust_workflow_cargo_config_template': {
      name: 'Template .cargo/config.toml',
      type: 'configuration_snippet',
      description: '開発効率を向上させるための推奨.cargo/config.toml設定。',
      config_content: |||
        [build]
        target-dir = ".target"
        rustflags = [
          "-C", "target-cpu=native",
          "-C", "link-arg=-fuse-ld=lld"
        ]

        [target.x86_64-unknown-linux-gnu]
        linker = "clang"

        [profile.dev]
        incremental = true
        codegen-units = 16

        [profile.release]
        incremental = false
        lto = "thin"
        codegen-units = 8
      |||,
      dependencies: [
        'rust_workflow_incremental_dev',
        'rust_workflow_fast_linker',
        'rust_workflow_native_cpu',
        'rust_workflow_shared_target',
      ],
      status: 'template',
    },

    'rust_workflow_category_proactive_error_detection': {
      name: '事前にエラーを見つける',
      type: 'workflow_category',
      description: '自動化された仕組みで、手動テストの前にエラーを発見する運用。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_git_hooks': {
      name: 'Git Hooks / pre-commit',
      type: 'automation',
      description: 'コミット前に自動でcheck/clippy/fmtを実行し、品質を維持する。',
      dependencies: ['rust_workflow_category_proactive_error_detection'],
      status: 'recommended_practice',
    },
    'rust_workflow_doc_tests': {
      name: 'Doc Tests',
      type: 'testing_practice',
      description: 'ドキュメント内のサンプルコードをテストし、正確性を保証する。',
      command: 'cargo test --doc',
      dependencies: ['rust_workflow_category_proactive_error_detection'],
      status: 'recommended_practice',
    },
    'rust_workflow_cargo_hack': {
      name: 'cargo-hack for feature combinations',
      type: 'testing_tool',
      description: 'featureの組み合わせ爆発をテストし、互換性を検証する。',
      command: 'cargo hack check --each-feature --no-dev-deps',
      dependencies: ['rust_workflow_category_proactive_error_detection'],
      status: 'specialized_tool',
    },

    'rust_workflow_category_performance_measurement': {
      name: '可視化して計測する',
      type: 'workflow_category',
      description: 'ビルドのボトルネックを特定し、データに基づいて最適化を行う。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_build_profile': {
      name: 'Build Time Profiling',
      type: 'profiling_tool',
      description: '`cargo build -Z timings` を使って各コンパイルステップの時間を計測する。',
      command: 'cargo build -Z timings',
      dependencies: ['rust_workflow_category_performance_measurement'],
      status: 'nightly_feature',
    },
    'rust_workflow_bloat_analysis': {
      name: 'Bloat Analysis',
      type: 'analysis_tool',
      description: '`cargo-bloat`でバイナリサイズの内訳を調査し、肥大化の原因を特定する。',
      command: 'cargo bloat --release --crates',
      dependencies: ['rust_workflow_category_performance_measurement'],
      status: 'tooling_setup',
    },
    'rust_workflow_llvm_lines_analysis': {
      name: 'LLVM Lines Analysis',
      type: 'analysis_tool',
      description: '`cargo-llvm-lines`で関数の行数を分析し、モノモーフィズムによるコード膨張などを発見する。',
      command: 'cargo llvm-lines --bin your_binary',
      dependencies: ['rust_workflow_category_performance_measurement'],
      status: 'tooling_setup',
    },
    
    'rust_workflow_category_ci_best_practices': {
      name: 'CI/チームでのベストプラクティス',
      type: 'workflow_category',
      description: 'CI環境とチーム開発におけるビルドの安定性と速度を向上させるためのプラクティス。',
      dependencies: [],
      provides: ['workflow_grouping'],
    },
    'rust_workflow_ci_config': {
      name: 'CI Configuration',
      type: 'ci_cd_practice',
      description: 'CIではインクリメンタルビルドをOFFにし、sccacheやtargetディレクトリのキャッシュを有効にする。',
      dependencies: ['rust_workflow_category_ci_best_practices'],
      status: 'recommended_practice',
    },
    'rust_workflow_resolver_v2': {
      name: 'Resolver v2',
      type: 'build_config',
      description: '`Cargo.toml`で`resolver = "2"`を設定し、feature解決の再現性と安定性を向上させる。',
      dependencies: ['rust_workflow_category_ci_best_practices'],
      status: 'recommended_practice',
    },
    'rust_workflow_nextest': {
      name: 'cargo-nextest',
      type: 'testing_tool',
      description: 'テストを並列実行して高速化し、フィードバックを早める。',
      command: 'cargo nextest run',
      dependencies: ['rust_workflow_category_ci_best_practices'],
      status: 'tooling_setup',
    },
  },
  groups: {
    'rust_build_workflow': {
      name: 'Rust Build Optimization Workflow',
      description: 'Rustのコンパイル時間を短縮し、開発サイクルを高速化するためのワークフロー。',
      nodes: [
        'rust_workflow_category_compile_avoidance',
        'rust_workflow_check',
        'rust_workflow_lint_on_save',
        'rust_workflow_warnings_as_errors',
        'rust_workflow_trybuild',
        'rust_workflow_category_build_slimming',
        'rust_workflow_workspace_division',
        'rust_workflow_minimal_features',
        'rust_workflow_build_rs_optimization',
        'rust_workflow_dependency_audit',
        'rust_workflow_category_compilation_acceleration',
        'rust_workflow_incremental_dev',
        'rust_workflow_sccache',
        'rust_workflow_fast_linker',
        'rust_workflow_native_cpu',
        'rust_workflow_shared_target',
        'rust_workflow_cargo_config_template',
        'rust_workflow_category_proactive_error_detection',
        'rust_workflow_git_hooks',
        'rust_workflow_doc_tests',
        'rust_workflow_cargo_hack',
        'rust_workflow_category_performance_measurement',
        'rust_workflow_build_profile',
        'rust_workflow_bloat_analysis',
        'rust_workflow_llvm_lines_analysis',
        'rust_workflow_category_ci_best_practices',
        'rust_workflow_ci_config',
        'rust_workflow_resolver_v2',
        'rust_workflow_nextest',
      ],
    },
  },
}

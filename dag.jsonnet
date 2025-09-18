{
  // Kotoba GP2-based Graph Rewriting Language - Process Network DAG
  // Multi-crate architecture with 95% test coverage
  // Topological sort: Build order
  // Reverse topological sort: Problem resolution order

  // ==========================================
  // ノード定義 (Components/Processes)
  // ==========================================

  nodes: {
    // 基底層
    'types': {
      name: 'types',
      path: 'crates/kotoba-core/src/types.rs',
      type: 'foundation',
      description: '共通型定義 (Value, VertexId, EdgeId, GraphRef, etc.)',
      dependencies: [],
      provides: ['Value', 'VertexId', 'EdgeId', 'GraphRef', 'TxId', 'ContentHash'],
      status: 'published',
      published_version: '0.1.19',
      crate_name: 'kotoba-core',
      build_order: 1,
    },

    // IR層
    'ir_catalog': {
      name: 'ir_catalog',
      path: 'crates/kotoba-core/src/ir/catalog.rs',
      type: 'ir',
      description: 'スキーマ/索引/不変量定義',
      dependencies: ['types'],
      provides: ['Catalog', 'LabelDef', 'IndexDef', 'Invariant'],
      status: 'published',
      published_version: '0.1.19',
      crate_name: 'kotoba-core',
      build_order: 2,
    },

    'schema_validator': {
      name: 'schema_validator',
      path: 'crates/kotoba-schema/src/validator.rs',
      type: 'schema',
      description: 'Graph schema validation engine',
      dependencies: ['types', 'ir_catalog'],
      provides: ['SchemaValidator', 'ValidationResult'],
      status: 'published',
      published_version: '0.1.16',
      crate_name: 'kotoba-schema',
      build_order: 3,
    },

    'ir_rule': {
      name: 'ir_rule',
      path: 'crates/kotoba-core/src/ir/rule.rs',
      type: 'ir',
      description: 'DPO型付き属性グラフ書換えルール',
      dependencies: ['types'],
      provides: ['RuleIR', 'Match', 'Guard'],
      status: 'published',
      published_version: '0.1.19',
      crate_name: 'kotoba-core',
      build_order: 2,
    },

    'ir_query': {
      name: 'ir_query',
      path: 'crates/kotoba-core/src/ir/query.rs',
      type: 'ir',
      description: 'GQL論理プラン代数',
      dependencies: ['types'],
      provides: ['PlanIR', 'LogicalOp', 'Expr', 'Predicate'],
      status: 'published',
      published_version: '0.1.19',
      crate_name: 'kotoba-core',
      build_order: 2,
    },

    'ir_patch': {
      name: 'ir_patch',
      path: 'crates/kotoba-core/src/ir/patch.rs',
      type: 'ir',
      description: '差分表現 (addV/E, delV/E, setProp, relink)',
      dependencies: ['types'],
      provides: ['Patch', 'AddVertex', 'AddEdge', 'UpdateProp'],
      status: 'planned',
      build_order: 2,
    },

    'ir_strategy': {
      name: 'ir_strategy',
      path: 'crates/kotoba-core/src/ir/strategy.rs',
      type: 'ir',
      description: '戦略表現 (once|exhaust|while|seq|choice|priority)',
      dependencies: ['types', 'ir_patch'],
      provides: ['StrategyIR', 'StrategyOp', 'StrategyResult', 'Externs'],
      status: 'planned',
      build_order: 3,
    },

    // Workflow 層 (Itonami) - Phase 1 Complete
    'ir_workflow': {
      name: 'ir_workflow',
      path: 'crates/kotoba-workflow/src/ir.rs',
      type: 'workflow',
      description: 'TemporalベースワークフローIR (WorkflowIR, Activity, Saga)',
      dependencies: ['types', 'ir_strategy'],
      provides: ['WorkflowIR', 'ActivityIR', 'WorkflowExecution', 'SagaPattern'],
      status: 'planned',
      build_order: 4,
    },

    // グラフ層
    'graph_vertex': {
      name: 'graph_vertex',
      path: 'src/graph/vertex.rs',
      type: 'graph',
      description: '頂点関連構造体とビルダー',
      dependencies: ['types'],
      provides: ['VertexBuilder', 'VertexData'],
      status: 'planned',
      build_order: 2,
    },

    'graph_edge': {
      name: 'graph_edge',
      path: 'src/graph/edge.rs',
      type: 'graph',
      description: 'エッジ関連構造体とビルダー',
      dependencies: ['types'],
      provides: ['EdgeBuilder', 'EdgeData'],
      status: 'planned',
      build_order: 2,
    },

    'graph_core': {
      name: 'graph_core',
      path: 'src/graph/graph.rs',
      type: 'graph',
      description: '列指向グラフ表現とGraphRef',
      dependencies: ['types', 'graph_vertex', 'graph_edge'],
      provides: ['Graph', 'GraphRef'],
      status: 'planned',
      build_order: 3,
    },

    // ストレージ層
    'storage_mvcc': {
      name: 'storage_mvcc',
      path: 'crates/kotoba-storage/src/storage/mvcc.rs',
      type: 'storage',
      description: 'MVCCマネージャー',
      dependencies: ['types', 'graph_core'],
      provides: ['MVCCManager', 'Transaction', 'TxState'],
      status: 'planned',
      build_order: 4,
    },

    'storage_merkle': {
      name: 'storage_merkle',
      path: 'crates/kotoba-storage/src/storage/merkle.rs',
      type: 'storage',
      description: 'Merkle DAG永続化',
      dependencies: ['types', 'graph_core'],
      provides: ['MerkleDAG', 'MerkleNode', 'GraphVersion'],
      status: 'planned',
      build_order: 4,
    },

    'storage_lsm': {
      name: 'storage_lsm',
      path: 'crates/kotoba-storage/src/storage/lsm.rs',
      type: 'storage',
      description: 'RocksDB-based high-performance storage (95% test coverage)',
      dependencies: ['types'],
      provides: ['LSMTree', 'RocksDB'],
      status: 'published',
      published_version: '0.1.16',
      crate_name: 'kotoba-storage',
      build_order: 4,
    },

    'storage_object': {
      name: 'storage_object',
      path: 'crates/kotoba-storage/src/storage/object.rs',
      type: 'storage',
      description: 'Object storage backend (AWS S3, GCP Cloud Storage, Azure Blob Storage)',
      dependencies: ['types'],
      provides: ['ObjectStorageBackend', 'ObjectStorageProvider'],
      status: 'planned',
      build_order: 4,
    },

    'storage_main': {
      name: 'storage_main',
      path: 'crates/kotoba-storage/',
      type: 'storage_api',
      description: 'Main storage crate providing unified backend interface including KotobaDB',
      dependencies: ['types', 'errors', 'graph', 'cid', 'db'],
      provides: ['StorageBackend', 'StorageBackendFactory', 'KotobaDBBackend'],
      status: 'in_progress',
      build_order: 7,
    },

    // プランナー層
    'planner_logical': {
      name: 'planner_logical',
      path: 'src/planner/logical.rs',
      type: 'planner',
      description: '論理プランナー (GQL → 論理プラン)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core'],
      provides: ['LogicalPlanner', 'CostEstimator'],
      status: 'planned',
      build_order: 5,
    },

    'planner_physical': {
      name: 'planner_physical',
      path: 'src/planner/physical.rs',
      type: 'planner',
      description: '物理プランナー (論理プラン → 物理プラン)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core'],
      provides: ['PhysicalPlanner', 'PhysicalPlan', 'PhysicalOp'],
      status: 'planned',
      build_order: 5,
    },

    'planner_optimizer': {
      name: 'planner_optimizer',
      path: 'src/planner/optimizer.rs',
      type: 'planner',
      description: 'クエリ最適化器 (述語押下げ, 結合順序DP, インデックス選択)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core', 'planner_logical', 'planner_physical'],
      provides: ['QueryOptimizer', 'OptimizationRule'],
      status: 'planned',
      build_order: 6,
    },

    // 実行層
    'execution_parser': {
      name: 'execution_parser',
      path: 'src/execution/gql_parser.rs',
      type: 'execution',
      description: 'GQLパーサー',
      dependencies: ['types', 'ir_query'],
      provides: ['GqlParser'],
      status: 'planned',
      build_order: 5,
    },

    'execution_engine': {
      name: 'execution_engine',
      path: 'crates/kotoba-execution/src/execution/executor.rs',
      type: 'execution',
      description: 'クエリ実行器',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core', 'storage_mvcc', 'storage_merkle', 'planner_logical', 'planner_physical', 'planner_optimizer', 'execution_parser'],
      provides: ['QueryExecutor'],
      status: 'published',
      published_version: '0.1.16',
      crate_name: 'kotoba-execution',
      build_order: 7,
    },

    // Workflow 実行層 (Itonami) - Phase 2 Complete: MVCC + Event Sourcing
    'workflow_executor': {
      name: 'workflow_executor',
      path: 'crates/kotoba-workflow/src/executor.rs',
      type: 'workflow',
      description: 'Temporalベースワークフロー実行器 (MVCC + Event Sourcing)',
      dependencies: ['types', 'ir_workflow', 'graph_core', 'storage_mvcc', 'storage_merkle', 'execution_engine'],
      provides: ['WorkflowExecutor', 'ActivityExecutor', 'SagaExecutor', 'WorkflowStateManager', 'EventSourcingManager'],
      status: 'planned',
      build_order: 8,
    },

    'workflow_store': {
      name: 'workflow_store',
      path: 'crates/kotoba-workflow/src/store.rs',
      type: 'workflow',
      description: 'ワークフロー状態永続化 (MVCC + Event Sourcing + Snapshots)',
      dependencies: ['types', 'ir_workflow', 'storage_mvcc', 'storage_merkle'],
      provides: ['WorkflowStore', 'WorkflowStateManager', 'EventStore', 'SnapshotManager', 'EventSourcingManager'],
      status: 'planned',
      build_order: 7,
    },

    'workflow_designer': {
      name: 'workflow_designer',
      path: 'packages/kotoba-workflow-designer/src/index.tsx',
      type: 'ecosystem',
      description: 'Visual workflow designer UI with React/TypeScript',
      dependencies: ['types'],
      provides: ['WorkflowDesigner', 'ActivityPalette', 'PropertyPanel', 'WorkflowCanvas'],
      status: 'planned',
      build_order: 9,
    },

    'activity_libraries': {
      name: 'activity_libraries',
      path: 'crates/kotoba-workflow-activities/src/lib.rs',
      type: 'ecosystem',
      description: 'Pre-built activity libraries (HTTP, Database, Cloud, etc.)',
      dependencies: ['types', 'ir_workflow', 'workflow_executor'],
      provides: ['ActivityLibrary', 'HttpActivities', 'DatabaseActivities', 'CloudActivities'],
      status: 'planned',
      build_order: 10,
    },

    'kubernetes_operator': {
      name: 'kubernetes_operator',
      path: 'crates/kotoba-workflow-operator/src/lib.rs',
      type: 'ecosystem',
      description: 'Kubernetes operator for workflow management',
      dependencies: ['types', 'ir_workflow', 'workflow_executor', 'workflow_store'],
      provides: ['WorkflowOperator', 'WorkflowController', 'WorkflowReconciler'],
      status: 'planned',
      build_order: 11,
    },

    'cloud_integrations': {
      name: 'cloud_integrations',
      path: 'crates/kotoba-cloud-integrations/src/lib.rs',
      type: 'ecosystem',
      description: 'Cloud-native integrations (AWS, GCP, Azure)',
      dependencies: ['types'],
      provides: ['CloudIntegrationManager', 'AWSService', 'GCPService', 'AzureService'],
      status: 'planned',
      build_order: 12,
    },


    // 書換え層
    'rewrite_matcher': {
      name: 'rewrite_matcher',
      path: 'crates/kotoba-rewrite/src/rewrite/matcher.rs',
      type: 'rewrite',
      description: 'ルールマッチング (LHS + NACチェック)',
      dependencies: ['types', 'ir_rule', 'ir_catalog', 'graph_core'],
      provides: ['RuleMatcher'],
      status: 'planned',
      build_order: 5,
    },

    'rewrite_applier': {
      name: 'rewrite_applier',
      path: 'crates/kotoba-rewrite/src/rewrite/applier.rs',
      type: 'rewrite',
      description: 'ルール適用 (パッチ生成)',
      dependencies: ['types', 'ir_rule', 'ir_patch', 'graph_core'],
      provides: ['RuleApplier'],
      status: 'planned',
      build_order: 5,
    },

    'rewrite_engine': {
      name: 'rewrite_engine',
      path: 'crates/kotoba-rewrite/src/rewrite/engine.rs',
      type: 'rewrite',
      description: 'DPO書換えエンジン (マッチング + 適用 + 戦略実行)',
      dependencies: ['types', 'ir_rule', 'ir_strategy', 'graph_core', 'storage_mvcc', 'storage_merkle', 'rewrite_matcher', 'rewrite_applier'],
      provides: ['RewriteEngine', 'RewriteExterns'],
      status: 'planned',
      build_order: 6,
    },

    // セキュリティ層
    'security_jwt': {
      name: 'security_jwt',
      path: 'crates/kotoba-security/src/jwt.rs',
      type: 'security',
      description: 'JWTトークンの生成・検証機能',
      dependencies: ['types'],
      provides: ['JwtService', 'JwtClaims', 'TokenPair'],
      status: 'planned',
      build_order: 4,
    },

    'security_oauth2': {
      name: 'security_oauth2',
      path: 'crates/kotoba-security/src/oauth2.rs',
      type: 'security',
      description: 'OAuth2/OpenID Connect統合',
      dependencies: ['types', 'security_jwt'],
      provides: ['OAuth2Service', 'OAuth2Provider', 'OAuth2Config'],
      status: 'planned',
      build_order: 5,
    },

    'security_mfa': {
      name: 'security_mfa',
      path: 'crates/kotoba-security/src/mfa.rs',
      type: 'security',
      description: '多要素認証 (TOTP) 機能',
      dependencies: ['types'],
      provides: ['MfaService', 'MfaSecret', 'MfaCode'],
      status: 'planned',
      build_order: 4,
    },

    'security_password': {
      name: 'security_password',
      path: 'crates/kotoba-security/src/password.rs',
      type: 'security',
      description: 'パスワードハッシュ化・検証機能',
      dependencies: ['types'],
      provides: ['PasswordService', 'PasswordHash'],
      status: 'planned',
      build_order: 4,
    },

    'security_session': {
      name: 'security_session',
      path: 'crates/kotoba-security/src/session.rs',
      type: 'security',
      description: 'セッション管理機能',
      dependencies: ['types'],
      provides: ['SessionManager', 'SessionData'],
      status: 'planned',
      build_order: 4,
    },

    'security_core': {
      name: 'security_core',
      path: 'crates/kotoba-security/src/lib.rs',
      type: 'security',
      description: 'セキュリティ統合サービス',
      dependencies: ['types', 'security_jwt', 'security_oauth2', 'security_mfa', 'security_password', 'security_session', 'security_capabilities'],
      provides: ['SecurityService', 'SecurityError'],
      status: 'planned',
      build_order: 6,
    },

    'security_capabilities': {
      name: 'security_capabilities',
      path: 'crates/kotoba-security/src/capabilities.rs',
      type: 'security',
      description: 'Deno風capabilityベースセキュリティシステム',
      dependencies: ['types'],
      provides: ['Capability', 'CapabilitySet', 'CapabilityService', 'ResourceType', 'Action'],
      status: 'planned',
      build_order: 4,
    },

    // ==========================================
    // 分散実行・ネットワーク層
    // ==========================================

    'distributed_engine': {
      name: 'distributed_engine',
      path: 'crates/kotoba-distributed/src/lib.rs',
      type: 'distributed',
      description: '分散実行エンジン - CIDベースの分散グラフ処理',
      dependencies: ['types', 'graph_core', 'execution_engine', 'rewrite_engine', 'storage_mvcc', 'storage_merkle'],
      provides: ['DistributedEngine', 'CidCache', 'ClusterManager', 'DistributedTask', 'TaskResult'],
      status: 'planned',
      build_order: 8,
    },

    'network_protocol': {
      name: 'network_protocol',
      path: 'crates/kotoba-network/src/lib.rs',
      type: 'network',
      description: 'ネットワーク通信プロトコル - 分散実行のための通信層',
      dependencies: ['types', 'distributed_engine'],
      provides: ['NetworkMessage', 'NetworkManager', 'MessageHandler', 'TcpConnectionManager'],
      status: 'planned',
      build_order: 9,
    },

    'cid_system': {
      name: 'cid_system',
      path: 'crates/kotoba-cid/src/lib.rs',
      type: 'cid',
      description: 'CID (Content ID) システム - Merkle DAGにおけるコンテンツアドレッシング',
      dependencies: ['types'],
      provides: ['CidCalculator', 'CidManager', 'MerkleTreeBuilder', 'JsonCanonicalizer'],
      status: 'planned',
      build_order: 3,
    },

    'cli_interface': {
      name: 'cli_interface',
      path: 'crates/kotoba-cli/src/lib.rs',
      type: 'cli',
      description: 'CLI - Denoを参考にしたコマンドラインインターフェース',
      dependencies: ['types', 'distributed_engine', 'network_protocol', 'cid_system'],
      provides: ['Cli', 'Commands', 'ConfigManager', 'ProgressBar', 'LogFormatter'],
      status: 'planned',
      build_order: 10,
    },

    'kotoba_lsp': {
      name: 'kotoba_lsp',
      path: 'crates/kotoba-lsp/src/main.rs',
      type: 'lsp',
      description: 'Language Server Protocol implementation for Kotoba language',
      dependencies: ['kotobanet_core', 'jsonnet_core'],
      provides: ['lsp_server_binary'],
      status: 'in_progress',
      build_order: 10,
    },

    // ==========================================
    // Jsonnet 0.21.0 実装層 (Google Jsonnet完全対応)
    // ==========================================

    'jsonnet_error': {
      name: 'jsonnet_error',
      path: 'crates/kotoba-jsonnet/src/error.rs',
      type: 'jsonnet',
      description: 'Jsonnet評価エラー定義 (JsonnetError, Result)',
      dependencies: [],
      provides: ['JsonnetError', 'Result<T>'],
      status: 'planned',
      build_order: 1,
    },

    'jsonnet_value': {
      name: 'jsonnet_value',
      path: 'crates/kotoba-jsonnet/src/value.rs',
      type: 'jsonnet',
      description: 'Jsonnet値型定義 (JsonnetValue, JsonnetFunction)',
      dependencies: ['jsonnet_error'],
      provides: ['JsonnetValue', 'JsonnetFunction'],
      status: 'planned',
      build_order: 2,
    },

    'jsonnet_ast': {
      name: 'jsonnet_ast',
      path: 'crates/kotoba-jsonnet/src/ast.rs',
      type: 'jsonnet',
      description: 'Jsonnet抽象構文木定義 (Expr, ObjectField, BinaryOp, etc.)',
      dependencies: ['jsonnet_value'],
      provides: ['Expr', 'Stmt', 'Program', 'ObjectField', 'BinaryOp', 'UnaryOp'],
      status: 'planned',
      build_order: 3,
    },

    'jsonnet_lexer': {
      name: 'jsonnet_lexer',
      path: 'crates/kotoba-jsonnet/src/lexer.rs',
      type: 'jsonnet',
      description: 'Jsonnet字句解析器 (Lexer) - トークン化',
      dependencies: ['jsonnet_error'],
      provides: ['Lexer', 'Token', 'TokenWithPos', 'Position'],
      status: 'planned',
      build_order: 2,
    },

    'jsonnet_parser': {
      name: 'jsonnet_parser',
      path: 'crates/kotoba-jsonnet/src/parser.rs',
      type: 'jsonnet',
      description: 'Jsonnet構文解析器 (Parser) - AST構築',
      dependencies: ['jsonnet_ast', 'jsonnet_lexer'],
      provides: ['Parser', 'GqlToken'],
      status: 'planned',
      build_order: 4,
    },

    'jsonnet_evaluator': {
      name: 'jsonnet_evaluator',
      path: 'crates/kotoba-jsonnet/src/evaluator.rs',
      type: 'jsonnet',
      description: 'Jsonnet評価器 (Evaluator) - 式評価と実行',
      dependencies: ['jsonnet_ast', 'jsonnet_value'],
      provides: ['Evaluator'],
      status: 'planned',
      build_order: 5,
    },

    'jsonnet_stdlib': {
      name: 'jsonnet_stdlib',
      path: 'crates/kotoba-jsonnet/src/stdlib.rs',
      type: 'jsonnet',
      description: 'Jsonnet標準ライブラリ (80+関数) - std.*関数群',
      dependencies: ['jsonnet_value'],
      provides: ['StdLib', 'std_length', 'std_type', 'std_makeArray', 'std_filter', 'std_map', 'std_foldl', 'std_foldr', 'std_range', 'std_join', 'std_split', 'std_contains', 'std_startsWith', 'std_endsWith', 'std_substr', 'std_char', 'std_codepoint', 'std_toString', 'std_parseInt', 'std_parseJson', 'std_encodeUTF8', 'std_decodeUTF8', 'std_md5', 'std_base64', 'std_base64Decode', 'std_manifestJson', 'std_manifestJsonEx', 'std_manifestYaml', 'std_escapeStringJson', 'std_escapeStringYaml', 'std_escapeStringPython', 'std_escapeStringBash', 'std_escapeStringDollars', 'std_stringChars', 'std_stringBytes', 'std_format', 'std_isArray', 'std_isBoolean', 'std_isFunction', 'std_isNumber', 'std_isObject', 'std_isString', 'std_count', 'std_find', 'std_member', 'std_modulo', 'std_pow', 'std_exp', 'std_log', 'std_sqrt', 'std_sin', 'std_cos', 'std_tan', 'std_asin', 'std_acos', 'std_atan', 'std_floor', 'std_ceil', 'std_round', 'std_abs', 'std_max', 'std_min', 'std_clamp', 'std_assertEqual', 'std_sort', 'std_uniq', 'std_reverse', 'std_mergePatch', 'std_get', 'std_objectFields', 'std_objectFieldsAll', 'std_objectHas', 'std_objectHasAll', 'std_objectValues', 'std_objectValuesAll', 'std_prune', 'std_mapWithKey'],
      status: 'planned',
      build_order: 5,
    },

    'jsonnet_core': {
      name: 'jsonnet_core',
      path: 'crates/kotoba-jsonnet/src/lib.rs',
      type: 'jsonnet',
      description: 'JsonnetコアAPI - evaluate(), evaluate_to_json(), evaluate_to_yaml()',
      dependencies: ['jsonnet_evaluator', 'jsonnet_stdlib'],
      provides: ['evaluate', 'evaluate_with_filename', 'evaluate_to_json', 'evaluate_to_yaml', 'VERSION'],
      status: 'planned',
      build_order: 6,
    },

    // ==========================================
    // Kotoba Kotobanet 拡張層 (Kotoba特化拡張)
    // ==========================================

    'kotobanet_error': {
      name: 'kotobanet_error',
      path: 'crates/kotoba-kotobas/src/error.rs',
      type: 'kotobanet',
      description: 'Kotoba Kotobanet エラー定義',
      dependencies: [],
      provides: ['KotobaNetError', 'Result<T>'],
      status: 'planned',
      build_order: 7,
    },

    'kotobanet_http_parser': {
      name: 'kotobanet_http_parser',
      path: 'crates/kotoba-kotobas/src/http_parser.rs',
      type: 'kotobanet',
      description: 'HTTP Parser for .kotoba.json configuration files',
      dependencies: ['kotobanet_error', 'jsonnet_core'],
      provides: ['HttpParser', 'HttpRouteConfig', 'HttpConfig'],
      status: 'planned',
      build_order: 8,
    },

    'kotobanet_frontend': {
      name: 'kotobanet_frontend',
      path: 'crates/kotoba-kotobas/src/frontend.rs',
      type: 'kotobanet',
      description: 'Frontend Framework for React component definitions',
      dependencies: ['kotobanet_error', 'jsonnet_core'],
      provides: ['FrontendParser', 'ComponentDef', 'PageDef', 'ApiRouteDef', 'FrontendConfig'],
      status: 'planned',
      build_order: 8,
    },

    'kotobanet_deploy': {
      name: 'kotobanet_deploy',
      path: 'crates/kotoba-kotobas/src/deploy.rs',
      type: 'kotobanet',
      description: 'Deploy Configuration for deployment settings',
      dependencies: ['kotobanet_error', 'jsonnet_core'],
      provides: ['DeployParser', 'DeployConfig', 'ScalingConfig', 'RegionConfig'],
      status: 'planned',
      build_order: 8,
    },

    'kotobanet_config': {
      name: 'kotobanet_config',
      path: 'crates/kotoba-kotobas/src/config.rs',
      type: 'kotobanet',
      description: 'General configuration management',
      dependencies: ['kotobanet_error', 'jsonnet_core'],
      provides: ['ConfigParser', 'AppConfig', 'DatabaseConfig', 'CacheConfig'],
      status: 'planned',
      build_order: 8,
    },

    'kotobanet_core': {
      name: 'kotobanet_core',
      path: 'crates/kotoba-kotobas/src/lib.rs',
      type: 'kotobanet',
      description: 'Kotoba Kotobanet コアAPI - evaluate_kotoba(), HTTP/Frontend/Deploy/Config パーサー統合',
      dependencies: ['kotobanet_error', 'kotobanet_http_parser', 'kotobanet_frontend', 'kotobanet_deploy', 'kotobanet_config', 'jsonnet_core'],
      provides: ['evaluate_kotoba', 'evaluate_kotoba_to_json', 'evaluate_kotoba_to_yaml', 'HttpParser', 'FrontendParser', 'DeployParser', 'ConfigParser'],
      status: 'planned',
      build_order: 9,
    },

    // HTTPサーバー層
    'http_ir': {
      name: 'http_ir',
      path: 'src/http/ir.rs',
      type: 'http',
      description: 'HTTPサーバー用IR定義 (Route, Middleware, Request, Response)',
      dependencies: ['types', 'ir_catalog', 'security_core'],
      provides: ['HttpRoute', 'HttpMiddleware', 'HttpRequest', 'HttpResponse', 'HttpConfig'],
      status: 'planned',
      build_order: 7,
    },

    'http_parser': {
      name: 'http_parser',
      path: 'src/http/parser.rs',
      type: 'http',
      description: '.kotoba.json/.kotobaファイル（Jsonnet形式）のパーサー',
      dependencies: ['types', 'http_ir'],
      provides: ['HttpConfigParser', 'KotobaParser'],
      status: 'pending',
      build_order: 5,
    },

    'http_handlers': {
      name: 'http_handlers',
      path: 'src/http/handlers.rs',
      type: 'http',
      description: 'HTTPハンドラーとミドルウェア処理',
      dependencies: ['types', 'http_ir', 'graph_core', 'rewrite_engine', 'storage_mvcc', 'storage_merkle', 'security_core'],
      provides: ['HttpHandler', 'MiddlewareProcessor', 'RequestProcessor'],
      status: 'pending',
      build_order: 8,
    },

    'http_engine': {
      name: 'http_engine',
      path: 'src/http/engine.rs',
      type: 'http',
      description: 'HTTPサーバーエンジン',
      dependencies: ['types', 'http_ir', 'http_handlers', 'graph_core', 'storage_mvcc', 'storage_merkle', 'rewrite_engine', 'security_core'],
      provides: ['HttpEngine', 'ServerState'],
      status: 'pending',
      build_order: 9,
    },

    'http_server': {
      name: 'http_server',
      path: 'crates/kotoba-server/src/http/server.rs',
      type: 'http',
      description: 'メインHTTPサーバー',
      dependencies: ['types', 'http_ir', 'http_parser', 'http_engine', 'http_handlers', 'graphql_schema', 'graphql_handler'],
      provides: ['HttpServer', 'ServerBuilder'],
      status: 'planned',
      build_order: 10,
    },

    // ==========================================
    // GraphQL 層
    // ==========================================

    'graphql_schema': {
      name: 'graphql_schema',
      path: 'crates/kotoba-server/src/http/graphql.rs',
      type: 'graphql',
      description: 'GraphQLスキーマ定義とスキーマ管理操作',
      dependencies: ['types', 'schema_validator'],
      provides: ['GraphQLSchema', 'SchemaMutations', 'SchemaQueries'],
      status: 'planned',
      build_order: 9,
    },

    'graphql_handler': {
      name: 'graphql_handler',
      path: 'crates/kotoba-server/src/http/graphql.rs',
      type: 'graphql',
      description: 'GraphQLリクエスト処理と実行エンジン',
      dependencies: ['types', 'graphql_schema'],
      provides: ['GraphQLHandler', 'RequestExecutor'],
      status: 'planned',
      build_order: 9,
    },

    // フロントエンドフレームワーク層
    'frontend_component_ir': {
      name: 'frontend_component_ir',
      path: 'src/frontend/component_ir.rs',
      type: 'frontend',
      description: 'ReactコンポーネントIR定義 (Server/Client Components, Props, State)',
      dependencies: ['types'],
      provides: ['ComponentIR', 'ElementIR', 'JSXIR', 'HookIR'],
      status: 'planned',
      build_order: 3,
    },

    'frontend_route_ir': {
      name: 'frontend_route_ir',
      path: 'src/frontend/route_ir.rs',
      type: 'frontend',
      description: 'App RouterシステムIR定義 (ファイルベースルーティング, Layout, Loading, Error境界)',
      dependencies: ['types', 'frontend_component_ir'],
      provides: ['RouteIR', 'RouteTableIR', 'NavigationIR'],
      status: 'planned',
      build_order: 4,
    },

    'frontend_render_ir': {
      name: 'frontend_render_ir',
      path: 'src/frontend/render_ir.rs',
      type: 'frontend',
      description: 'コンポーネントツリーとレンダリングエンジンのIR定義',
      dependencies: ['types', 'frontend_component_ir'],
      provides: ['VirtualNodeIR', 'RenderContext', 'RenderResultIR', 'DiffIR'],
      status: 'planned',
      build_order: 4,
    },

    'frontend_build_ir': {
      name: 'frontend_build_ir',
      path: 'src/frontend/build_ir.rs',
      type: 'frontend',
      description: 'ブイルド/バンドルシステムのIR定義',
      dependencies: ['types', 'frontend_component_ir'],
      provides: ['BuildConfigIR', 'BundleResultIR', 'CodeSplittingIR'],
      status: 'planned',
      build_order: 4,
    },

    'frontend_api_ir': {
      name: 'frontend_api_ir',
      path: 'src/frontend/api_ir.rs',
      type: 'frontend',
      description: 'APIルートIR定義 (REST/GraphQL/WebSocket)',
      dependencies: ['types'],
      provides: ['ApiRouteIR', 'DatabaseIR', 'MiddlewareIR', 'WebSocketIR'],
      status: 'planned',
      build_order: 4,
    },

    'frontend_framework': {
      name: 'frontend_framework',
      path: 'src/frontend/framework.rs',
      type: 'frontend',
      description: 'Web Frameworkのコア実装',
      dependencies: ['types', 'frontend_component_ir', 'frontend_route_ir', 'frontend_render_ir', 'frontend_build_ir', 'frontend_api_ir', 'http_ir'],
      provides: ['WebFramework', 'ComponentRenderer', 'BuildEngine'],
      status: 'in_progress',
      build_order: 5,
    },

    // メインライブラリ
    'lib': {
      name: 'lib',
      path: 'src/lib.rs',
      type: 'library',
      description: 'メインライブラリインターフェース',
      dependencies: ['types', 'ir_catalog', 'ir_rule', 'ir_query', 'ir_patch', 'ir_strategy', 'graph_core', 'storage_mvcc', 'storage_merkle', 'storage_lsm', 'storage_object', 'security_core', 'planner_logical', 'planner_physical', 'planner_optimizer', 'execution_parser', 'execution_engine', 'rewrite_matcher', 'rewrite_applier', 'rewrite_engine', 'http_ir', 'http_parser', 'http_handlers', 'http_engine', 'http_server'],
      provides: ['kotoba'],
      status: 'planned',
      build_order: 11,
    },

    // Examples層
    'example_frontend_app': {
      name: 'example_frontend_app',
      path: 'examples/frontend_app/main.rs',
      type: 'example',
      description: 'JsonnetベースのフルスタックWebフレームワークの使用例',
      dependencies: ['lib', 'frontend_framework', 'http_server'],
      provides: ['frontend_app_example'],
      status: 'planned',
      build_order: 12,
    },

    'example_http_server': {
      name: 'example_http_server',
      path: 'examples/http_server/main.rs',
      type: 'example',
      description: 'HTTPサーバーの使用例',
      dependencies: ['lib', 'http_server'],
      provides: ['http_server_example'],
      status: 'planned',
      build_order: 12,
    },

    'example_social_network': {
      name: 'example_social_network',
      path: 'examples/social_network/main.rs',
      type: 'example',
      description: 'ソーシャルネットワークグラフ処理の使用例',
      dependencies: ['lib', 'graph_core', 'execution_engine', 'rewrite_engine'],
      provides: ['social_network_example'],
      status: 'planned',
      build_order: 12,
    },

    'example_tauri_react_app': {
      name: 'example_tauri_react_app',
      path: 'examples/tauri_react_app/main.rs',
      type: 'example',
      description: 'Tauri + React + Kotoba Frontend Frameworkのデスクトップアプリケーション例',
      dependencies: ['lib', 'frontend_framework', 'graph_core', 'storage_mvcc', 'storage_merkle'],
      provides: ['tauri_react_app_example'],
      status: 'in_progress',
      build_order: 13,
    },

    // ==========================================
    // Deploy層 (Deno Deploy相当)
    // ==========================================

    'deploy_config': {
      name: 'deploy_config',
      path: 'crates/kotoba-deploy/src/config.rs',
      type: 'deploy',
      description: 'デプロイ設定のIR定義 (Jsonnetベースの.kotoba-deployファイル)',
      dependencies: ['types'],
      provides: ['DeployConfig', 'ScalingConfig', 'RegionConfig'],
      status: 'planned',
      build_order: 7,
    },

    'deploy_parser': {
      name: 'deploy_parser',
      path: 'crates/kotoba-deploy/src/parser.rs',
      type: 'deploy',
      description: '.kotoba-deployファイルのパーサー',
      dependencies: ['types', 'deploy_config'],
      provides: ['DeployConfigParser'],
      status: 'planned',
      build_order: 8,
    },

    'deploy_scaling': {
      name: 'deploy_scaling',
      path: 'crates/kotoba-deploy/src/scaling.rs',
      type: 'deploy',
      description: '自動スケーリングエンジン',
      dependencies: ['types', 'deploy_config', 'graph_core'],
      provides: ['ScalingEngine', 'LoadBalancer', 'AutoScaler'],
      status: 'planned',
      build_order: 9,
    },

    'deploy_network': {
      name: 'deploy_network',
      path: 'crates/kotoba-deploy/src/network.rs',
      type: 'deploy',
      description: 'グローバル分散ネットワーク管理',
      dependencies: ['types', 'deploy_config', 'deploy_scaling'],
      provides: ['NetworkManager', 'RegionManager', 'EdgeRouter'],
      status: 'planned',
      build_order: 10,
    },

    'deploy_git_integration': {
      name: 'deploy_git_integration',
      path: 'crates/kotoba-deploy/src/git_integration.rs',
      type: 'deploy',
      description: 'GitHub連携と自動デプロイ',
      dependencies: ['types', 'deploy_config', 'deploy_network'],
      provides: ['GitIntegration', 'AutoDeploy', 'WebhookHandler'],
      status: 'planned',
      build_order: 11,
    },

    'deploy_controller': {
      name: 'deploy_controller',
      path: 'crates/kotoba-deploy/src/controller.rs',
      type: 'deploy',
      description: 'ISO GQLを使用したデプロイコントロール',
      dependencies: ['types', 'deploy_config', 'deploy_scaling', 'deploy_network', 'deploy_git_integration', 'graph_core', 'rewrite_engine'],
      provides: ['DeployController', 'DeploymentManager'],
      status: 'planned',
      build_order: 12,
    },

    'deploy_cli': {
      name: 'deploy_cli',
      path: 'crates/kotoba-deploy-cli/src/lib.rs',
      type: 'deploy',
      description: 'kotoba deploy CLIコマンド',
      dependencies: ['types', 'deploy_controller', 'http_server'],
      provides: ['DeployCLI'],
      status: 'planned',
      build_order: 13,
    },

    'deploy_runtime': {
      name: 'deploy_runtime',
      path: 'crates/kotoba-deploy/src/runtime.rs',
      type: 'deploy',
      description: 'デプロイ実行ランタイム (WebAssembly + WASM Edge対応)',
      dependencies: ['types', 'deploy_controller', 'wasm'],
      provides: ['DeployRuntime', 'WasmRuntime'],
      status: 'planned',
      build_order: 14,
    },

    'deploy_example_simple': {
      name: 'deploy_example_simple',
      path: 'examples/deploy/simple.kotoba-deploy',
      type: 'deploy_example',
      description: 'シンプルなデプロイメント設定例',
      dependencies: ['deploy_config'],
      provides: ['simple_deploy_example'],
      status: 'pending',
      build_order: 15,
    },

    'deploy_example_microservices': {
      name: 'deploy_example_microservices',
      path: 'examples/deploy/microservices.kotoba-deploy',
      type: 'deploy_example',
      description: 'マイクロサービスデプロイメント設定例',
      dependencies: ['deploy_config', 'deploy_example_simple'],
      provides: ['microservices_deploy_example'],
      status: 'pending',
      build_order: 16,
    },

    // ==========================================
    // Deploy拡張層 (新しく実装された拡張機能)
    // ==========================================

    // CLI拡張
    'deploy_cli_core': {
      name: 'deploy_cli_core',
      path: 'crates/kotoba-deploy-cli/src/lib.rs',
      type: 'deploy_cli',
      description: '拡張CLIマネージャー - デプロイメント管理、設定管理、進捗表示',
      dependencies: ['types', 'deploy_controller', 'http_server'],
      provides: ['CliManager', 'DeploymentInfo', 'OutputFormat', 'FormatOutput'],
      status: 'planned',
      build_order: 15,
    },

    'deploy_cli_binary': {
      name: 'deploy_cli_binary',
      path: 'crates/kotoba-deploy-cli/src/main.rs',
      type: 'deploy_cli',
      description: 'CLIバイナリ - 完全なデプロイメント処理、設定ファイル管理、進捗バー表示',
      dependencies: ['deploy_cli_core', 'deploy_controller', 'deploy_scaling', 'deploy_network', 'deploy_runtime'],
      provides: ['kotoba-deploy-cli'],
      status: 'planned',
      build_order: 16,
    },

    // コントローラー拡張
    'deploy_controller_core': {
      name: 'deploy_controller_core',
      path: 'crates/kotoba-deploy-controller/src/lib.rs',
      type: 'deploy_controller',
      description: '高度なデプロイコントローラー - ロールバック、ブルーグリーン、カナリアデプロイ',
      dependencies: ['types', 'deploy_config', 'deploy_scaling', 'deploy_network', 'deploy_git_integration', 'graph_core', 'rewrite_engine'],
      provides: ['DeployController', 'DeploymentHistoryManager', 'RollbackManager', 'BlueGreenDeploymentManager', 'CanaryDeploymentManager', 'HealthCheckManager'],
      status: 'planned',
      build_order: 17,
    },

    // ネットワーク拡張
    'deploy_network_core': {
      name: 'deploy_network_core',
      path: 'crates/kotoba-deploy-network/src/lib.rs',
      type: 'deploy_network',
      description: '高度なネットワークマネージャー - CDN統合、セキュリティ、エッジ最適化',
      dependencies: ['types', 'deploy_config', 'deploy_scaling'],
      provides: ['NetworkManager', 'CdnManager', 'SecurityManager', 'GeoManager', 'EdgeOptimizationManager'],
      status: 'planned',
      build_order: 18,
    },

    // スケーリング拡張（完了）
    'deploy_scaling_core': {
      name: 'deploy_scaling_core',
      path: 'crates/kotoba-deploy-scaling/src/lib.rs',
      type: 'deploy_scaling',
      description: 'AI予測スケーリングエンジン - トラフィック予測、コスト最適化、異常検知',
      dependencies: ['types', 'deploy_config', 'graph_core'],
      provides: ['PredictiveScaler', 'CostOptimizer', 'AdvancedMetricsAnalyzer', 'IntegratedScalingManager'],
      status: 'planned',
      build_order: 19,
    },

    // Hosting Server 層
    'deploy_hosting_server': {
      name: 'deploy_hosting_server',
      path: 'src/deploy/hosting_server.rs',
      type: 'deploy',
      description: 'ホスティングサーバーの実装 - デプロイされたアプリをホスト',
      dependencies: ['deploy_controller_core', 'http_server', 'frontend_framework', 'graph_core', 'execution_engine', 'storage_mvcc', 'storage_merkle'],
      provides: ['HostingServer', 'AppHost', 'RuntimeManager'],
      status: 'planned',
      build_order: 20,
    },

    'deploy_hosting_manager': {
      name: 'deploy_hosting_manager',
      path: 'src/deploy/hosting_manager.rs',
      type: 'deploy',
      description: 'ホスティングマネージャー - アプリのライフサイクル管理',
      dependencies: ['deploy_hosting_server', 'deploy_scaling', 'deploy_network'],
      provides: ['HostingManager', 'DeploymentLifecycle'],
      status: 'planned',
      build_order: 18,
    },

    'deploy_hosting_example': {
      name: 'deploy_hosting_example',
      path: 'examples/deploy/hosting_example.rs',
      type: 'deploy_example',
      description: 'ホスティングサーバーの使用例',
      dependencies: ['deploy_hosting_manager', 'deploy_cli'],
      provides: ['hosting_server_example'],
      status: 'pending',
      build_order: 19,
    },

    // ==========================================
    // Static Site Generator 層 (Kotoba SSG) - Kotoba言語で実装されたGitHub Pages
    // ==========================================

    'markdown_parser': {
      name: 'markdown_parser',
      path: 'crates/kotoba-ssg/src/markdown/parser.rs',
      type: 'ssg',
      description: 'Markdownパーサー - MarkdownファイルをHTMLに変換',
      dependencies: ['types', 'jsonnet_core'],
      provides: ['MarkdownParser', 'HtmlRenderer', 'CodeHighlighter', 'TableRenderer'],
      status: 'planned',
      build_order: 15,
    },

    'html_template_engine': {
      name: 'html_template_engine',
      path: 'crates/kotoba-ssg/src/template/engine.rs',
      type: 'ssg',
      description: 'HTMLテンプレートエンジン - Jsonnetベースのテンプレート処理',
      dependencies: ['types', 'jsonnet_core', 'markdown_parser'],
      provides: ['TemplateEngine', 'LayoutRenderer', 'PartialRenderer', 'AssetManager'],
      status: 'planned',
      build_order: 16,
    },

    'static_site_generator': {
      name: 'static_site_generator',
      path: 'crates/kotoba-ssg/src/generator.rs',
      type: 'ssg',
      description: '静的サイトジェネレーター - 完全Kotoba言語実装のSSG',
      dependencies: ['types', 'jsonnet_core', 'markdown_parser', 'html_template_engine', 'http_ir'],
      provides: ['SiteGenerator', 'PageBuilder', 'SitemapGenerator', 'FeedGenerator'],
      status: 'planned',
      build_order: 17,
    },

    'github_pages_deployer': {
      name: 'github_pages_deployer',
      path: 'crates/kotoba-ssg/src/deploy/github_pages.rs',
      type: 'ssg',
      description: 'GitHub Pagesデプロイヤー - GitHub Pagesへの自動デプロイメント',
      dependencies: ['types', 'static_site_generator', 'http_ir'],
      provides: ['GitHubPagesDeployer', 'GitIntegration', 'CNAMEHandler', 'RedirectManager'],
      status: 'planned',
      build_order: 18,
    },

    'documentation_builder': {
      name: 'documentation_builder',
      path: 'crates/kotoba-ssg/src/builder/documentation.rs',
      type: 'ssg',
      description: 'ドキュメントビルダー - 技術ドキュメント特化のビルダー',
      dependencies: ['types', 'static_site_generator', 'docs_core'],
      provides: ['DocumentationBuilder', 'ApiDocGenerator', 'CodeExampleRenderer', 'SearchIndexBuilder'],
      status: 'planned',
      build_order: 19,
    },

    // ==========================================
    // AI Agent 層 (Manimani) - Jsonnet-only AI Agent Framework
    // ==========================================

    'ai_agent_parser': {
      name: 'ai_agent_parser',
      path: 'crates/kotoba-kotobas/src/ai_parser.rs',
      type: 'ai_agent',
      description: 'Jsonnet-based AI agent定義パーサー - .manimaniファイルの解析',
      dependencies: ['kotobanet_core', 'jsonnet_core'],
      provides: ['AiAgentParser', 'AgentConfig', 'ToolConfig', 'ChainConfig'],
      status: 'pending',
      build_order: 20,
    },

    'db_handler': {
      name: 'db_handler',
      path: 'crates/kotoba-jsonnet/src/runtime/db.rs',
      type: 'runtime_extension',
      description: 'Jsonnet evaluator handler for database operations (GQL Query, Rewrite Rules)',
      dependencies: ['jsonnet_core', 'execution_engine', 'rewrite_engine'],
      provides: ['DbHandler', 'std.ext.db.query', 'std.ext.db.rewrite', 'std.ext.db.patch'],
      status: 'in_progress',
      build_order: 21,
    },

    'ai_runtime': {
      name: 'ai_runtime',
      path: 'crates/kotoba-kotobas/src/ai_runtime.rs',
      type: 'ai_agent',
      description: 'AI Agent実行ランタイム - Jsonnet evaluator拡張によるAI処理',
      dependencies: ['ai_agent_parser', 'jsonnet_core', 'http_ir', 'db_handler'],
      provides: ['AiRuntime', 'AgentExecutor', 'AsyncEvaluator', 'StreamingProcessor'],
      status: 'pending',
      build_order: 22,
    },

    'ai_models': {
      name: 'ai_models',
      path: 'crates/kotoba-kotobas/src/ai_models.rs',
      type: 'ai_agent',
      description: 'AIモデル統合 - OpenAI, Anthropic, Google AIなどのAPI統合',
      dependencies: ['ai_runtime', 'jsonnet_core'],
      provides: ['OpenAiModel', 'AnthropicModel', 'GoogleAiModel', 'ModelManager', 'ApiClient'],
      status: 'pending',
      build_order: 23,
    },

    'ai_tools': {
      name: 'ai_tools',
      path: 'crates/kotoba-kotobas/src/ai_tools.rs',
      type: 'ai_agent',
      description: 'AIツールシステム - 外部コマンド実行、関数呼び出し、データ処理',
      dependencies: ['ai_runtime', 'jsonnet_core'],
      provides: ['ToolExecutor', 'CommandTool', 'FunctionTool', 'DataTool', 'ToolRegistry'],
      status: 'pending',
      build_order: 24,
    },

    'ai_memory': {
      name: 'ai_memory',
      path: 'crates/kotoba-kotobas/src/ai_memory.rs',
      type: 'ai_agent',
      description: 'AIメモリ管理 - 会話履歴、コンテキスト、状態管理',
      dependencies: ['ai_runtime', 'storage_mvcc', 'storage_merkle', 'db_handler'],
      provides: ['MemoryManager', 'ConversationMemory', 'VectorMemory', 'StateManager'],
      status: 'pending',
      build_order: 25,
    },

    'ai_chains': {
      name: 'ai_chains',
      path: 'crates/kotoba-kotobas/src/ai_chains.rs',
      type: 'ai_agent',
      description: 'AIチェーンシステム - 複数ステップのワークフロー実行',
      dependencies: ['ai_agent_parser', 'ai_runtime', 'ai_models', 'ai_tools', 'ai_memory'],
      provides: ['ChainExecutor', 'SequentialChain', 'ParallelChain', 'ConditionalChain', 'LoopChain'],
      status: 'pending',
      build_order: 26,
    },

    'ai_examples': {
      name: 'ai_examples',
      path: 'examples/ai_agents/',
      type: 'ai_example',
      description: 'AI Agentの使用例 - Jsonnet-only AI agentアプリケーション',
      dependencies: ['ai_chains', 'ai_models', 'ai_tools', 'ai_memory'],
      provides: ['ai_agent_examples', 'chatbot_example', 'code_assistant_example', 'data_analyzer_example'],
      status: 'pending',
      build_order: 27,
    },

    'package_manager': {
      name: 'package_manager',
      path: 'crates/kotoba-package-manager/src/lib.rs',
      type: 'package_manager',
      description: 'npm/deno like package manager with merkledag + cid',
      dependencies: ['types', 'cid_system'],
      provides: ['PackageManager', 'PackageResolver', 'PackageInstaller'],
      status: 'in_progress',
      build_order: 4,
    },

    'state_graph_lib': {
      name: 'state_graph_lib',
      path: 'crates/kotoba-state-graph/src/lib.rs',
      type: 'ui_library',
      description: 'UI state management library providing schema, rules, and a .kotobas accessor library.',
      dependencies: ['types', 'rewrite_engine', 'execution_engine'],
      provides: ['UiVertexType', 'UiEdgeLabel', 'UiPropKey', 'get_standard_ui_rules', 'state.kotoba'],
      status: 'planned',
      build_order: 7,
    },

    // ==========================================
    // Kotoba Documentation Generator (kdoc)
    // ==========================================

    'docs_parser': {
      name: 'docs_parser',
      path: 'crates/kotoba-docs/src/parser.rs',
      type: 'documentation',
      description: 'Multi-language source code parser for documentation generation (Rust, JS, TS, Python, Go)',
      dependencies: ['types'],
      provides: ['DocParser', 'LanguageParser', 'RustParser', 'JavaScriptParser', 'TypeScriptParser', 'PythonParser', 'GoParser'],
      status: 'planned',
      build_order: 3,
    },

    'docs_config': {
      name: 'docs_config',
      path: 'crates/kotoba-docs/src/config.rs',
      type: 'documentation',
      description: 'Documentation configuration management and TOML/JSON/YAML parsing',
      dependencies: ['types'],
      provides: ['DocsConfig', 'ConfigManager', 'auto_detect_config', 'create_default_config_file'],
      status: 'planned',
      build_order: 3,
    },

    'docs_generator': {
      name: 'docs_generator',
      path: 'crates/kotoba-docs/src/generator.rs',
      type: 'documentation',
      description: 'Documentation generation engine with HTML/Markdown/JSON output support',
      dependencies: ['types', 'docs_parser', 'docs_config'],
      provides: ['DocGenerator', 'OutputFormat', 'GenerateResult', 'DocItem'],
      status: 'planned',
      build_order: 4,
    },

    'docs_template': {
      name: 'docs_template',
      path: 'crates/kotoba-docs/src/template.rs',
      type: 'documentation',
      description: 'Template engine for documentation with Tera integration and custom filters',
      dependencies: ['types', 'docs_generator'],
      provides: ['TemplateEngine', 'TemplateContext', 'TemplateFilter', 'DocTemplate'],
      status: 'planned',
      build_order: 5,
    },

    'docs_search': {
      name: 'docs_search',
      path: 'crates/kotoba-docs/src/search.rs',
      type: 'documentation',
      description: 'Full-text search engine with fuzzy matching and indexing',
      dependencies: ['types', 'docs_parser'],
      provides: ['SearchEngine', 'SearchResult', 'SearchOptions', 'SearchEntry'],
      status: 'planned',
      build_order: 4,
    },

    'docs_server': {
      name: 'docs_server',
      path: 'crates/kotoba-docs/src/server.rs',
      type: 'documentation',
      description: 'Web server for documentation with REST API and static file serving',
      dependencies: ['types', 'docs_generator', 'docs_search', 'http_ir'],
      provides: ['DocServer', 'ServerState', 'SearchParams', 'SearchResponse'],
      status: 'planned',
      build_order: 8,
    },

    'docs_core': {
      name: 'docs_core',
      path: 'crates/kotoba-docs/src/lib.rs',
      type: 'documentation',
      description: 'Kotoba Documentation Generator core library - main API and error handling',
      dependencies: ['types', 'docs_parser', 'docs_config', 'docs_generator', 'docs_template', 'docs_search', 'docs_server'],
      provides: ['DocsError', 'Result<T>', 'DocType', 'DocsConfig', 'DocItem'],
      status: 'planned',
      build_order: 6,
    },

    'docs_cli': {
      name: 'docs_cli',
      path: 'crates/kotoba-cli/src/main.rs',
      type: 'documentation_cli',
      description: 'CLI commands for documentation generation (generate, serve, search, init)',
      dependencies: ['types', 'docs_core', 'cli_interface'],
      provides: ['docs generate', 'docs serve', 'docs search', 'docs init'],
      status: 'planned',
      build_order: 11,
    },

    // ==========================================
    // KotobaDB 層
    // ==========================================
    'db_core': {
      name: 'db_core',
      path: 'crates/kotoba-db-core/',
      type: 'db',
      description: 'Core traits, data structures, and transaction logic for KotobaDB.',
      dependencies: ['types', 'cid_system'],
      provides: ['StorageEngine', 'Transaction', 'ACID'],
      status: 'in_progress',
      build_order: 4,
    },

    'db_engine_memory': {
      name: 'db_engine_memory',
      path: 'crates/kotoba-db-engine-memory/',
      type: 'db_engine',
      description: 'Default in-memory storage engine for KotobaDB.',
      dependencies: ['db_core'],
      provides: ['MemoryStorageEngine'],
      status: 'planned',
      build_order: 5,
    },

    'db_engine_lsm': {
      name: 'db_engine_lsm',
      path: 'crates/kotoba-db-engine-lsm/',
      type: 'db_engine',
      description: 'LSM-Tree based high-performance storage engine for KotobaDB.',
      dependencies: ['db_core'],
      provides: ['LSMStorageEngine', 'WALManager', 'SSTableManager'],
      status: 'in_progress',
      build_order: 5,
    },

    'db': {
      name: 'db',
      path: 'crates/kotoba-db/',
      type: 'db_api',
      description: 'User-facing API for KotobaDB, a graph-native, version-controlled database.',
      dependencies: ['db_core', 'db_engine_memory', 'db_engine_lsm'],
      provides: ['KotobaDB', 'DBSnapshot'],
      status: 'planned',
      build_order: 6,
    },

    // 分散システム層
    'db_cluster': {
      name: 'db_cluster',
      path: 'crates/kotoba-db-cluster/',
      type: 'db_cluster',
      description: 'Distributed clustering and consensus for KotobaDB with Raft algorithm.',
      dependencies: ['db_core', 'db'],
      provides: ['KotobaCluster', 'RaftConsensus', 'PartitionManager', 'ReplicationManager'],
      status: 'planned',
      build_order: 8,
    },

    // ==========================================
    // Future Features (実装予定)
    // ==========================================

    // 運用機能強化 (Operational Features)
    'backup_restore': {
      name: 'backup_restore',
      path: 'crates/kotoba-backup/',
      type: 'operational',
      description: 'Automated backup and restore system for KotobaDB.',
      dependencies: ['db', 'storage_main'],
      provides: ['BackupManager', 'RestoreManager', 'PointInTimeRecovery'],
      status: 'completed',
      build_order: 9,
      priority: 'high',
      estimated_effort: '2-3 weeks',
    },

    'monitoring_metrics': {
      name: 'monitoring_metrics',
      path: 'crates/kotoba-monitoring/',
      type: 'operational',
      description: 'Comprehensive monitoring and metrics collection system.',
      dependencies: ['db', 'db_cluster'],
      provides: ['MetricsCollector', 'HealthChecker', 'PerformanceMonitor', 'PrometheusExporter'],
      status: 'completed',
      build_order: 9,
      priority: 'high',
      estimated_effort: '2-3 weeks',
    },

    'config_management': {
      name: 'config_management',
      path: 'crates/kotoba-config/',
      type: 'operational',
      description: 'Configuration management and CLI tools for operational tasks.',
      dependencies: ['db', 'db_cluster'],
      provides: ['ConfigManager', 'AdminCLI', 'ClusterManager', 'MigrationTools'],
      status: 'completed',
      build_order: 9,
      priority: 'medium',
      estimated_effort: '1-2 weeks',
    },

    // パフォーマンス最適化 (Performance Optimization)
    'benchmarking_suite': {
      name: 'benchmarking_suite',
      path: 'crates/kotoba-bench/',
      type: 'performance',
      description: 'Comprehensive benchmarking suite for performance testing.',
      dependencies: ['db', 'db_cluster'],
      provides: ['BenchmarkRunner', 'PerformanceAnalyzer', 'LoadGenerator', 'MetricsReporter'],
      status: 'completed',
      build_order: 10,
      priority: 'high',
      estimated_effort: '2-3 weeks',
    },

    'profiling_tools': {
      name: 'profiling_tools',
      path: 'crates/kotoba-profiler/',
      type: 'performance',
      description: 'Performance profiling and optimization tools.',
      dependencies: ['db', 'benchmarking_suite'],
      provides: ['Profiler', 'MemoryAnalyzer', 'QueryOptimizer', 'PerformanceAdvisor'],
      status: 'completed',
      build_order: 10,
      priority: 'medium',
      estimated_effort: '2-3 weeks',
    },

    'memory_optimization': {
      name: 'memory_optimization',
      path: 'crates/kotoba-memory/',
      type: 'performance',
      description: 'Advanced memory management and optimization features.',
      dependencies: ['db', 'profiling_tools'],
      provides: ['MemoryPool', 'CacheManager', 'MemoryProfiler', 'GCOptimizer'],
      status: 'completed',
      build_order: 11,
      priority: 'medium',
      estimated_effort: '2-3 weeks',
    },

    // テストと品質保証 (Testing & QA)
    'integration_tests': {
      name: 'integration_tests',
      path: 'tests/integration/',
      type: 'testing',
      description: 'Comprehensive integration test suite.',
      dependencies: ['db', 'db_cluster', 'benchmarking_suite'],
      provides: ['IntegrationTestSuite', 'EndToEndTests', 'ClusterTests'],
      status: 'completed',
      build_order: 12,
      priority: 'high',
      estimated_effort: '1-2 weeks',
    },

    'load_testing': {
      name: 'load_testing',
      path: 'tests/load/',
      type: 'testing',
      description: 'Load testing and stress testing framework.',
      dependencies: ['db', 'db_cluster', 'benchmarking_suite'],
      provides: ['LoadTestRunner', 'StressTester', 'ConcurrencyTester', 'ScalabilityTester'],
      status: 'completed',
      build_order: 12,
      priority: 'high',
      estimated_effort: '2-3 weeks',
    },

    'ci_cd_pipeline': {
      name: 'ci_cd_pipeline',
      path: '.github/workflows/',
      type: 'testing',
      description: 'CI/CD pipeline with automated testing and deployment.',
      dependencies: ['integration_tests', 'load_testing', 'benchmarking_suite'],
      provides: ['CIPipeline', 'AutoDeployment', 'QualityGates', 'ReleaseAutomation'],
      status: 'completed',
      build_order: 13,
      priority: 'high',
      estimated_effort: '1-2 weeks',
    },

    // ドキュメント拡張 (Documentation Expansion)
    'api_reference': {
      name: 'api_reference',
      path: 'docs/api/',
      type: 'documentation',
      description: 'Complete API reference documentation.',
      dependencies: ['db', 'db_cluster'],
      provides: ['APIReference', 'CodeExamples', 'TypeDefinitions', 'FunctionIndex'],
      status: 'planned',
      build_order: 14,
      priority: 'medium',
      estimated_effort: '1-2 weeks',
    },

    'deployment_guides': {
      name: 'deployment_guides',
      path: 'docs/deployment/',
      type: 'documentation',
      description: 'Comprehensive deployment and operations guides.',
      dependencies: ['db_cluster', 'config_management'],
      provides: ['DeploymentGuide', 'OperationsManual', 'TroubleshootingGuide', 'BestPractices'],
      status: 'planned',
      build_order: 14,
      priority: 'medium',
      estimated_effort: '1-2 weeks',
    },

    'tutorials': {
      name: 'tutorials',
      path: 'docs/tutorials/',
      type: 'documentation',
      description: 'Step-by-step tutorials and learning resources.',
      dependencies: ['api_reference', 'deployment_guides'],
      provides: ['TutorialSeries', 'QuickStartGuide', 'AdvancedExamples', 'VideoTutorials'],
      status: 'planned',
      build_order: 15,
      priority: 'medium',
      estimated_effort: '1-2 weeks',
    },

    // コミュニティとエコシステム (Community & Ecosystem)
    'sample_applications': {
      name: 'sample_applications',
      path: 'examples/',
      type: 'community',
      description: 'Sample applications and use case demonstrations.',
      dependencies: ['db', 'db_cluster', 'tutorials'],
      provides: ['WebAppDemo', 'AnalyticsApp', 'IoTApplication', 'SocialNetworkDemo'],
      status: 'planned',
      build_order: 16,
      priority: 'medium',
      estimated_effort: '2-3 weeks',
    },

    'contribution_guidelines': {
      name: 'contribution_guidelines',
      path: 'CONTRIBUTING.md',
      type: 'community',
      description: 'Comprehensive contribution guidelines and development workflow.',
      dependencies: ['ci_cd_pipeline', 'integration_tests'],
      provides: ['ContributingGuide', 'DevelopmentWorkflow', 'CodeReviewProcess', 'ReleaseProcess'],
      status: 'planned',
      build_order: 16,
      priority: 'low',
      estimated_effort: '1 week',
    },

    'open_source_release': {
      name: 'open_source_release',
      path: 'RELEASE.md',
      type: 'community',
      description: 'Open source release preparation and community management.',
      dependencies: ['sample_applications', 'contribution_guidelines', 'deployment_guides'],
      provides: ['GitHubRelease', 'CommunityManagement', 'MarketingMaterials', 'RoadmapPlanning'],
      status: 'planned',
      build_order: 17,
      priority: 'high',
      estimated_effort: '2-3 weeks',
    },

    // 先進機能 (Advanced Features)
    'multi_model_support': {
      name: 'multi_model_support',
      path: 'crates/kotoba-multi-model/',
      type: 'advanced',
      description: 'Multi-model database support (documents, time-series, key-value).',
      dependencies: ['db', 'db_cluster'],
      provides: ['DocumentStore', 'TimeSeriesDB', 'KeyValueStore', 'UnifiedAPI'],
      status: 'planned',
      build_order: 18,
      priority: 'medium',
      estimated_effort: '4-6 weeks',
    },

    'machine_learning_integration': {
      name: 'machine_learning_integration',
      path: 'crates/kotoba-ml/',
      type: 'advanced',
      description: 'Machine learning and AI integration capabilities.',
      dependencies: ['db', 'multi_model_support'],
      provides: ['MLPipeline', 'FeatureStore', 'ModelRegistry', 'PredictionAPI'],
      status: 'planned',
      build_order: 19,
      priority: 'low',
      estimated_effort: '6-8 weeks',
    },

    'streaming_processing': {
      name: 'streaming_processing',
      path: 'crates/kotoba-streaming/',
      type: 'advanced',
      description: 'Real-time streaming data processing and analytics.',
      dependencies: ['db_cluster', 'multi_model_support'],
      provides: ['StreamProcessor', 'RealTimeAnalytics', 'EventProcessing', 'CDC'],
      status: 'planned',
      build_order: 20,
      priority: 'low',
      estimated_effort: '4-6 weeks',
    },

    'advanced_query_language': {
      name: 'advanced_query_language',
      path: 'crates/kotoba-query/',
      type: 'advanced',
      description: 'Advanced query language with graph traversals and analytics.',
      dependencies: ['db', 'multi_model_support'],
      provides: ['GraphQL', 'Cypher', 'GQL', 'AnalyticsQueries'],
      status: 'planned',
      build_order: 20,
      priority: 'medium',
      estimated_effort: '4-6 weeks',
    },

    // クラウド統合 (Cloud Integration)
    'cloud_integrations': {
      name: 'cloud_integrations',
      path: 'crates/kotoba-cloud/',
      type: 'cloud',
      description: 'Cloud platform integrations (AWS, GCP, Azure).',
      dependencies: ['db_cluster', 'backup_restore'],
      provides: ['AWSIntegration', 'GCPIntegration', 'AzureIntegration', 'CloudFormation'],
      status: 'planned',
      build_order: 21,
      priority: 'low',
      estimated_effort: '3-4 weeks',
    },

    'serverless_deployment': {
      name: 'serverless_deployment',
      path: 'crates/kotoba-serverless/',
      type: 'cloud',
      description: 'Serverless deployment and scaling capabilities.',
      dependencies: ['cloud_integrations', 'monitoring_metrics'],
      provides: ['LambdaDeployment', 'CloudRun', 'KubernetesOperator', 'AutoScaling'],
      status: 'planned',
      build_order: 22,
      priority: 'low',
      estimated_effort: '4-5 weeks',
    },
  },

  // ==========================================
  // エッジ定義 (Dependencies)
  // ==========================================

  edges: [
    // types -> すべて
    { from: 'types', to: 'ir_catalog' },
    { from: 'types', to: 'schema_validator' },
    { from: 'types', to: 'ir_rule' },
    { from: 'types', to: 'ir_query' },
    { from: 'types', to: 'ir_patch' },
    { from: 'types', to: 'graph_vertex' },
    { from: 'types', to: 'graph_edge' },
    { from: 'types', to: 'storage_mvcc' },
    { from: 'types', to: 'storage_merkle' },
    { from: 'types', to: 'storage_lsm' },
    { from: 'types', to: 'storage_object' },
    { from: 'types', to: 'planner_logical' },
    { from: 'types', to: 'planner_physical' },
    { from: 'types', to: 'execution_parser' },
    { from: 'types', to: 'execution_engine' },
    { from: 'types', to: 'rewrite_matcher' },
    { from: 'types', to: 'rewrite_applier' },
    { from: 'types', to: 'rewrite_engine' },
    { from: 'types', to: 'lib' },
    { from: 'ir_catalog', to: 'lib' },
    { from: 'schema_validator', to: 'lib' },
    { from: 'ir_rule', to: 'lib' },
    { from: 'ir_query', to: 'lib' },
    { from: 'ir_patch', to: 'lib' },
    { from: 'ir_strategy', to: 'lib' },

    // IR相互依存
    { from: 'ir_catalog', to: 'schema_validator' },
    { from: 'types', to: 'ir_strategy' },
    { from: 'ir_patch', to: 'ir_strategy' },
    { from: 'ir_strategy', to: 'rewrite_engine' },

    // Workflow 層依存
    { from: 'types', to: 'ir_workflow' },
    { from: 'ir_strategy', to: 'ir_workflow' },
    { from: 'types', to: 'workflow_executor' },
    { from: 'types', to: 'workflow_store' },
    { from: 'ir_workflow', to: 'workflow_executor' },
    { from: 'ir_workflow', to: 'workflow_store' },
    { from: 'graph_core', to: 'workflow_executor' },
    { from: 'storage_mvcc', to: 'workflow_executor' },
    { from: 'storage_merkle', to: 'workflow_executor' },
    { from: 'execution_engine', to: 'workflow_executor' },
    { from: 'storage_mvcc', to: 'workflow_store' },
    { from: 'storage_merkle', to: 'workflow_store' },

    // Phase 4: Ecosystem 依存
    { from: 'types', to: 'workflow_designer' },
    { from: 'types', to: 'activity_libraries' },
    { from: 'ir_workflow', to: 'activity_libraries' },
    { from: 'workflow_executor', to: 'activity_libraries' },
    { from: 'types', to: 'kubernetes_operator' },
    { from: 'ir_workflow', to: 'kubernetes_operator' },
    { from: 'workflow_executor', to: 'kubernetes_operator' },
    { from: 'workflow_store', to: 'kubernetes_operator' },
    { from: 'types', to: 'cloud_integrations' },

    // グラフ層依存
    { from: 'types', to: 'graph_core' },
    { from: 'graph_vertex', to: 'graph_core' },
    { from: 'graph_edge', to: 'graph_core' },
    { from: 'graph_core', to: 'storage_mvcc' },
    { from: 'graph_core', to: 'storage_merkle' },
    { from: 'graph_core', to: 'planner_logical' },
    { from: 'graph_core', to: 'planner_physical' },
    { from: 'graph_core', to: 'execution_engine' },
    { from: 'graph_core', to: 'rewrite_matcher' },
    { from: 'graph_core', to: 'rewrite_applier' },
    { from: 'graph_core', to: 'rewrite_engine' },
    { from: 'graph_core', to: 'lib' },

    // ストレージ層依存
    { from: 'storage_mvcc', to: 'execution_engine' },
    { from: 'storage_mvcc', to: 'rewrite_engine' },
    { from: 'storage_mvcc', to: 'lib' },
    { from: 'storage_merkle', to: 'execution_engine' },
    { from: 'storage_merkle', to: 'rewrite_engine' },
    { from: 'storage_merkle', to: 'lib' },
    { from: 'storage_lsm', to: 'lib' },
    { from: 'storage_object', to: 'lib' },
    { from: 'security_core', to: 'lib' },

    // プランナー層依存
    { from: 'ir_query', to: 'planner_logical' },
    { from: 'ir_catalog', to: 'planner_logical' },
    { from: 'ir_query', to: 'planner_physical' },
    { from: 'ir_catalog', to: 'planner_physical' },
    { from: 'types', to: 'planner_optimizer' },
    { from: 'ir_query', to: 'planner_optimizer' },
    { from: 'ir_catalog', to: 'planner_optimizer' },
    { from: 'graph_core', to: 'planner_optimizer' },
    { from: 'planner_logical', to: 'planner_optimizer' },
    { from: 'planner_physical', to: 'planner_optimizer' },
    { from: 'planner_logical', to: 'execution_engine' },
    { from: 'planner_physical', to: 'execution_engine' },
    { from: 'planner_optimizer', to: 'execution_engine' },
    { from: 'ir_query', to: 'execution_engine' },
    { from: 'ir_catalog', to: 'execution_engine' },
    { from: 'planner_logical', to: 'lib' },
    { from: 'planner_physical', to: 'lib' },
    { from: 'planner_optimizer', to: 'lib' },

    // 実行層依存
    { from: 'ir_query', to: 'execution_parser' },
    { from: 'execution_parser', to: 'execution_engine' },
    { from: 'execution_parser', to: 'lib' },
    { from: 'execution_engine', to: 'lib' },

    // 書換え層依存
    { from: 'ir_rule', to: 'rewrite_matcher' },
    { from: 'ir_catalog', to: 'rewrite_matcher' },
    { from: 'ir_rule', to: 'rewrite_applier' },
    { from: 'ir_patch', to: 'rewrite_applier' },
    { from: 'ir_rule', to: 'rewrite_engine' },
    { from: 'rewrite_matcher', to: 'rewrite_engine' },
    { from: 'rewrite_applier', to: 'rewrite_engine' },
    { from: 'rewrite_matcher', to: 'lib' },
    { from: 'rewrite_applier', to: 'lib' },
    { from: 'rewrite_engine', to: 'lib' },

    // セキュリティ層依存
    { from: 'types', to: 'security_jwt' },
    { from: 'types', to: 'security_oauth2' },
    { from: 'security_jwt', to: 'security_oauth2' },
    { from: 'types', to: 'security_mfa' },
    { from: 'types', to: 'security_password' },
    { from: 'types', to: 'security_session' },
    { from: 'types', to: 'security_capabilities' },
    { from: 'security_jwt', to: 'security_core' },
    { from: 'security_oauth2', to: 'security_core' },
    { from: 'security_mfa', to: 'security_core' },
    { from: 'security_password', to: 'security_core' },
    { from: 'security_session', to: 'security_core' },
    { from: 'security_capabilities', to: 'security_core' },
    { from: 'security_core', to: 'http_ir' },

    // ==========================================
    // Jsonnet 0.21.0 依存関係
    // ==========================================

    // Jsonnet error dependencies
    { from: 'jsonnet_error', to: 'jsonnet_value' },
    { from: 'jsonnet_error', to: 'jsonnet_lexer' },

    // Jsonnet value dependencies
    { from: 'jsonnet_value', to: 'jsonnet_ast' },
    { from: 'jsonnet_value', to: 'jsonnet_evaluator' },
    { from: 'jsonnet_value', to: 'jsonnet_stdlib' },

    // Jsonnet AST dependencies
    { from: 'jsonnet_ast', to: 'jsonnet_parser' },
    { from: 'jsonnet_ast', to: 'jsonnet_evaluator' },

    // Jsonnet lexer dependencies
    { from: 'jsonnet_lexer', to: 'jsonnet_parser' },

    // Jsonnet parser dependencies
    { from: 'jsonnet_parser', to: 'jsonnet_core' },

    // Jsonnet evaluator dependencies
    { from: 'jsonnet_evaluator', to: 'jsonnet_core' },

    // Jsonnet stdlib dependencies
    { from: 'jsonnet_stdlib', to: 'jsonnet_core' },

    // Integration with main library
    { from: 'jsonnet_core', to: 'lib' },
    { from: 'jsonnet_core', to: 'http_parser' },  // Jsonnet parser integration

    // ==========================================
    // Kotoba Kotobanet 依存関係
    // ==========================================

    // Kotobanet error dependencies
    { from: 'kotobanet_error', to: 'kotobanet_http_parser' },
    { from: 'kotobanet_error', to: 'kotobanet_frontend' },
    { from: 'kotobanet_error', to: 'kotobanet_deploy' },
    { from: 'kotobanet_error', to: 'kotobanet_config' },
    { from: 'kotobanet_error', to: 'kotobanet_core' },

    // Kotobanet components dependencies
    { from: 'jsonnet_core', to: 'kotobanet_http_parser' },
    { from: 'jsonnet_core', to: 'kotobanet_frontend' },
    { from: 'jsonnet_core', to: 'kotobanet_deploy' },
    { from: 'jsonnet_core', to: 'kotobanet_config' },

    // Kotobanet core dependencies
    { from: 'kotobanet_http_parser', to: 'kotobanet_core' },
    { from: 'kotobanet_frontend', to: 'kotobanet_core' },
    { from: 'kotobanet_deploy', to: 'kotobanet_core' },
    { from: 'kotobanet_config', to: 'kotobanet_core' },

    // Integration with other components
    { from: 'kotobanet_core', to: 'lib' },
    { from: 'kotobanet_http_parser', to: 'http_parser' },  // HTTP parser enhancement
    { from: 'kotobanet_frontend', to: 'frontend_framework' },  // Frontend enhancement
    { from: 'kotobanet_deploy', to: 'deploy_parser' },  // Deploy enhancement
    { from: 'kotobanet_config', to: 'deploy_config' },  // Config enhancement

    // HTTPサーバー層依存
    { from: 'types', to: 'http_ir' },
    { from: 'ir_catalog', to: 'http_ir' },
    { from: 'security_core', to: 'http_ir' },
    { from: 'http_ir', to: 'http_parser' },
    { from: 'types', to: 'http_parser' },
    { from: 'http_ir', to: 'http_handlers' },
    { from: 'graph_core', to: 'http_handlers' },
    { from: 'rewrite_engine', to: 'http_handlers' },
    { from: 'storage_mvcc', to: 'http_handlers' },
    { from: 'storage_merkle', to: 'http_handlers' },
    { from: 'security_core', to: 'http_handlers' },
    { from: 'http_ir', to: 'http_engine' },
    { from: 'http_handlers', to: 'http_engine' },
    { from: 'graph_core', to: 'http_engine' },
    { from: 'storage_mvcc', to: 'http_engine' },
    { from: 'storage_merkle', to: 'http_engine' },
    { from: 'rewrite_engine', to: 'http_engine' },
    { from: 'security_core', to: 'http_engine' },
    { from: 'http_ir', to: 'http_server' },
    { from: 'http_parser', to: 'http_server' },
    { from: 'http_engine', to: 'http_server' },
    { from: 'http_handlers', to: 'http_server' },

    // GraphQL dependencies
    { from: 'types', to: 'graphql_schema' },
    { from: 'schema_validator', to: 'graphql_schema' },
    { from: 'types', to: 'graphql_handler' },
    { from: 'graphql_schema', to: 'graphql_handler' },
    { from: 'graphql_schema', to: 'http_server' },
    { from: 'graphql_handler', to: 'http_server' },
    { from: 'http_ir', to: 'lib' },
    { from: 'http_parser', to: 'lib' },
    { from: 'http_handlers', to: 'lib' },
    { from: 'http_engine', to: 'lib' },
    { from: 'http_server', to: 'lib' },

    // フロントエンドフレームワーク層依存
    { from: 'types', to: 'frontend_component_ir' },
    { from: 'types', to: 'frontend_route_ir' },
    { from: 'frontend_component_ir', to: 'frontend_route_ir' },
    { from: 'types', to: 'frontend_render_ir' },
    { from: 'frontend_component_ir', to: 'frontend_render_ir' },
    { from: 'types', to: 'frontend_build_ir' },
    { from: 'frontend_component_ir', to: 'frontend_build_ir' },
    { from: 'types', to: 'frontend_api_ir' },
    { from: 'types', to: 'frontend_framework' },
    { from: 'frontend_component_ir', to: 'frontend_framework' },
    { from: 'frontend_route_ir', to: 'frontend_framework' },
    { from: 'frontend_render_ir', to: 'frontend_framework' },
    { from: 'frontend_build_ir', to: 'frontend_framework' },
    { from: 'frontend_api_ir', to: 'frontend_framework' },
    { from: 'http_ir', to: 'frontend_framework' },
    { from: 'frontend_component_ir', to: 'lib' },
    { from: 'frontend_route_ir', to: 'lib' },
    { from: 'frontend_render_ir', to: 'lib' },
    { from: 'frontend_build_ir', to: 'lib' },
    { from: 'frontend_api_ir', to: 'lib' },
    { from: 'frontend_framework', to: 'lib' },

    // Examples層依存
    { from: 'lib', to: 'example_frontend_app' },
    { from: 'frontend_framework', to: 'example_frontend_app' },
    { from: 'http_server', to: 'example_frontend_app' },
    { from: 'lib', to: 'example_http_server' },
    { from: 'http_server', to: 'example_http_server' },
    { from: 'lib', to: 'example_social_network' },
    { from: 'graph_core', to: 'example_social_network' },
    { from: 'execution_engine', to: 'example_social_network' },
    { from: 'rewrite_engine', to: 'example_social_network' },
    { from: 'lib', to: 'example_tauri_react_app' },
    { from: 'frontend_framework', to: 'example_tauri_react_app' },
    { from: 'graph_core', to: 'example_tauri_react_app' },
    { from: 'storage_mvcc', to: 'example_tauri_react_app' },
    { from: 'storage_merkle', to: 'example_tauri_react_app' },

    // ==========================================
    // Deploy層依存関係
    // ==========================================

    // Deploy config dependencies
    { from: 'types', to: 'deploy_config' },
    { from: 'deploy_config', to: 'deploy_parser' },
    { from: 'types', to: 'deploy_parser' },

    // Deploy scaling dependencies
    { from: 'types', to: 'deploy_scaling' },
    { from: 'deploy_config', to: 'deploy_scaling' },
    { from: 'graph_core', to: 'deploy_scaling' },

    // Deploy network dependencies
    { from: 'types', to: 'deploy_network' },
    { from: 'deploy_config', to: 'deploy_network' },
    { from: 'deploy_scaling', to: 'deploy_network' },

    // Deploy git integration dependencies
    { from: 'types', to: 'deploy_git_integration' },
    { from: 'deploy_config', to: 'deploy_git_integration' },
    { from: 'deploy_network', to: 'deploy_git_integration' },

    // Deploy controller dependencies
    { from: 'types', to: 'deploy_controller' },
    { from: 'deploy_config', to: 'deploy_controller' },
    { from: 'deploy_scaling', to: 'deploy_controller' },
    { from: 'deploy_network', to: 'deploy_controller' },
    { from: 'deploy_git_integration', to: 'deploy_controller' },
    { from: 'graph_core', to: 'deploy_controller' },
    { from: 'rewrite_engine', to: 'deploy_controller' },

    // Deploy CLI dependencies
    { from: 'types', to: 'deploy_cli' },
    { from: 'deploy_controller', to: 'deploy_cli' },
    { from: 'http_server', to: 'deploy_cli' },

    // Deploy runtime dependencies
    { from: 'types', to: 'deploy_runtime' },
    { from: 'deploy_controller', to: 'deploy_runtime' },
    { from: 'wasm', to: 'deploy_runtime' },

    // Deploy examples dependencies
    { from: 'deploy_config', to: 'deploy_example_simple' },
    { from: 'deploy_config', to: 'deploy_example_microservices' },
    { from: 'deploy_example_simple', to: 'deploy_example_microservices' },

    // Integration with main library
    { from: 'deploy_config', to: 'lib' },
    { from: 'deploy_parser', to: 'lib' },
    { from: 'deploy_scaling', to: 'lib' },
    { from: 'deploy_network', to: 'lib' },
    { from: 'deploy_git_integration', to: 'lib' },
    { from: 'deploy_controller', to: 'lib' },
    { from: 'deploy_cli', to: 'lib' },
    { from: 'deploy_runtime', to: 'lib' },

    // Hosting Server dependencies
    { from: 'deploy_controller', to: 'deploy_hosting_server' },
    { from: 'http_server', to: 'deploy_hosting_server' },
    { from: 'frontend_framework', to: 'deploy_hosting_server' },
    { from: 'graph_core', to: 'deploy_hosting_server' },
    { from: 'execution_engine', to: 'deploy_hosting_server' },
    { from: 'storage_mvcc', to: 'deploy_hosting_server' },
    { from: 'storage_merkle', to: 'deploy_hosting_server' },
    { from: 'deploy_hosting_server', to: 'deploy_hosting_manager' },
    { from: 'deploy_scaling', to: 'deploy_hosting_manager' },
    { from: 'deploy_network', to: 'deploy_hosting_manager' },
    { from: 'deploy_hosting_manager', to: 'deploy_hosting_example' },
    { from: 'deploy_cli', to: 'deploy_hosting_example' },

    // Hosting integration with main library
    { from: 'deploy_hosting_server', to: 'lib' },
    { from: 'deploy_hosting_manager', to: 'lib' },

    // ==========================================
    // AI Agent 層依存関係
    // ==========================================

    // AI agent parser dependencies
    { from: 'jsonnet_core', to: 'ai_agent_parser' },
    { from: 'kotobanet_core', to: 'ai_agent_parser' },

    // DB handler dependencies
    { from: 'jsonnet_core', to: 'db_handler' },
    { from: 'execution_engine', to: 'db_handler' },
    { from: 'rewrite_engine', to: 'db_handler' },

    // AI runtime dependencies
    { from: 'ai_agent_parser', to: 'ai_runtime' },
    { from: 'jsonnet_core', to: 'ai_runtime' },
    { from: 'http_ir', to: 'ai_runtime' },
    { from: 'db_handler', to: 'ai_runtime' },

    // AI models dependencies
    { from: 'ai_runtime', to: 'ai_models' },
    { from: 'jsonnet_core', to: 'ai_models' },

    // AI tools dependencies
    { from: 'ai_runtime', to: 'ai_tools' },
    { from: 'jsonnet_core', to: 'ai_tools' },

    // AI memory dependencies
    { from: 'ai_runtime', to: 'ai_memory' },
    { from: 'storage_mvcc', to: 'ai_memory' },
    { from: 'storage_merkle', to: 'ai_memory' },
    { from: 'db_handler', to: 'ai_memory' },

    // AI chains dependencies
    { from: 'ai_agent_parser', to: 'ai_chains' },
    { from: 'ai_runtime', to: 'ai_chains' },
    { from: 'ai_models', to: 'ai_chains' },
    { from: 'ai_tools', to: 'ai_chains' },
    { from: 'ai_memory', to: 'ai_chains' },

    // AI examples dependencies
    { from: 'ai_chains', to: 'ai_examples' },
    { from: 'ai_models', to: 'ai_examples' },
    { from: 'ai_tools', to: 'ai_examples' },
    { from: 'ai_memory', to: 'ai_examples' },

    // Integration with main library
    { from: 'ai_agent_parser', to: 'lib' },
    { from: 'ai_runtime', to: 'lib' },
    { from: 'ai_models', to: 'lib' },
    { from: 'ai_tools', to: 'lib' },
    { from: 'ai_memory', to: 'lib' },
    { from: 'ai_chains', to: 'lib' },

    // ==========================================
    // Deploy拡張機能の依存関係
    // ==========================================

    // CLI拡張の依存関係
    { from: 'types', to: 'deploy_cli_core' },
    { from: 'deploy_controller', to: 'deploy_cli_core' },
    { from: 'http_server', to: 'deploy_cli_core' },
    { from: 'deploy_cli_core', to: 'deploy_cli_binary' },
    { from: 'deploy_controller', to: 'deploy_cli_binary' },
    { from: 'deploy_scaling', to: 'deploy_cli_binary' },
    { from: 'deploy_network', to: 'deploy_cli_binary' },
    { from: 'deploy_runtime', to: 'deploy_cli_binary' },

    // コントローラー拡張の依存関係
    { from: 'types', to: 'deploy_controller_core' },
    { from: 'deploy_config', to: 'deploy_controller_core' },
    { from: 'deploy_scaling', to: 'deploy_controller_core' },
    { from: 'deploy_network', to: 'deploy_controller_core' },
    { from: 'deploy_git_integration', to: 'deploy_controller_core' },
    { from: 'graph_core', to: 'deploy_controller_core' },
    { from: 'rewrite_engine', to: 'deploy_controller_core' },

    // ネットワーク拡張の依存関係
    { from: 'types', to: 'deploy_network_core' },
    { from: 'deploy_config', to: 'deploy_network_core' },
    { from: 'deploy_scaling', to: 'deploy_network_core' },

    // スケーリング拡張の依存関係（準備中）
    { from: 'types', to: 'deploy_scaling_core' },
    { from: 'deploy_config', to: 'deploy_scaling_core' },
    { from: 'graph_core', to: 'deploy_scaling_core' },

    // Hosting Serverの更新された依存関係
    { from: 'deploy_controller_core', to: 'deploy_hosting_server' },
    { from: 'deploy_controller_core', to: 'deploy_hosting_manager' },

    // CLI拡張の統合
    { from: 'deploy_cli_core', to: 'lib' },
    { from: 'deploy_cli_binary', to: 'lib' },
    { from: 'deploy_controller_core', to: 'lib' },
    { from: 'deploy_network_core', to: 'lib' },
    { from: 'deploy_scaling_core', to: 'lib' },

    // ==========================================
    // 新規クレートの依存関係
    // ==========================================

    // Distributed engine dependencies
    { from: 'types', to: 'distributed_engine' },
    { from: 'graph_core', to: 'distributed_engine' },
    { from: 'execution_engine', to: 'distributed_engine' },
    { from: 'rewrite_engine', to: 'distributed_engine' },
    { from: 'storage_mvcc', to: 'distributed_engine' },
    { from: 'storage_merkle', to: 'distributed_engine' },

    // Network protocol dependencies
    { from: 'types', to: 'network_protocol' },
    { from: 'distributed_engine', to: 'network_protocol' },

    // CID system dependencies
    { from: 'types', to: 'cid_system' },

    // CLI interface dependencies
    { from: 'types', to: 'cli_interface' },
    { from: 'distributed_engine', to: 'cli_interface' },
    { from: 'network_protocol', to: 'cli_interface' },
    { from: 'cid_system', to: 'cli_interface' },

    // Integration with main library
    { from: 'distributed_engine', to: 'lib' },
    { from: 'network_protocol', to: 'lib' },
    { from: 'cid_system', to: 'lib' },
    { from: 'cli_interface', to: 'lib' },

    // LSP server dependencies
    { from: 'kotobanet_core', to: 'kotoba_lsp' },
    { from: 'jsonnet_core', to: 'kotoba_lsp' },

    // Package manager dependencies
    { from: 'types', to: 'package_manager' },
    { from: 'cid_system', to: 'package_manager' },

    // Integration with main library
    { from: 'distributed_engine', to: 'lib' },
    { from: 'network_protocol', to: 'lib' },

    // State Graph Library dependencies
    { from: 'types', to: 'state_graph_lib' },
    { from: 'rewrite_engine', to: 'state_graph_lib' },
    { from: 'execution_engine', to: 'state_graph_lib' },
    { from: 'state_graph_lib', to: 'lib' },

    // ==========================================
    // Kotoba Documentation Generator Dependencies
    // ==========================================

    // Documentation parser dependencies
    { from: 'types', to: 'docs_parser' },
    { from: 'docs_parser', to: 'docs_generator' },
    { from: 'docs_parser', to: 'docs_search' },
    { from: 'docs_parser', to: 'docs_core' },

    // Documentation config dependencies
    { from: 'types', to: 'docs_config' },
    { from: 'docs_config', to: 'docs_generator' },
    { from: 'docs_config', to: 'docs_core' },

    // Documentation generator dependencies
    { from: 'types', to: 'docs_generator' },
    { from: 'docs_generator', to: 'docs_template' },
    { from: 'docs_generator', to: 'docs_server' },
    { from: 'docs_generator', to: 'docs_core' },

    // Documentation template dependencies
    { from: 'types', to: 'docs_template' },
    { from: 'docs_template', to: 'docs_core' },

    // Documentation search dependencies
    { from: 'types', to: 'docs_search' },
    { from: 'docs_search', to: 'docs_server' },
    { from: 'docs_search', to: 'docs_core' },

    // Documentation server dependencies
    { from: 'types', to: 'docs_server' },
    { from: 'http_ir', to: 'docs_server' },
    { from: 'docs_server', to: 'docs_core' },

    // Documentation core dependencies
    { from: 'types', to: 'docs_core' },
    { from: 'docs_core', to: 'docs_cli' },
    { from: 'docs_core', to: 'lib' },

    // Documentation CLI dependencies
    { from: 'types', to: 'docs_cli' },
    { from: 'cli_interface', to: 'docs_cli' },
    { from: 'docs_cli', to: 'lib' },

    // KotobaDB dependencies
    { from: 'types', to: 'db_core' },
    { from: 'cid_system', to: 'db_core' },
    { from: 'db_core', to: 'db_engine_memory' },
    { from: 'db_core', to: 'db_engine_lsm' },
    { from: 'db_core', to: 'db' },
    { from: 'db_engine_memory', to: 'db' },
    { from: 'db_engine_lsm', to: 'db' },

    // Distributed cluster dependencies
    { from: 'db_core', to: 'db_cluster' },
    { from: 'db', to: 'db_cluster' },

    // Storage integration
    { from: 'types', to: 'storage_main' },
    { from: 'errors', to: 'storage_main' },
    { from: 'graph', to: 'storage_main' },
    { from: 'cid', to: 'storage_main' },
    { from: 'db', to: 'storage_main' },

    // ==========================================
    // Future Features Dependencies
    // ==========================================

    // Operational Features
    { from: 'db', to: 'backup_restore' },
    { from: 'storage_main', to: 'backup_restore' },

    { from: 'db', to: 'monitoring_metrics' },
    { from: 'db_cluster', to: 'monitoring_metrics' },

    { from: 'db', to: 'config_management' },
    { from: 'db_cluster', to: 'config_management' },

    // Performance Optimization
    { from: 'db', to: 'benchmarking_suite' },
    { from: 'db_cluster', to: 'benchmarking_suite' },

    { from: 'db', to: 'profiling_tools' },
    { from: 'benchmarking_suite', to: 'profiling_tools' },

    { from: 'db', to: 'memory_optimization' },
    { from: 'profiling_tools', to: 'memory_optimization' },

    // Testing & QA
    { from: 'db', to: 'integration_tests' },
    { from: 'db_cluster', to: 'integration_tests' },
    { from: 'benchmarking_suite', to: 'integration_tests' },

    { from: 'db', to: 'load_testing' },
    { from: 'db_cluster', to: 'load_testing' },
    { from: 'benchmarking_suite', to: 'load_testing' },

    { from: 'integration_tests', to: 'ci_cd_pipeline' },
    { from: 'load_testing', to: 'ci_cd_pipeline' },
    { from: 'benchmarking_suite', to: 'ci_cd_pipeline' },

    // Documentation Expansion
    { from: 'db', to: 'api_reference' },
    { from: 'db_cluster', to: 'api_reference' },

    { from: 'db_cluster', to: 'deployment_guides' },
    { from: 'config_management', to: 'deployment_guides' },

    { from: 'api_reference', to: 'tutorials' },
    { from: 'deployment_guides', to: 'tutorials' },

    // Community & Ecosystem
    { from: 'db', to: 'sample_applications' },
    { from: 'db_cluster', to: 'sample_applications' },
    { from: 'tutorials', to: 'sample_applications' },

    { from: 'ci_cd_pipeline', to: 'contribution_guidelines' },
    { from: 'integration_tests', to: 'contribution_guidelines' },

    { from: 'sample_applications', to: 'open_source_release' },
    { from: 'contribution_guidelines', to: 'open_source_release' },
    { from: 'deployment_guides', to: 'open_source_release' },

    // Advanced Features
    { from: 'db', to: 'multi_model_support' },
    { from: 'db_cluster', to: 'multi_model_support' },

    { from: 'db', to: 'machine_learning_integration' },
    { from: 'multi_model_support', to: 'machine_learning_integration' },

    { from: 'db_cluster', to: 'streaming_processing' },
    { from: 'multi_model_support', to: 'streaming_processing' },

    { from: 'db', to: 'advanced_query_language' },
    { from: 'multi_model_support', to: 'advanced_query_language' },

    // Cloud Integration
    { from: 'db_cluster', to: 'cloud_integrations' },
    { from: 'backup_restore', to: 'cloud_integrations' },

    { from: 'cloud_integrations', to: 'serverless_deployment' },
    { from: 'monitoring_metrics', to: 'serverless_deployment' },

    // LSP server dependencies
    { from: 'kotobanet_core', to: 'kotoba_lsp' },

    // ==========================================
    // Static Site Generator 依存関係
    // ==========================================

    // Markdown parser dependencies
    { from: 'types', to: 'markdown_parser' },
    { from: 'jsonnet_core', to: 'markdown_parser' },

    // HTML template engine dependencies
    { from: 'types', to: 'html_template_engine' },
    { from: 'jsonnet_core', to: 'html_template_engine' },
    { from: 'markdown_parser', to: 'html_template_engine' },

    // Static site generator dependencies
    { from: 'types', to: 'static_site_generator' },
    { from: 'jsonnet_core', to: 'static_site_generator' },
    { from: 'markdown_parser', to: 'static_site_generator' },
    { from: 'html_template_engine', to: 'static_site_generator' },
    { from: 'http_ir', to: 'static_site_generator' },

    // GitHub Pages deployer dependencies
    { from: 'types', to: 'github_pages_deployer' },
    { from: 'static_site_generator', to: 'github_pages_deployer' },
    { from: 'http_ir', to: 'github_pages_deployer' },

    // Documentation builder dependencies
    { from: 'types', to: 'documentation_builder' },
    { from: 'static_site_generator', to: 'documentation_builder' },
    { from: 'docs_core', to: 'documentation_builder' },

    // SSG integration with main library
    { from: 'markdown_parser', to: 'lib' },
    { from: 'html_template_engine', to: 'lib' },
    { from: 'static_site_generator', to: 'lib' },
    { from: 'github_pages_deployer', to: 'lib' },
    { from: 'documentation_builder', to: 'lib' },
  ],

  // ==========================================
  // トポロジカルソート (ビルド順序)
  // ==========================================

  topological_order: [
    'types',
    'jsonnet_error',
    'ir_catalog',
    'ir_rule',
    'ir_query',
    'ir_patch',
    'graph_vertex',
    'graph_edge',
    'jsonnet_value',
    'cid_system',
    'schema_validator',
    'ir_strategy',
    'frontend_component_ir',
    'docs_parser',
    'docs_config',
    'jsonnet_ast',
    'jsonnet_lexer',
    'graph_core',
    'storage_main',
    'db_core',
    'db_engine_memory',
    'db_engine_lsm',
    'db_cluster',

    // Future Features (planned)
    'backup_restore',
    'monitoring_metrics',
    'config_management',
    'benchmarking_suite',
    'profiling_tools',
    'memory_optimization',
    'integration_tests',
    'load_testing',
    'ci_cd_pipeline',
    'api_reference',
    'deployment_guides',
    'tutorials',
    'sample_applications',
    'contribution_guidelines',
    'open_source_release',
    'multi_model_support',
    'machine_learning_integration',
    'streaming_processing',
    'advanced_query_language',
    'cloud_integrations',
    'serverless_deployment',

    'package_manager',
    'storage_mvcc',
    'security_jwt',
    'security_mfa',
    'security_password',
    'security_session',
    'security_capabilities',
    'security_oauth2',
    'security_core',
    'planner_logical',
    'planner_physical',
    'execution_parser',
    'rewrite_matcher',
    'rewrite_applier',
    'planner_optimizer',
    'rewrite_engine',
    'execution_engine',
    'workflow_designer',
    'activity_libraries',
    'kubernetes_operator',
    'cloud_integrations',
    'distributed_engine',
    'network_protocol',
    'cli_interface',
    'kotoba_lsp',
    'http_ir',
    'http_parser',
    'http_handlers',
    'http_engine',
    'http_server',
    'deploy_config',
    'deploy_parser',
    'deploy_scaling',
    'deploy_network',
    'deploy_git_integration',
    'frontend_framework',
    'deploy_controller',
    'graphql_schema',
    'graphql_handler',
    'deploy_cli',
    'deploy_runtime',
    'deploy_example_simple',
    'deploy_example_microservices',
    'deploy_cli_core',
    'deploy_cli_binary',
    'deploy_controller_core',
    'deploy_network_core',
    'deploy_scaling_core',
    'deploy_cli_binary',
    'deploy_hosting_server',
    'deploy_hosting_manager',
    'deploy_hosting_example',
    'ai_agent_parser',
    'db_handler',
    'ai_runtime',
    'ai_models',
    'ai_tools',
    'ai_memory',
    'ai_chains',
    'ai_examples',
    'state_graph_lib',
    'docs_generator',
    'docs_search',
    'storage_main',

    // Future Features (reverse topological order)
    'serverless_deployment',
    'cloud_integrations',
    'advanced_query_language',
    'streaming_processing',
    'machine_learning_integration',
    'multi_model_support',
    'open_source_release',
    'contribution_guidelines',
    'sample_applications',
    'tutorials',
    'deployment_guides',
    'api_reference',
    'ci_cd_pipeline',
    'load_testing',
    'integration_tests',
    'memory_optimization',
    'profiling_tools',
    'benchmarking_suite',
    'config_management',
    'monitoring_metrics',
    'backup_restore',

    'db_cluster',
    'db_engine_lsm',
    'db_engine_memory',
    'planner_logical',
    'planner_physical',
    'jsonnet_parser',
    'frontend_route_ir',
    'frontend_render_ir',
    'frontend_build_ir',
    'frontend_api_ir',
    'docs_template',
    'db',
    'docs_server',
    'docs_core',
    'docs_cli',
  ],

  // ==========================================
  // 逆トポロジカルソート (問題解決順序)
  // ==========================================

  reverse_topological_order: [
    'documentation_builder',
    'github_pages_deployer',
    'static_site_generator',
    'html_template_engine',
    'markdown_parser',
    'db',
    'db_engine_memory',
    'db_core',
    'ai_examples',
    'ai_chains',
    'ai_memory',
    'ai_tools',
    'ai_models',
    'ai_runtime',
    'db_handler',
    'ai_agent_parser',
    'deploy_hosting_example',
    'deploy_hosting_manager',
    'deploy_hosting_server',
    'deploy_cli_binary',
    'deploy_scaling_core',
    'deploy_network_core',
    'deploy_controller_core',
    'deploy_cli_core',
    'docs_cli',
    'docs_core',
    'docs_server',
    'docs_search',
    'docs_template',
    'docs_generator',
    'docs_config',
    'docs_parser',
    'deploy_example_microservices',
    'deploy_example_simple',
    'example_tauri_react_app',
    'example_social_network',
    'example_http_server',
    'example_frontend_app',
    'lib',

    // SSG layer (build_order: 15-19)
    'markdown_parser',
    'html_template_engine',
    'static_site_generator',
    'github_pages_deployer',
    'documentation_builder',

    'cli_interface',
    'kotoba_lsp',
    'deploy_runtime',
    'deploy_cli',
    'http_server',
    'graphql_handler',
    'graphql_schema',
    'deploy_controller',
    'frontend_framework',
    'deploy_git_integration',
    'deploy_network',
    'http_engine',
    'http_handlers',
    'deploy_scaling',
    'deploy_parser',
    'http_parser',
    'deploy_config',
    'frontend_api_ir',
    'frontend_build_ir',
    'frontend_render_ir',
    'frontend_route_ir',
    'frontend_component_ir',
    'http_ir',
    'execution_engine',
    'network_protocol',
    'distributed_engine',
    'cloud_integrations',
    'kubernetes_operator',
    'activity_libraries',
    'workflow_designer',
    'rewrite_engine',
    'planner_optimizer',
    'rewrite_applier',
    'rewrite_matcher',
    'execution_parser',
    'planner_physical',
    'planner_logical',
    'storage_lsm',
    'storage_object',
    'storage_merkle',
    'storage_mvcc',
    'graph_core',
    'ir_strategy',
    'graph_edge',
    'graph_vertex',
    'ir_patch',
    'ir_query',
    'ir_rule',
    'ir_catalog',
    'schema_validator',
    'jsonnet_core',
    'kotobanet_core',
    'kotobanet_config',
    'kotobanet_deploy',
    'kotobanet_frontend',
    'kotobanet_http_parser',
    'kotobanet_error',
    'jsonnet_stdlib',
    'jsonnet_evaluator',
    'jsonnet_parser',
    'jsonnet_lexer',
    'jsonnet_ast',
    'jsonnet_value',
    'jsonnet_error',
    'cid_system',
    'types',
    'package_manager',
    'state_graph_lib',
  ],

  // ==========================================
  // ユーティリティ関数
  // ==========================================

  // 指定されたノードの依存関係を取得
  get_dependencies(node_name)::
    [edge.from for edge in self.edges if edge.to == node_name],

  // 指定されたノードが依存しているノードを取得
  get_dependents(node_name)::
    [edge.to for edge in self.edges if edge.from == node_name],

  // 指定されたノードの情報を取得
  get_node(node_name)::
    self.nodes[node_name],

  // 指定されたタイプのノードを取得
  get_nodes_by_type(node_type)::
    [node for node in std.objectValues(self.nodes) if node.type == node_type],

  // ビルド順序でソートされたノードを取得
  get_nodes_in_build_order()::
    [self.nodes[name] for name in self.topological_order],

  // 問題解決順序でソートされたノードを取得
  get_nodes_in_problem_resolution_order()::
    [self.nodes[name] for name in self.reverse_topological_order],

  // 指定されたノードのビルド順序を取得
  get_build_order(node_name)::
    self.nodes[node_name].build_order,

  // 循環依存がないかチェック
  validate_dag()::
    local node_names = std.objectFields(self.nodes);
    local edge_count = std.length(self.edges);
    local expected_edges = std.length(node_names) - 1;
    if edge_count > expected_edges then
      error '循環依存の可能性があります'
    else
      'DAGは有効です',

  // ノードの状態サマリー
  get_status_summary()::
    local completed = std.length([n for n in std.objectValues(self.nodes) if n.status == 'completed']);
    local total = std.length(std.objectValues(self.nodes));
    {
      completed: completed,
      total: total,
      completion_rate: completed / total * 100,
    },

  // ==========================================
  // メタデータ
  // ==========================================

  metadata: {
    project_name: 'Kotoba',
    description: 'GP2系グラフ書換え言語 - ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム + 高度なデプロイ拡張機能 + ドキュメントジェネレータ',
    version: '0.1.0',
    architecture: 'Process Network Graph Model',
    created_at: '2025-01-12',
    last_updated: '2025-09-17',
    author: 'jun784',

    deploy_extensions: {
      description: '高度なデプロイメント拡張機能群',
      version: '0.1.0',
      last_updated: '2025-09-17',

      cli_extension: {
        name: 'CLI拡張',
        description: '完全なデプロイメント管理CLI - 設定ファイル処理、進捗バー、詳細オプション',
        components: [
          'deploy_cli_core',
          'deploy_cli_binary'
        ],
        features: [
          'デプロイメント設定管理',
          '進捗バー表示',
          'JSON/YAML/人間可読形式出力',
          '設定ファイル自動生成',
          'デプロイメント履歴管理',
          'ステータス監視'
        ],
        status: 'completed'
      },

      controller_extension: {
        name: 'コントローラー拡張',
        description: '高度なデプロイコントローラー - ロールバック、ブルーグリーン、カナリアデプロイ',
        components: [
          'deploy_controller_core'
        ],
        features: [
          'ロールバック機能',
          'ブルーグリーンデプロイ',
          'カナリアデプロイ',
          'デプロイメント履歴管理',
          'ヘルスチェック統合',
          '自動ロールバック'
        ],
        status: 'completed'
      },

      network_extension: {
        name: 'ネットワーク拡張',
        description: '高度なネットワークマネージャー - CDN統合、セキュリティ、エッジ最適化',
        components: [
          'deploy_network_core'
        ],
        features: [
          'CDN統合 (Cloudflare, AWS CloudFront)',
          'レートリミッティング',
          'WAF (Web Application Firewall)',
          'DDoS対策',
          'SSL/TLS証明書自動管理',
          '地理情報ベース最適化',
          'エッジ最適化',
          'キャッシュ管理'
        ],
        status: 'completed'
      },

      scaling_extension: {
        name: 'スケーリング拡張',
        description: 'AI予測スケーリングエンジン - トラフィック予測、コスト最適化',
        components: [
          'deploy_scaling_core'
        ],
        features: [
          'AIトラフィック予測',
          '自動スケーリング',
          'コスト最適化',
          'パフォーマンス監視',
          'インテリジェントスケーリング',
          '負荷分散最適化'
        ],
        status: 'pending'
      }
    },

    documentation_generator: {
      name: 'Kotoba Documentation Generator (kdoc)',
      description: '高度なドキュメント生成システム - Denoを参考にした使い勝手で、美しいHTMLドキュメントを自動生成',
      version: '0.1.0',
      last_updated: '2025-09-17',
      status: 'planned',

      components: [
        {
          name: 'docs_parser',
          description: '多言語ソースコードパーサー (Rust, JS, TS, Python, Go)',
          features: ['comment_extraction', 'docstring_parsing', 'cross_references', 'language_detection']
        },
        {
          name: 'docs_config',
          description: '設定管理とTOML/JSON/YAMLパーサー',
          features: ['auto_detection', 'project_inference', 'validation', 'flexible_config']
        },
        {
          name: 'docs_generator',
          description: 'ドキュメント生成エンジン',
          features: ['html_output', 'markdown_output', 'json_output', 'template_system']
        },
        {
          name: 'docs_template',
          description: 'Teraベースのテンプレートシステム',
          features: ['custom_filters', 'responsive_design', 'theme_support', 'extensible']
        },
        {
          name: 'docs_search',
          description: '全文検索エンジン',
          features: ['fuzzy_search', 'indexing', 'ranking', 'real_time_search']
        },
        {
          name: 'docs_server',
          description: 'WebサーバーとREST API',
          features: ['hot_reload', 'api_endpoints', 'static_files', 'cors_support']
        },
        {
          name: 'docs_core',
          description: 'コアAPIとエラーハンドリング',
          features: ['unified_api', 'error_types', 'type_system', 'extensibility']
        },
        {
          name: 'docs_cli',
          description: 'CLIコマンド統合',
          features: ['generate_command', 'serve_command', 'search_command', 'init_command']
        }
      ],

      features: [
        'Multi-language support (5+ languages)',
        'HTML/Markdown/JSON output formats',
        'Full-text search with fuzzy matching',
        'Responsive web interface',
        'Template customization',
        'REST API for integrations',
        'Auto-configuration detection',
        'Cross-reference generation',
        'Search indexing and ranking',
        'Hot reload development server'
      ],

      cli_commands: [
        'kotoba docs generate  # ドキュメント生成',
        'kotoba docs serve     # 開発サーバー起動',
        'kotoba docs search    # ドキュメント検索',
        'kotoba docs init      # 設定初期化'
      ],

      output_formats: [
        'HTML (responsive, searchable)',
        'Markdown (GitHub compatible)',
        'JSON (programmatic access)',
        'PDF (future extension)'
      ],

      integration_points: [
        'kotoba-cli (CLI integration)',
        'http_ir (web server integration)',
        'types (core types)',
        'cli_interface (command system)'
      ]
    },
    jsonnet_compatibility: {
      version: '0.21.0',
      implementation: 'pure_rust',
      source: 'https://github.com/google/jsonnet',
      features: [
        'complete_ast',
        'full_lexer',
        'recursive_parser',
        'evaluator_with_stdlib',
        '80_plus_std_functions',
        'import_importstr',
        'error_handling',
        'json_yaml_output',
      ],
      status: 'fully_compatible',
    },
    kotobanet_extensions: {
      crate: 'kotoba-kotobas',
      description: 'Kotoba-specific Jsonnet extensions',
      components: [
        {
          name: 'http_parser',
          description: '.kotoba.json configuration file parsing',
          features: ['route_config', 'middleware_config', 'auth_config', 'cors_config'],
        },
        {
          name: 'frontend',
          description: 'React component definitions in Jsonnet',
          features: ['component_defs', 'page_routes', 'api_routes', 'state_management'],
        },
        {
          name: 'deploy',
          description: 'Deployment configuration management',
          features: ['scaling_config', 'region_config', 'networking', 'monitoring', 'security'],
        },
        {
          name: 'config',
          description: 'General application configuration',
          features: ['database_config', 'cache_config', 'messaging_config', 'external_services'],
        },
      ],
      integration_points: [
        'http_parser',
        'frontend_framework',
        'deploy_parser',
        'deploy_config',
      ],
      status: 'fully_implemented',
    },
  },
}

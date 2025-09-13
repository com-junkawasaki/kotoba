{
  // Kotoba GP2系グラフ書換え言語 - プロセスネットワークDAG
  // トポロジカルソート: ビルド順序
  // 逆トポロジカルソート: 問題解決順序

  // ==========================================
  // ノード定義 (Components/Processes)
  // ==========================================

  nodes: {
    // 基底層
    'types': {
      name: 'types',
      path: 'src/types.rs',
      type: 'foundation',
      description: '共通型定義 (Value, VertexId, EdgeId, GraphRef, etc.)',
      dependencies: [],
      provides: ['Value', 'VertexId', 'EdgeId', 'GraphRef', 'TxId', 'ContentHash'],
      status: 'completed',
      build_order: 1,
    },

    // IR層
    'ir_catalog': {
      name: 'ir_catalog',
      path: 'src/ir/catalog.rs',
      type: 'ir',
      description: 'スキーマ/索引/不変量定義',
      dependencies: ['types'],
      provides: ['Catalog', 'LabelDef', 'IndexDef', 'Invariant'],
      status: 'completed',
      build_order: 2,
    },

    'ir_rule': {
      name: 'ir_rule',
      path: 'src/ir/rule.rs',
      type: 'ir',
      description: 'DPO型付き属性グラフ書換えルール',
      dependencies: ['types'],
      provides: ['RuleIR', 'Match', 'Guard'],
      status: 'completed',
      build_order: 2,
    },

    'ir_query': {
      name: 'ir_query',
      path: 'src/ir/query.rs',
      type: 'ir',
      description: 'GQL論理プラン代数',
      dependencies: ['types'],
      provides: ['PlanIR', 'LogicalOp', 'Expr', 'Predicate'],
      status: 'completed',
      build_order: 2,
    },

    'ir_patch': {
      name: 'ir_patch',
      path: 'src/ir/patch.rs',
      type: 'ir',
      description: '差分表現 (addV/E, delV/E, setProp, relink)',
      dependencies: ['types'],
      provides: ['Patch', 'AddVertex', 'AddEdge', 'UpdateProp'],
      status: 'completed',
      build_order: 2,
    },

    'ir_strategy': {
      name: 'ir_strategy',
      path: 'src/ir/strategy.rs',
      type: 'ir',
      description: '戦略表現 (once|exhaust|while|seq|choice|priority)',
      dependencies: ['types', 'ir_patch'],
      provides: ['StrategyIR', 'StrategyOp', 'StrategyResult', 'Externs'],
      status: 'completed',
      build_order: 3,
    },

    // グラフ層
    'graph_vertex': {
      name: 'graph_vertex',
      path: 'src/graph/vertex.rs',
      type: 'graph',
      description: '頂点関連構造体とビルダー',
      dependencies: ['types'],
      provides: ['VertexBuilder', 'VertexData'],
      status: 'completed',
      build_order: 2,
    },

    'graph_edge': {
      name: 'graph_edge',
      path: 'src/graph/edge.rs',
      type: 'graph',
      description: 'エッジ関連構造体とビルダー',
      dependencies: ['types'],
      provides: ['EdgeBuilder', 'EdgeData'],
      status: 'completed',
      build_order: 2,
    },

    'graph_core': {
      name: 'graph_core',
      path: 'src/graph/graph.rs',
      type: 'graph',
      description: '列指向グラフ表現とGraphRef',
      dependencies: ['types', 'graph_vertex', 'graph_edge'],
      provides: ['Graph', 'GraphRef'],
      status: 'completed',
      build_order: 3,
    },

    // ストレージ層
    'storage_mvcc': {
      name: 'storage_mvcc',
      path: 'src/storage/mvcc.rs',
      type: 'storage',
      description: 'MVCCマネージャー',
      dependencies: ['types', 'graph_core'],
      provides: ['MVCCManager', 'Transaction', 'TxState'],
      status: 'completed',
      build_order: 4,
    },

    'storage_merkle': {
      name: 'storage_merkle',
      path: 'src/storage/merkle.rs',
      type: 'storage',
      description: 'Merkle DAG永続化',
      dependencies: ['types', 'graph_core'],
      provides: ['MerkleDAG', 'MerkleNode', 'GraphVersion'],
      status: 'completed',
      build_order: 4,
    },

    'storage_lsm': {
      name: 'storage_lsm',
      path: 'src/storage/lsm.rs',
      type: 'storage',
      description: 'LSMツリーストレージ',
      dependencies: ['types'],
      provides: ['LSMTree', 'SSTable', 'LSMEntry'],
      status: 'completed',
      build_order: 4,
    },

    // プランナー層
    'planner_logical': {
      name: 'planner_logical',
      path: 'src/planner/logical.rs',
      type: 'planner',
      description: '論理プランナー (GQL → 論理プラン)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core'],
      provides: ['LogicalPlanner', 'CostEstimator'],
      status: 'completed',
      build_order: 5,
    },

    'planner_physical': {
      name: 'planner_physical',
      path: 'src/planner/physical.rs',
      type: 'planner',
      description: '物理プランナー (論理プラン → 物理プラン)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core'],
      provides: ['PhysicalPlanner', 'PhysicalPlan', 'PhysicalOp'],
      status: 'completed',
      build_order: 5,
    },

    'planner_optimizer': {
      name: 'planner_optimizer',
      path: 'src/planner/optimizer.rs',
      type: 'planner',
      description: 'クエリ最適化器 (述語押下げ, 結合順序DP, インデックス選択)',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core', 'planner_logical', 'planner_physical'],
      provides: ['QueryOptimizer', 'OptimizationRule'],
      status: 'completed',
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
      status: 'completed',
      build_order: 5,
    },

    'execution_engine': {
      name: 'execution_engine',
      path: 'src/execution/executor.rs',
      type: 'execution',
      description: 'クエリ実行器',
      dependencies: ['types', 'ir_query', 'ir_catalog', 'graph_core', 'storage_mvcc', 'storage_merkle', 'planner_logical', 'planner_physical', 'planner_optimizer', 'execution_parser'],
      provides: ['QueryExecutor'],
      status: 'completed',
      build_order: 7,
    },

    // 書換え層
    'rewrite_matcher': {
      name: 'rewrite_matcher',
      path: 'src/rewrite/matcher.rs',
      type: 'rewrite',
      description: 'ルールマッチング (LHS + NACチェック)',
      dependencies: ['types', 'ir_rule', 'ir_catalog', 'graph_core'],
      provides: ['RuleMatcher'],
      status: 'completed',
      build_order: 5,
    },

    'rewrite_applier': {
      name: 'rewrite_applier',
      path: 'src/rewrite/applier.rs',
      type: 'rewrite',
      description: 'ルール適用 (パッチ生成)',
      dependencies: ['types', 'ir_rule', 'ir_patch', 'graph_core'],
      provides: ['RuleApplier'],
      status: 'completed',
      build_order: 5,
    },

    'rewrite_engine': {
      name: 'rewrite_engine',
      path: 'src/rewrite/engine.rs',
      type: 'rewrite',
      description: 'DPO書換えエンジン (マッチング + 適用 + 戦略実行)',
      dependencies: ['types', 'ir_rule', 'ir_strategy', 'graph_core', 'storage_mvcc', 'storage_merkle', 'rewrite_matcher', 'rewrite_applier'],
      provides: ['RewriteEngine', 'RewriteExterns'],
      status: 'completed',
      build_order: 6,
    },

    // メインライブラリ
    'lib': {
      name: 'lib',
      path: 'src/lib.rs',
      type: 'library',
      description: 'メインライブラリインターフェース',
      dependencies: ['types', 'ir_catalog', 'ir_rule', 'ir_query', 'ir_patch', 'ir_strategy', 'graph_core', 'storage_mvcc', 'storage_merkle', 'storage_lsm', 'planner_logical', 'planner_physical', 'planner_optimizer', 'execution_parser', 'execution_engine', 'rewrite_matcher', 'rewrite_applier', 'rewrite_engine'],
      provides: ['kotoba'],
      status: 'completed',
      build_order: 8,
    },
  },

  // ==========================================
  // エッジ定義 (Dependencies)
  // ==========================================

  edges: [
    // types -> すべて
    { from: 'types', to: 'ir_catalog' },
    { from: 'types', to: 'ir_rule' },
    { from: 'types', to: 'ir_query' },
    { from: 'types', to: 'ir_patch' },
    { from: 'types', to: 'graph_vertex' },
    { from: 'types', to: 'graph_edge' },
    { from: 'types', to: 'storage_mvcc' },
    { from: 'types', to: 'storage_merkle' },
    { from: 'types', to: 'storage_lsm' },
    { from: 'types', to: 'planner_logical' },
    { from: 'types', to: 'planner_physical' },
    { from: 'types', to: 'execution_parser' },
    { from: 'types', to: 'execution_engine' },
    { from: 'types', to: 'rewrite_matcher' },
    { from: 'types', to: 'rewrite_applier' },
    { from: 'types', to: 'rewrite_engine' },
    { from: 'types', to: 'lib' },
    { from: 'ir_catalog', to: 'lib' },
    { from: 'ir_rule', to: 'lib' },
    { from: 'ir_query', to: 'lib' },
    { from: 'ir_patch', to: 'lib' },
    { from: 'ir_strategy', to: 'lib' },

    // IR相互依存
    { from: 'types', to: 'ir_strategy' },
    { from: 'ir_patch', to: 'ir_strategy' },
    { from: 'ir_strategy', to: 'rewrite_engine' },

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
  ],

  // ==========================================
  // トポロジカルソート (ビルド順序)
  // ==========================================

  topological_order: [
    'types',
    'ir_catalog',
    'ir_rule',
    'ir_query',
    'ir_patch',
    'graph_vertex',
    'graph_edge',
    'ir_strategy',
    'graph_core',
    'storage_mvcc',
    'storage_merkle',
    'storage_lsm',
    'planner_logical',
    'planner_physical',
    'execution_parser',
    'rewrite_matcher',
    'rewrite_applier',
    'planner_optimizer',
    'rewrite_engine',
    'execution_engine',
    'lib',
  ],

  // ==========================================
  // 逆トポロジカルソート (問題解決順序)
  // ==========================================

  reverse_topological_order: [
    'lib',
    'execution_engine',
    'rewrite_engine',
    'planner_optimizer',
    'rewrite_applier',
    'rewrite_matcher',
    'execution_parser',
    'planner_physical',
    'planner_logical',
    'storage_lsm',
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
    'types',
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
    description: 'GP2系グラフ書換え言語 - ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム',
    version: '0.1.0',
    architecture: 'Process Network Graph Model',
    created_at: '2025-01-12',
    last_updated: std.extVar('last_updated'),
    author: 'jun784',
  },
}

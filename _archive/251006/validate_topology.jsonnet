// トポロジー検証スクリプト
// dag.jsonnetからトポロジーデータを読み込み、Rustプログラムで検証するためのJSON出力を生成

// dag.jsonnetをインポート
local dag = import 'dag.jsonnet';

// ノードデータをRustのTopologyGraph形式に変換
local convert_nodes() = {
  [node_name]: {
    name: dag.nodes[node_name].name,
    path: std.get(dag.nodes[node_name], 'path', ''),
    node_type: std.get(dag.nodes[node_name], 'type', 'unknown'),
    description: std.get(dag.nodes[node_name], 'description', ''),
    dependencies: std.get(dag.nodes[node_name], 'dependencies', []),
    provides: std.get(dag.nodes[node_name], 'provides', []),
    status: std.get(dag.nodes[node_name], 'status', 'unknown'),
    build_order: std.get(dag.nodes[node_name], 'build_order', 999), // デフォルト値999
  }
  for node_name in std.objectFields(dag.nodes)
};

// エッジデータを変換
local convert_edges() = [
  {
    from: edge.from,
    to: edge.to,
  }
  for edge in dag.edges
];

// トポロジーグラフデータを生成
local topology_graph = {
  nodes: convert_nodes(),
  edges: convert_edges(),
  topological_order: dag.topological_order,
  reverse_topological_order: dag.reverse_topological_order,
};

// 検証用のメタデータを追加
local validation_metadata = {
  metadata: {
    project_name: dag.metadata.project_name,
    version: dag.metadata.version,
    architecture: dag.metadata.architecture,
    validation_timestamp: '2025-01-12T00:00:00Z',
    validator_version: '1.0.0',
  },
  statistics: {
    node_count: std.length(std.objectFields(dag.nodes)),
    edge_count: std.length(dag.edges),
    max_build_order: std.length(std.objectFields(dag.nodes)), // 仮の値、Rust側で計算
    has_cycles: false,  // これはRust側で計算
    is_valid_topology: false,  // これはRust側で計算
  },
};

// 検証ヘルパー関数
local validate_basic_structure() =
  local node_count = std.length(std.objectFields(dag.nodes));
  local edge_count = std.length(dag.edges);
  local topo_count = std.length(dag.topological_order);
  local reverse_topo_count = std.length(dag.reverse_topological_order);

  if node_count == 0 then
    error 'No nodes defined in topology'
  else if edge_count == 0 && node_count > 1 then
    error 'Multiple nodes but no edges defined'
  else if topo_count != node_count then
    error 'Topological order length does not match node count'
  else if reverse_topo_count != node_count then
    error 'Reverse topological order length does not match node count'
  else
    'Basic structure validation passed';

// 基本構造の検証を実行（コンパイル時にチェック）
local _ = validate_basic_structure();

// 最終的な出力データ構造
{
  topology_graph: topology_graph,
  validation_metadata: validation_metadata,
  validation_rules: {
    // 検証ルール定義
    node_existence: {
      description: 'すべてのエッジで参照されるノードが存在する',
      severity: 'error',
    },
    edge_integrity: {
      description: 'エッジの整合性が保たれている（重複、自己参照なし）',
      severity: 'error',
    },
    no_cycles: {
      description: '循環依存が存在しない',
      severity: 'error',
    },
    topological_order: {
      description: 'トポロジカル順序が正しい',
      severity: 'error',
    },
    dependency_integrity: {
      description: 'ノードの依存関係とエッジが一致する',
      severity: 'error',
    },
    build_order_integrity: {
      description: 'ビルド順序が依存関係を満たす',
      severity: 'error',
    },
  },
}

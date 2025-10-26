#!/bin/bash

# トポロジー検証スクリプト
# dag.jsonnetからトポロジーデータを生成し、Rustプログラムで検証を実行する

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Kotoba Topology Validation ==="
echo

# 前提条件のチェック
echo "Checking prerequisites..."

if ! command -v jsonnet &> /dev/null; then
    echo "❌ jsonnet is not installed. Please install jsonnet first."
    echo "   See: https://jsonnet.org/"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ cargo is not installed. Please install Rust first."
    echo "   See: https://rustup.rs/"
    exit 1
fi

if [ ! -f "$PROJECT_DIR/dag.jsonnet" ]; then
    echo "❌ dag.jsonnet not found in project root"
    exit 1
fi

echo "✅ Prerequisites check passed"
echo

# 作業ディレクトリに移動
cd "$PROJECT_DIR"

# jsonnetスクリプトを実行してトポロジーデータを生成
echo "Generating topology data from dag.jsonnet..."
if ! jsonnet validate_topology.jsonnet > topology_data.json; then
    echo "❌ Failed to generate topology data from jsonnet"
    exit 1
fi

echo "✅ Topology data generated successfully"
echo

# 生成されたJSONデータを確認
echo "Generated topology data summary:"
if command -v jq &> /dev/null; then
    echo "  Nodes: $(jq '.topology_graph.nodes | length' topology_data.json)"
    echo "  Edges: $(jq '.topology_graph.edges | length' topology_data.json)"
    echo "  Topological order length: $(jq '.topology_graph.topological_order | length' topology_data.json)"
    echo
else
    echo "  (Install jq for detailed summary)"
    echo
fi

# Rustのトポロジー検証テストを実行
echo "Running topology validation tests..."
if cargo test test_topology_validation_from_jsonnet --lib -- --nocapture; then
    echo "✅ Topology validation tests passed"
else
    echo "❌ Topology validation tests failed"
    exit 1
fi

echo
echo "Running additional topology tests..."
if cargo test topology_validation --lib -- --nocapture; then
    echo "✅ All topology tests passed"
else
    echo "❌ Some topology tests failed"
    exit 1
fi

echo
echo "=== Topology Validation Complete ==="
echo "🎉 All checks passed! The process network topology is valid."

# 一時ファイルをクリーンアップ
rm -f topology_data.json

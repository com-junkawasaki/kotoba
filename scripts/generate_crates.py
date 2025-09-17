#!/usr/bin/env python3
"""
Kotoba Multi-Crate Generator
dag.jsonnetから依存関係を読み取り、各crateのCargo.tomlを自動生成
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Set

class CrateGenerator:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.crates_dir = project_root / "crates"
        self.dag_data = self._load_dag_data()

        # Crate構成定義
        self.crate_mapping = {
            'kotoba-core': [
                'types', 'ir_catalog', 'ir_rule', 'ir_query', 'ir_patch', 'ir_strategy'
            ],
            'kotoba-graph': [
                'graph_vertex', 'graph_edge', 'graph_core'
            ],
            'kotoba-storage': [
                'storage_mvcc', 'storage_merkle', 'storage_lsm'
            ],
            'kotoba-execution': [
                'execution_parser', 'execution_engine', 'planner_logical',
                'planner_physical', 'planner_optimizer'
            ],
            'kotoba-rewrite': [
                'rewrite_matcher', 'rewrite_applier', 'rewrite_engine'
            ],
            'kotoba-server': [
                'http_ir', 'http_parser', 'http_handlers', 'http_engine', 'http_server',
                'frontend_component_ir', 'frontend_route_ir', 'frontend_render_ir',
                'frontend_build_ir', 'frontend_api_ir', 'frontend_framework'
            ]
        }

        # 共通依存関係
        self.common_deps = {
            "serde": { "version": "1.0", "features": ["derive"] },
            "serde_json": "1.0",
            "thiserror": "2.0",
            "anyhow": "1.0",
        }

    def _load_dag_data(self) -> Dict:
        """手動で依存関係データを定義（dag.jsonnetの代わり）"""
        return {
            "nodes": {
                "types": {"status": "completed"},
                "ir_catalog": {"status": "completed"},
                "ir_rule": {"status": "completed"},
                "ir_query": {"status": "completed"},
                "ir_patch": {"status": "completed"},
                "ir_strategy": {"status": "completed"},
                "graph_vertex": {"status": "completed"},
                "graph_edge": {"status": "completed"},
                "graph_core": {"status": "completed"},
                "storage_mvcc": {"status": "completed"},
                "storage_merkle": {"status": "completed"},
                "storage_lsm": {"status": "completed"},
                "execution_parser": {"status": "completed"},
                "execution_engine": {"status": "completed"},
                "planner_logical": {"status": "completed"},
                "planner_physical": {"status": "completed"},
                "planner_optimizer": {"status": "completed"},
                "rewrite_matcher": {"status": "completed"},
                "rewrite_applier": {"status": "completed"},
                "rewrite_engine": {"status": "completed"},
                "http_ir": {"status": "completed"},
                "http_parser": {"status": "pending"},
                "http_handlers": {"status": "pending"},
                "http_engine": {"status": "pending"},
                "http_server": {"status": "pending"},
                "frontend_component_ir": {"status": "completed"},
                "frontend_route_ir": {"status": "completed"},
                "frontend_render_ir": {"status": "completed"},
                "frontend_build_ir": {"status": "completed"},
                "frontend_api_ir": {"status": "completed"},
                "frontend_framework": {"status": "in_progress"}
            },
            "edges": [
                {"from": "types", "to": "ir_catalog"},
                {"from": "types", "to": "graph_vertex"},
                {"from": "types", "to": "graph_edge"},
                {"from": "graph_vertex", "to": "graph_core"},
                {"from": "graph_edge", "to": "graph_core"},
                {"from": "types", "to": "graph_core"},
            ]
        }

    def _get_crate_dependencies(self, crate_name: str) -> List[str]:
        """指定されたcrateの依存関係を取得"""
        if crate_name not in self.crate_mapping:
            return []

        components = self.crate_mapping[crate_name]
        all_deps = set()

        for comp in components:
            if comp in self.dag_data['nodes']:
                deps = [edge['from'] for edge in self.dag_data['edges'] if edge['to'] == comp]
                all_deps.update(deps)

        # 同じcrate内の依存関係は除外
        external_deps = []
        for dep in all_deps:
            dep_crate = self._get_crate_from_component(dep)
            if dep_crate != crate_name:
                external_deps.append(dep_crate)

        return list(set(external_deps))

    def _get_crate_from_component(self, component: str) -> str:
        """コンポーネント名からcrate名を取得"""
        for crate_name, components in self.crate_mapping.items():
            if component in components:
                return crate_name
        return 'kotoba-core'  # デフォルト

    def _get_crate_version(self, crate_name: str) -> str:
        """crateのバージョンを取得"""
        # ステータスに基づいてバージョン決定
        completed_count = 0
        total_count = 0

        for comp in self.crate_mapping.get(crate_name, []):
            if comp in self.dag_data['nodes']:
                total_count += 1
                if self.dag_data['nodes'][comp].get('status') == 'completed':
                    completed_count += 1

        if total_count == 0:
            return "0.1.0"

        completion_rate = completed_count / total_count
        if completion_rate >= 0.8:
            return "0.1.0"
        elif completion_rate >= 0.5:
            return "0.1.0-alpha"
        else:
            return "0.1.0-dev"

    def generate_cargo_toml(self, crate_name: str) -> str:
        """指定されたcrateのCargo.tomlを生成"""
        version = self._get_crate_version(crate_name)
        dependencies = self._get_crate_dependencies(crate_name)

        # Cargo.tomlテンプレート
        toml_content = f"""[package]
name = "{crate_name}"
version = "{version}"
edition = "2021"
description = "Kotoba {crate_name.split('-')[1].title()} Components"
license = "Apache-2.0"
repository = "https://github.com/com-junkawasaki/kotoba"

[dependencies]
"""

        # 共通依存関係を追加
        for dep, config in self.common_deps.items():
            if isinstance(config, str):
                toml_content += f'{dep} = "{config}"\n'
            elif isinstance(config, dict):
                features = config.get('features', [])
                if features:
                    toml_content += f'{dep} = {{ version = "{config["version"]}", features = {features} }}\n'
                else:
                    toml_content += f'{dep} = "{config["version"]}"\n'

        # 内部依存関係を追加
        for dep in dependencies:
            if dep and dep != crate_name:
                dep_version = self._get_crate_version(dep)
                toml_content += f'{dep} = {{ path = "../{dep}", version = "{dep_version}" }}\n'

        # WASM対応
        if crate_name in ['kotoba-core', 'kotoba-graph', 'kotoba-server']:
            toml_content += """
[features]
default = ["std"]
std = []
wasm = ["wasm-bindgen", "web-sys", "js-sys"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = { version = "0.2", optional = true }
web-sys = { version = "0.3", optional = true }
js-sys = { version = "0.3", optional = true }
"""

        # dev-dependencies
        toml_content += """
[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
"""

        return toml_content

    def create_crate_structure(self, crate_name: str):
        """指定されたcrateのディレクトリ構造を作成"""
        crate_dir = self.crates_dir / crate_name
        crate_dir.mkdir(parents=True, exist_ok=True)

        # Cargo.toml生成
        cargo_toml = self.generate_cargo_toml(crate_name)
        (crate_dir / "Cargo.toml").write_text(cargo_toml)

        # srcディレクトリ作成
        src_dir = crate_dir / "src"
        src_dir.mkdir(exist_ok=True)

        # lib.rs作成
        lib_content = f"""//! {crate_name} - Kotoba {crate_name.split('-')[1].title()} Components

pub mod prelude {{
    // Re-export commonly used items
}}

#[cfg(test)]
mod tests {{
    // Tests will be added here
}}
"""
        (src_dir / "lib.rs").write_text(lib_content)

        print(f"Created crate: {crate_name}")

    def generate_workspace_cargo_toml(self) -> str:
        """ワークスペース全体のCargo.tomlを生成"""
        workspace_members = list(self.crate_mapping.keys())

        members_str = "\n".join(f'    "{member}",' for member in workspace_members)

        return f"""[workspace]
members = [
{members_str}
    "kotoba",  # Root crate
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/com-junkawasaki/kotoba"

[workspace.dependencies]
# Common dependencies can be defined here
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
thiserror = "2.0"
anyhow = "1.0"
"""

    def run(self):
        """メイン実行関数"""
        print("Generating Kotoba multi-crate structure...")

        # 各crateを作成
        for crate_name in self.crate_mapping.keys():
            self.create_crate_structure(crate_name)

        # ワークスペースCargo.tomlを更新
        workspace_toml = self.generate_workspace_cargo_toml()
        (self.project_root / "Cargo.toml").write_text(workspace_toml)

        print("Multi-crate generation completed!")

        # 統計情報表示
        total_components = sum(len(comps) for comps in self.crate_mapping.values())
        print(f"Generated {len(self.crate_mapping)} crates with {total_components} components")

def main():
    project_root = Path(__file__).parent.parent
    generator = CrateGenerator(project_root)
    generator.run()

if __name__ == "__main__":
    main()

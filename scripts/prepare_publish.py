#!/usr/bin/env python3
"""
Kotoba Crates Publish Preparation
各crateのCargo.tomlにpublishに必要なフィールドを追加
"""

import os
import glob
from pathlib import Path

def update_cargo_toml(crate_path: Path, crate_name: str):
    """指定されたcrateのCargo.tomlをpublish用に更新"""

    cargo_toml_path = crate_path / "Cargo.toml"

    if not cargo_toml_path.exists():
        print(f"Warning: {cargo_toml_path} not found")
        return

    with open(cargo_toml_path, 'r') as f:
        content = f.read()

    # 既存のフィールドを確認
    has_authors = "authors" in content
    has_keywords = "keywords" in content
    has_categories = "categories" in content
    has_readme = "readme" in content
    has_documentation = "documentation" in content
    has_homepage = "homepage" in content

    # 更新が必要な部分
    updates = []

    if not has_authors:
        updates.append('authors = ["Jun Kawasaki <jun784@example.com>"]')

    if not has_keywords:
        if "core" in crate_name:
            keywords = 'keywords = ["graph", "database", "gql", "rewrite", "types"]'
        elif "graph" in crate_name:
            keywords = 'keywords = ["graph", "data-structure", "vertex", "edge"]'
        elif "storage" in crate_name:
            keywords = 'keywords = ["storage", "persistence", "mvcc", "merkle"]'
        elif "execution" in crate_name:
            keywords = 'keywords = ["query", "execution", "planner", "gql"]'
        elif "rewrite" in crate_name:
            keywords = 'keywords = ["rewrite", "graph-transformation", "dpo"]'
        elif "web" in crate_name:
            keywords = 'keywords = ["web", "http", "frontend", "framework"]'
        else:
            keywords = 'keywords = ["graph", "database", "gql"]'
        updates.append(keywords)

    if not has_categories:
        if "core" in crate_name or "graph" in crate_name:
            categories = 'categories = ["data-structures", "algorithms"]'
        elif "storage" in crate_name:
            categories = 'categories = ["database-implementations", "data-structures"]'
        elif "execution" in crate_name or "rewrite" in crate_name:
            categories = 'categories = ["algorithms", "database-implementations"]'
        elif "web" in crate_name:
            categories = 'categories = ["web-programming", "api-bindings"]'
        else:
            categories = 'categories = ["database", "data-structures"]'
        updates.append(categories)

    if not has_readme:
        updates.append('readme = "README.md"')

    if not has_documentation:
        updates.append('documentation = "https://docs.rs/' + crate_name + '"')

    if not has_homepage:
        updates.append('homepage = "https://github.com/com-junkawasaki/kotoba"')

    # 更新内容を適用
    if updates:
        # [package]セクションの後に追加
        lines = content.split('\n')
        package_section_end = -1

        for i, line in enumerate(lines):
            if line.strip() == "[package]":
                # [package]セクションの終わりを見つける
                for j in range(i + 1, len(lines)):
                    if lines[j].strip().startswith('[') and lines[j].strip() != "[package]":
                        package_section_end = j
                        break
                    elif lines[j].strip() == "" and j == len(lines) - 1:
                        package_section_end = j + 1
                        break
                break

        if package_section_end > 0:
            # 更新内容を挿入
            updated_lines = lines[:package_section_end]
            updated_lines.append("")  # 空行を追加
            updated_lines.extend(updates)
            updated_lines.append("")  # 空行を追加
            updated_lines.extend(lines[package_section_end:])

            new_content = '\n'.join(updated_lines)

            with open(cargo_toml_path, 'w') as f:
                f.write(new_content)

            print(f"Updated {crate_name}:")
            for update in updates:
                print(f"  + {update}")
        else:
            print(f"Warning: Could not find package section in {crate_name}")
    else:
        print(f"{crate_name}: Already up to date")

def create_readme_for_crate(crate_path: Path, crate_name: str):
    """各crateにREADME.mdを作成"""
    readme_path = crate_path / "README.md"

    if readme_path.exists():
        print(f"README already exists for {crate_name}")
        return

    if "core" in crate_name:
        description = "# Kotoba Core\n\nCore components for Kotoba graph processing system.\n\nProvides fundamental types and IR definitions."
    elif "graph" in crate_name:
        description = "# Kotoba Graph\n\nGraph data structures and operations for Kotoba.\n\nIncludes vertex, edge, and graph implementations."
    elif "storage" in crate_name:
        description = "# Kotoba Storage\n\nStorage layer for Kotoba with MVCC and Merkle DAG.\n\nProvides persistent storage with versioning."
    elif "execution" in crate_name:
        description = "# Kotoba Execution\n\nQuery execution and planning for Kotoba.\n\nIncludes GQL parser, logical planner, and physical execution."
    elif "rewrite" in crate_name:
        description = "# Kotoba Rewrite\n\nGraph rewriting engine for Kotoba.\n\nImplements DPO (Double Pushout) graph transformations."
    elif "web" in crate_name:
        description = "# Kotoba Web\n\nWeb framework and HTTP components for Kotoba.\n\nProvides HTTP server and frontend integration."
    else:
        description = f"# {crate_name}\n\n{crate_name} components for Kotoba."

    with open(readme_path, 'w') as f:
        f.write(description + "\n\n## License\n\nMIT OR Apache-2.0")

    print(f"Created README.md for {crate_name}")

def main():
    project_root = Path(__file__).parent.parent
    crates_dir = project_root / "crates"

    print("Preparing Kotoba crates for publishing...")

    # 各crateを処理
    for crate_path in crates_dir.iterdir():
        if crate_path.is_dir():
            crate_name = crate_path.name
            print(f"\nProcessing {crate_name}...")

            update_cargo_toml(crate_path, crate_name)
            create_readme_for_crate(crate_path, crate_name)

    print("\nPublish preparation completed!")

if __name__ == "__main__":
    main()

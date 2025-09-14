# Kotoba Crates Publish Guide

## Prerequisites
1. Create account at https://crates.io/
2. Get API token from https://crates.io/me
3. Login with: cargo login YOUR_API_TOKEN

## Publish Order (due to dependencies)
1. kotoba-core (no dependencies)
2. kotoba-graph (depends on kotoba-core)
3. kotoba-storage (depends on kotoba-core)
4. kotoba-execution (depends on kotoba-core)
5. kotoba-rewrite (depends on kotoba-core, kotoba-graph, kotoba-storage)
6. kotoba-web (depends on kotoba-core)

## Publish Commands
cargo publish -p kotoba-core
cargo publish -p kotoba-graph
cargo publish -p kotoba-storage
cargo publish -p kotoba-execution
cargo publish -p kotoba-rewrite
cargo publish -p kotoba-web

## Verification
After publishing, verify at:
- https://crates.io/crates/kotoba-core
- https://crates.io/crates/kotoba-graph
- etc.


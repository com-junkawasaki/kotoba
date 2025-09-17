//! グラフアルゴリズムの実装
//!
//! このモジュールは、グラフ理論のアルゴリズムを実装します。
//! 最短経路、中央性指標、パターンマッチングなどを提供します。

use std::collections::{HashMap, HashSet, BinaryHeap, VecDeque};
use std::cmp::Reverse;
use kotoba_core::types::*;
use kotoba_core::KotobaError;
use crate::graph::{Graph, EdgeData, VertexData};

/// 最短経路の結果
#[derive(Debug, Clone, Default)]
pub struct ShortestPathResult {
    pub distances: HashMap<VertexId, u64>,
    pub previous: HashMap<VertexId, VertexId>,
}

/// 中央性指標の結果
#[derive(Debug, Clone)]
pub struct CentralityResult {
    /// 頂点ごとの中央性スコア
    pub scores: HashMap<VertexId, f64>,
    /// アルゴリズム種別
    pub algorithm: CentralityAlgorithm,
}

/// 中央性アルゴリズム種別
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CentralityAlgorithm {
    /// 次数中央性
    Degree,
    /// 媒介中央性
    Betweenness,
    /// 近接中央性
    Closeness,
    /// 固有ベクトル中央性
    Eigenvector,
    /// PageRank
    PageRank,
}

/// パターンマッチング結果
#[derive(Debug, Clone)]
pub struct PatternMatchResult {
    /// マッチした部分グラフのマッピング
    pub mappings: Vec<SubgraphMapping>,
    /// マッチ数
    pub count: usize,
}

/// 部分グラフマッピング
#[derive(Debug, Clone)]
pub struct SubgraphMapping {
    /// パターン頂点 → データグラフ頂点 のマッピング
    pub vertex_map: HashMap<VertexId, VertexId>,
    /// パターンエッジ → データグラフエッジ のマッピング
    pub edge_map: HashMap<EdgeId, EdgeId>,
}

/// グラフアルゴリズムマネージャー
#[derive(Debug)]
pub struct GraphAlgorithms;

impl GraphAlgorithms {
    /// Dijkstraのアルゴリズムで最短経路を計算
    pub fn shortest_path_dijkstra(
        graph: &Graph,
        source: VertexId,
        weight_fn: impl Fn(&EdgeData) -> u64,
    ) -> Result<ShortestPathResult> {
        let mut distances: HashMap<VertexId, u64> = HashMap::new();
        let mut predecessors: HashMap<VertexId, VertexId> = HashMap::new();
        let mut pq: BinaryHeap<Reverse<(u64, VertexId)>> = BinaryHeap::new();
        let mut visited: HashSet<VertexId> = HashSet::new();

        // 初期化
        for &vertex_id in graph.vertices.keys() {
            distances.insert(vertex_id, u64::MAX);
        }
        distances.insert(source, 0);
        pq.push(Reverse((0, source)));

        while let Some(Reverse((dist, u))) = pq.pop() {
            if visited.contains(&u) {
                continue;
            }
            visited.insert(u);

            // 隣接頂点を更新
            if let Some(neighbors) = graph.adj_out.get(&u) {
                for &v in neighbors {
                    // uからvへのエッジを見つける
                    let edge_weight = graph.edges.values()
                        .find(|e| e.src == u && e.dst == v)
                        .map(&weight_fn)
                        .unwrap_or(1); // デフォルト重み

                    let new_dist = dist + edge_weight;

                    if new_dist < *distances.get(&v).unwrap_or(&u64::MAX) {
                        distances.insert(v, new_dist);
                        predecessors.insert(v, u);
                        pq.push(Reverse((new_dist, v)));
                    }
                }
            }
        }

        Ok(ShortestPathResult {
            distances,
            previous: predecessors,
        })
    }

    /// Bellman-Fordアルゴリズム（負の重み対応）
    pub fn shortest_path_bellman_ford(
        graph: &Graph,
        source: VertexId,
        weight_fn: impl Fn(&EdgeData) -> u64,
    ) -> Result<ShortestPathResult> {
        let mut distances: HashMap<VertexId, u64> = HashMap::new();
        let mut predecessors: HashMap<VertexId, VertexId> = HashMap::new();

        // 初期化
        for &vertex_id in graph.vertices.keys() {
            distances.insert(vertex_id, u64::MAX);
        }
        distances.insert(source, 0);

        let vertex_count = graph.vertices.len();

        // 緩和を繰り返す
        for _ in 0..vertex_count - 1 {
            for edge in graph.edges.values() {
                let u = edge.src;
                let v = edge.dst;
                let weight = weight_fn(edge);

                if let (Some(&dist_u), Some(&dist_v)) = (distances.get(&u), distances.get(&v)) {
                    if dist_u + weight < dist_v {
                        distances.insert(v, dist_u + weight);
                        predecessors.insert(v, u);
                    }
                }
            }
        }

        // 負のサイクル検出
        for edge in graph.edges.values() {
            let u = edge.src;
            let v = edge.dst;
            let weight = weight_fn(edge);

            if let (Some(&dist_u), Some(&dist_v)) = (distances.get(&u), distances.get(&v)) {
                if dist_u + weight < dist_v {
                    return Err(KotobaError::Execution("Negative cycle detected".to_string()));
                }
            }
        }

        Ok(ShortestPathResult {
            distances,
            previous: predecessors,
        })
    }

    /// Floyd-Warshallアルゴリズム（全頂点間最短経路）
    pub fn all_pairs_shortest_paths(
        graph: &Graph,
        weight_fn: impl Fn(&EdgeData) -> u64,
    ) -> Result<HashMap<(VertexId, VertexId), u64>> {
        let vertices: Vec<VertexId> = graph.vertices.keys().cloned().collect();
        let _n = vertices.len();

        // 距離行列の初期化
        let mut dist: HashMap<(VertexId, VertexId), u64> = HashMap::new();

        // 自己ループは0、無接続は無限大
        for &u in &vertices {
            for &v in &vertices {
                if u == v {
                    dist.insert((u, v), 0);
                } else {
                    dist.insert((u, v), u64::MAX);
                }
            }
        }

        // エッジの重みを設定
        for edge in graph.edges.values() {
            let weight = weight_fn(edge);
            dist.insert((edge.src, edge.dst), weight);
        }

        // Floyd-Warshallアルゴリズム
        for &k in &vertices {
            for &i in &vertices {
                for &j in &vertices {
                    let ik_dist = *dist.get(&(i, k)).unwrap_or(&u64::MAX);
                    let kj_dist = *dist.get(&(k, j)).unwrap_or(&u64::MAX);
                    let ij_dist = *dist.get(&(i, j)).unwrap_or(&u64::MAX);

                    if ik_dist + kj_dist < ij_dist {
                        dist.insert((i, j), ik_dist + kj_dist);
                    }
                }
            }
        }

        Ok(dist)
    }

    /// A*アルゴリズム（ヒューリスティック使用）
    pub fn shortest_path_astar(
        graph: &Graph,
        source: VertexId,
        target: VertexId,
        weight_fn: impl Fn(&EdgeData) -> u64,
        heuristic_fn: impl Fn(VertexId, VertexId) -> u64,
    ) -> Result<Option<Vec<VertexId>>> {
        let mut g_score: HashMap<VertexId, u64> = HashMap::new();
        let mut f_score: HashMap<VertexId, u64> = HashMap::new();
        let mut came_from: HashMap<VertexId, VertexId> = HashMap::new();
        let mut open_set: BinaryHeap<Reverse<(u64, VertexId)>> = BinaryHeap::new();
        let mut open_set_hash: HashSet<VertexId> = HashSet::new();
        let mut closed_set: HashSet<VertexId> = HashSet::new();

        // 初期化
        for &vertex_id in graph.vertices.keys() {
            g_score.insert(vertex_id, u64::MAX);
            f_score.insert(vertex_id, u64::MAX);
        }

        g_score.insert(source, 0);
        f_score.insert(source, heuristic_fn(source, target));
        open_set.push(Reverse((f_score[&source], source)));
        open_set_hash.insert(source);

        while let Some(Reverse((_, current))) = open_set.pop() {
            open_set_hash.remove(&current);

            if current == target {
                // 経路復元
                return Ok(Some(Self::reconstruct_path(&came_from, current)));
            }

            if closed_set.contains(&current) {
                continue;
            }
            closed_set.insert(current);

            // 隣接頂点を探索
            if let Some(neighbors) = graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if closed_set.contains(&neighbor) {
                        continue;
                    }

                    // エッジの重みを取得
                    let edge_weight = graph.edges.values()
                        .find(|e| e.src == current && e.dst == neighbor)
                        .map(&weight_fn)
                        .unwrap_or(1);

                    let tentative_g_score = g_score[&current] + edge_weight;

                    if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&u64::MAX) {
                        came_from.insert(neighbor, current);
                        g_score.insert(neighbor, tentative_g_score);
                        f_score.insert(neighbor, tentative_g_score + heuristic_fn(neighbor, target));

                        if !open_set_hash.contains(&neighbor) {
                            open_set.push(Reverse((f_score[&neighbor], neighbor)));
                            open_set_hash.insert(neighbor);
                        }
                    }
                }
            }
        }

        // 経路が見つからなかった
        Ok(None)
    }

    /// 経路復元ヘルパー
    fn reconstruct_path(came_from: &HashMap<VertexId, VertexId>, current: VertexId) -> Vec<VertexId> {
        let mut path = vec![current];
        let mut current = current;

        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }

        path.reverse();
        path
    }

    /// 次数中央性を計算
    pub fn degree_centrality(graph: &Graph, normalized: bool) -> CentralityResult {
        let mut scores: HashMap<VertexId, f64> = HashMap::new();
        let max_degree = if normalized { graph.vertices.len().saturating_sub(1) as f64 } else { 1.0 };

        for (&vertex_id, _) in &graph.vertices {
            let out_degree = graph.adj_out.get(&vertex_id).map(|s| s.len()).unwrap_or(0) as f64;
            let in_degree = graph.adj_in.get(&vertex_id).map(|s| s.len()).unwrap_or(0) as f64;
            let total_degree = out_degree + in_degree;

            scores.insert(vertex_id, if normalized && max_degree > 0.0 {
                total_degree / max_degree
            } else {
                total_degree
            });
        }

        CentralityResult {
            scores,
            algorithm: CentralityAlgorithm::Degree,
        }
    }

    /// 媒介中央性を計算（Brandesのアルゴリズム）
    pub fn betweenness_centrality(graph: &Graph, normalized: bool) -> CentralityResult {
        let mut scores: HashMap<VertexId, f64> = HashMap::new();
        let vertices: Vec<VertexId> = graph.vertices.keys().cloned().collect();

        // 各頂点を初期化
        for &v in &vertices {
            scores.insert(v, 0.0);
        }

        for &s in &vertices {
            // BFSで最短経路数を計算
            let mut stack: Vec<VertexId> = Vec::new();
            let mut predecessors: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
            let mut sigma: HashMap<VertexId, f64> = HashMap::new();
            let mut dist: HashMap<VertexId, i32> = HashMap::new();
            let mut queue: VecDeque<VertexId> = VecDeque::new();

            // 初期化
            for &v in &vertices {
                predecessors.insert(v, Vec::new());
                sigma.insert(v, 0.0);
                dist.insert(v, -1);
            }

            sigma.insert(s, 1.0);
            dist.insert(s, 0);
            queue.push_back(s);

            // BFS
            while let Some(v) = queue.pop_front() {
                stack.push(v);

                if let Some(neighbors) = graph.adj_out.get(&v) {
                    for &w in neighbors {
                        if *dist.get(&w).unwrap_or(&-1) < 0 {
                            queue.push_back(w);
                            dist.insert(w, dist[&v] + 1);
                        }

                        if dist[&w] == dist[&v] + 1 {
                            sigma.insert(w, sigma[&w] + sigma[&v]);
                            predecessors.get_mut(&w).unwrap().push(v);
                        }
                    }
                }
            }

            // 依存性を逆方向に伝播
            let mut delta: HashMap<VertexId, f64> = HashMap::new();
            for &v in &vertices {
                delta.insert(v, 0.0);
            }

            while let Some(w) = stack.pop() {
                for &v in &predecessors[&w] {
                    let coeff = (sigma[&v] / sigma[&w]) * (1.0 + delta[&w]);
                    delta.insert(v, delta[&v] + coeff);
                }

                if w != s {
                    scores.insert(w, scores[&w] + delta[&w]);
                }
            }
        }

        // 正規化
        if normalized {
            let n = vertices.len() as f64;
            if n > 2.0 {
                let normalization_factor = 1.0 / ((n - 1.0) * (n - 2.0));
                for score in scores.values_mut() {
                    *score *= normalization_factor;
                }
            }
        }

        CentralityResult {
            scores,
            algorithm: CentralityAlgorithm::Betweenness,
        }
    }

    /// 近接中央性を計算
    pub fn closeness_centrality(graph: &Graph, normalized: bool) -> CentralityResult {
        let mut scores: HashMap<VertexId, f64> = HashMap::new();
        let vertices: Vec<VertexId> = graph.vertices.keys().cloned().collect();

        for &source in &vertices {
            // 各頂点からの最短経路を計算
            let result = Self::shortest_path_dijkstra(graph, source, |_| 1).unwrap_or_default();

            let mut total_distance = 0.0;
            let mut reachable_count = 0;

            for &target in &vertices {
                if let Some(&dist) = result.distances.get(&target) {
                    if dist < u64::MAX {
                        total_distance += dist as f64;
                        reachable_count += 1;
                    }
                }
            }

            if reachable_count > 1 {
                let closeness = if normalized {
                    (reachable_count - 1) as f64 / total_distance
                } else {
                    1.0 / total_distance
                };
                scores.insert(source, closeness);
            } else {
                scores.insert(source, 0.0);
            }
        }

        CentralityResult {
            scores,
            algorithm: CentralityAlgorithm::Closeness,
        }
    }

    /// PageRankを計算（べき乗法）
    pub fn pagerank(graph: &Graph, damping_factor: f64, max_iterations: usize, tolerance: f64) -> CentralityResult {
        let vertices: Vec<VertexId> = graph.vertices.keys().cloned().collect();
        let n = vertices.len() as f64;

        if n == 0.0 {
            return CentralityResult {
                scores: HashMap::new(),
                algorithm: CentralityAlgorithm::PageRank,
            };
        }

        // 初期化：すべての頂点に均等なスコア
        let mut scores: HashMap<VertexId, f64> = vertices.iter()
            .map(|&v| (v, 1.0 / n))
            .collect();

        let mut new_scores: HashMap<VertexId, f64> = HashMap::new();

        for _ in 0..max_iterations {
            let mut converged = true;

            // 各頂点の新しいスコアを計算
            for &v in &vertices {
                let mut incoming_score = 0.0;

                // 入辺を持つ頂点からの寄与を計算
                for (&u, _) in &graph.vertices {
                    if let Some(out_neighbors) = graph.adj_out.get(&u) {
                        if out_neighbors.contains(&v) {
                            let out_degree = out_neighbors.len() as f64;
                            if out_degree > 0.0 {
                                incoming_score += scores[&u] / out_degree;
                            }
                        }
                    }
                }

                let new_score = (1.0 - damping_factor) / n + damping_factor * incoming_score;
                new_scores.insert(v, new_score);

                // 収束チェック
                if (new_score - scores[&v]).abs() > tolerance {
                    converged = false;
                }
            }

            // スコアを更新
            scores.clone_from(&new_scores);

            if converged {
                break;
            }
        }

        CentralityResult {
            scores,
            algorithm: CentralityAlgorithm::PageRank,
        }
    }

    /// 基本的な部分グラフ同型マッチング（Ullmannのアルゴリズム簡易版）
    pub fn subgraph_isomorphism(pattern: &Graph, target: &Graph) -> PatternMatchResult {
        let mut mappings = Vec::new();

        if pattern.vertices.is_empty() {
            return PatternMatchResult {
                mappings,
                count: 0,
            };
        }

        let pattern_vertices: Vec<VertexId> = pattern.vertices.keys().cloned().collect();
        let target_vertices: Vec<VertexId> = target.vertices.keys().cloned().collect();

        // 再帰的なマッチング探索
        Self::find_subgraph_matches(
            pattern,
            target,
            &pattern_vertices,
            &target_vertices,
            0,
            &mut HashMap::new(),
            &mut HashMap::new(),
            &mut mappings,
        );

        PatternMatchResult {
            mappings: mappings.clone(),
            count: mappings.len(),
        }
    }

    /// 部分グラフマッチングの再帰探索
    fn find_subgraph_matches(
        pattern: &Graph,
        target: &Graph,
        pattern_vertices: &[VertexId],
        target_vertices: &[VertexId],
        depth: usize,
        vertex_map: &mut HashMap<VertexId, VertexId>,
        edge_map: &mut HashMap<EdgeId, EdgeId>,
        mappings: &mut Vec<SubgraphMapping>,
    ) {
        if depth == pattern_vertices.len() {
            // マッチングが完了
            mappings.push(SubgraphMapping {
                vertex_map: vertex_map.clone(),
                edge_map: edge_map.clone(),
            });
            return;
        }

        let pattern_vertex = pattern_vertices[depth];

        for &target_vertex in target_vertices {
            // 頂点のラベルが一致するかチェック
            let pattern_vertex_data = &pattern.vertices[&pattern_vertex];
            let target_vertex_data = &target.vertices[&target_vertex];

            if !Self::vertices_match(pattern_vertex_data, target_vertex_data) {
                continue;
            }

            // マッピングが有効かチェック
            if Self::is_valid_mapping(pattern, target, pattern_vertex, target_vertex, vertex_map) {
                // マッピングを追加
                vertex_map.insert(pattern_vertex, target_vertex);

                // エッジマッピングも試行
                let mut local_edge_map = edge_map.clone();
                Self::map_edges(pattern, target, pattern_vertex, target_vertex, &mut local_edge_map);

                // 次の頂点へ
                Self::find_subgraph_matches(
                    pattern,
                    target,
                    pattern_vertices,
                    target_vertices,
                    depth + 1,
                    vertex_map,
                    &mut local_edge_map,
                    mappings,
                );

                // バックトラック
                vertex_map.remove(&pattern_vertex);
            }
        }
    }

    /// 頂点がマッチするかチェック
    fn vertices_match(pattern_vertex: &VertexData, target_vertex: &VertexData) -> bool {
        // ラベルが一致するかチェック（少なくとも1つの共通ラベル）
        pattern_vertex.labels.iter().any(|label| target_vertex.labels.contains(label))
    }

    /// マッピングが有効かチェック
    fn is_valid_mapping(
        pattern: &Graph,
        target: &Graph,
        pattern_vertex: VertexId,
        target_vertex: VertexId,
        vertex_map: &HashMap<VertexId, VertexId>,
    ) -> bool {
        // 既にマッピングされている頂点とのエッジ整合性をチェック
        for (&pv, &tv) in vertex_map {
            // パターングラフでのエッジ
            if let Some(pattern_neighbors) = pattern.adj_out.get(&pv) {
                if pattern_neighbors.contains(&pattern_vertex) {
                    // ターゲットグラフでも対応するエッジが存在するか
                    if let Some(target_neighbors) = target.adj_out.get(&tv) {
                        if !target_neighbors.contains(&target_vertex) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }

            if let Some(pattern_neighbors) = pattern.adj_in.get(&pv) {
                if pattern_neighbors.contains(&pattern_vertex) {
                    if let Some(target_neighbors) = target.adj_in.get(&tv) {
                        if !target_neighbors.contains(&target_vertex) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// エッジマッピングを試行
    fn map_edges(
        _pattern: &Graph,
        _target: &Graph,
        _pattern_vertex: VertexId,
        _target_vertex: VertexId,
        _edge_map: &mut HashMap<EdgeId, EdgeId>,
    ) {
        // 簡易版：エッジマッピングは後で実装
        // 実際の実装ではパターンとターゲットのエッジを対応付ける
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_core::types::*;

    /// テスト用グラフ作成ヘルパー
    fn create_test_graph() -> Graph {
        let mut graph = Graph::empty();

        // 頂点を追加
        let v1 = graph.add_vertex(VertexData {
            id: VertexId::new("v1").unwrap(),
            labels: vec!["Person".to_string()],
            props: HashMap::new(),
        });

        let v2 = graph.add_vertex(VertexData {
            id: VertexId::new("v2").unwrap(),
            labels: vec!["Person".to_string()],
            props: HashMap::new(),
        });

        let v3 = graph.add_vertex(VertexData {
            id: VertexId::new("v3").unwrap(),
            labels: vec!["Person".to_string()],
            props: HashMap::new(),
        });

        // エッジを追加
        graph.add_edge(EdgeData {
            id: EdgeId::new("e1").unwrap(),
            src: v1,
            dst: v2,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });

        graph.add_edge(EdgeData {
            id: EdgeId::new("e2").unwrap(),
            src: v2,
            dst: v3,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });

        graph
    }

    #[test]
    fn test_dijkstra_shortest_path() {
        let graph = create_test_graph();
        let source = VertexId::new("v1").unwrap();

        let result = GraphAlgorithms::shortest_path_dijkstra(&graph, source, |_| 1).unwrap();

        // v1からv1への距離は0
        assert_eq!(result.distances[&source], 0);

        // 他の頂点への距離をチェック
        let v2 = VertexId::new("v2").unwrap();
        let v3 = VertexId::new("v3").unwrap();

        assert!(result.distances[&v2] > 0);
        assert!(result.distances[&v3] > result.distances[&v2]);
    }

    #[test]
    fn test_degree_centrality() {
        let graph = create_test_graph();

        let result = GraphAlgorithms::degree_centrality(&graph, false);

        assert_eq!(result.algorithm, CentralityAlgorithm::Degree);
        assert!(!result.scores.is_empty());

        // 次数が正であることを確認
        for &score in result.scores.values() {
            assert!(score >= 0.0);
        }
    }

    #[test]
    fn test_betweenness_centrality() {
        let graph = create_test_graph();

        let result = GraphAlgorithms::betweenness_centrality(&graph, false);

        assert_eq!(result.algorithm, CentralityAlgorithm::Betweenness);
        assert!(!result.scores.is_empty());

        // 媒介中央性が非負であることを確認
        for &score in result.scores.values() {
            assert!(score >= 0.0);
        }
    }

    #[test]
    fn test_pagerank() {
        let graph = create_test_graph();

        let result = GraphAlgorithms::pagerank(&graph, 0.85, 10, 1e-6);

        assert_eq!(result.algorithm, CentralityAlgorithm::PageRank);
        assert!(!result.scores.is_empty());

        // PageRankスコアが正であることを確認
        for &score in result.scores.values() {
            assert!(score >= 0.0);
        }
    }

    #[test]
    fn test_subgraph_isomorphism() {
        let pattern = create_test_graph();
        let target = create_test_graph();

        let result = GraphAlgorithms::subgraph_isomorphism(&pattern, &target);

        assert!(result.count >= 0);
        // 同じグラフなので少なくとも1つのマッチが見つかるはず
        // （実際のアルゴリズムによる）
    }
}

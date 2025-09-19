//! グラフアルゴリズム実装
//!
//! このモジュールはグラフ構造に対する基本的なアルゴリズムを提供します。

use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::types::VertexId;
use crate::types::*;

/// グラフトラバーサルアルゴリズム
pub struct GraphTraversal<'a> {
    graph: &'a Graph,
    visited: HashSet<VertexId>,
    queue: VecDeque<VertexId>,
    stack: Vec<VertexId>,
}

impl<'a> GraphTraversal<'a> {
    /// 新しいトラバーサルを作成
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
            visited: HashSet::new(),
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    /// BFS (Breadth-First Search) を実行
    pub fn bfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.queue.clear();
        let mut result = Vec::new();

        self.visited.insert(start);
        self.queue.push_back(start);

        while let Some(current) = self.queue.pop_front() {
            result.push(current);

            // 隣接頂点を探索
            if let Some(neighbors) = self.graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if !self.visited.contains(&neighbor) {
                        self.visited.insert(neighbor);
                        self.queue.push_back(neighbor);
                    }
                }
            }
        }

        result
    }

    /// DFS (Depth-First Search) を実行
    pub fn dfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.stack.clear();
        let mut result = Vec::new();

        self.stack.push(start);

        while let Some(current) = self.stack.pop() {
            if !self.visited.contains(&current) {
                self.visited.insert(current);
                result.push(current);

                // 隣接頂点をスタックに積む（逆順）
                if let Some(neighbors) = self.graph.adj_out.get(&current) {
                    let neighbors_vec: Vec<_> = neighbors.iter().collect();
                    for &neighbor in neighbors_vec.iter().rev() {
                        if !self.visited.contains(&neighbor) {
                            self.stack.push(*neighbor);
                        }
                    }
                }
            }
        }

        result
    }

    /// 接続成分を検出
    pub fn connected_components(&mut self) -> Vec<Vec<VertexId>> {
        self.visited.clear();
        let mut components = Vec::new();

        for &vertex_id in self.graph.vertices.keys() {
            if !self.visited.contains(&vertex_id) {
                let component = self.bfs(vertex_id);
                components.push(component);
            }
        }

        components
    }
}

/// グラフアルゴリズムのユーティリティ関数
pub struct GraphAlgorithms;

impl GraphAlgorithms {
    /// 頂点間の最短経路を計算（BFS使用）
    pub fn shortest_path(graph: &Graph, start: VertexId, end: VertexId) -> Option<Vec<VertexId>> {
        if start == end {
            return Some(vec![start]);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent = HashMap::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        parent.insert(neighbor, current);
                        queue.push_back(neighbor);

                        if neighbor == end {
                            // パスを再構築
                            let mut path = vec![end];
                            let mut current_vertex = end;
                            while let Some(&parent_vertex) = parent.get(&current_vertex) {
                                path.push(parent_vertex);
                                current_vertex = parent_vertex;
                                if parent_vertex == start {
                                    break;
                                }
                            }
                            path.reverse();
                            return Some(path);
                        }
                    }
                }
            }
        }

        None // パスが見つからない
    }

    /// グラフのトポロジカルソート
    pub fn topological_sort(graph: &Graph) -> Result<Vec<VertexId>> {
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // 入次数を計算
        for &vertex_id in graph.vertices.keys() {
            in_degree.insert(vertex_id, 0);
        }

        for neighbors in graph.adj_in.values() {
            for &neighbor in neighbors {
                if let Some(degree) = in_degree.get_mut(&neighbor) {
                    *degree += 1;
                }
            }
        }

        // 入次数が0の頂点をキューに追加
        for (&vertex_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(vertex_id);
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current);

            if let Some(neighbors) = graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        // サイクル検出
        if result.len() != graph.vertices.len() {
            return Err(kotoba_errors::KotobaError::Validation("Graph contains a cycle".to_string()));
        }

        Ok(result)
    }

    /// 強連結成分を検出（Kosarajuのアルゴリズム）
    pub fn strongly_connected_components(graph: &Graph) -> Vec<Vec<VertexId>> {
        // 簡易実装：現在は接続成分として扱う
        let mut traversal = GraphTraversal::new(graph);
        traversal.connected_components()
    }

    /// 頂点の次数を計算
    pub fn degrees(graph: &Graph) -> HashMap<VertexId, (usize, usize)> {
        let mut degrees = HashMap::new();

        for &vertex_id in graph.vertices.keys() {
            let out_degree = graph.adj_out.get(&vertex_id).map(|s| s.len()).unwrap_or(0);
            let in_degree = graph.adj_in.get(&vertex_id).map(|s| s.len()).unwrap_or(0);
            degrees.insert(vertex_id, (in_degree, out_degree));
        }

        degrees
    }

    /// グラフの密度を計算
    pub fn density(graph: &Graph) -> f64 {
        let n = graph.vertices.len() as f64;
        let m = graph.edges.len() as f64;

        if n <= 1.0 {
            0.0
        } else {
            2.0 * m / (n * (n - 1.0))
        }
    }

    /// グラフが連結かどうかを判定
    pub fn is_connected(graph: &Graph) -> bool {
        if graph.vertices.is_empty() {
            return true;
        }

        let start = *graph.vertices.keys().next().unwrap();
        let mut traversal = GraphTraversal::new(graph);
        let reachable = traversal.bfs(start);

        reachable.len() == graph.vertices.len()
    }

    /// グラフにサイクルが存在するかどうかを判定
    pub fn has_cycle(graph: &Graph) -> bool {
        Self::topological_sort(graph).is_err()
    }
}

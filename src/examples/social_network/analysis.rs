//! Social Network分析
//!
//! ソーシャルネットワークのグラフ分析を行うモジュール

use crate::examples::social_network::*;
use crate::*;
use crate::types::VertexId;
use std::collections::{HashMap, HashSet};

/// ソーシャルネットワーク分析器
pub struct SocialNetworkAnalyzer;

impl SocialNetworkAnalyzer {
    /// ネットワークの中心性を計算（次数中心性）
    pub fn calculate_degree_centrality(network: &SocialNetwork) -> HashMap<String, f64> {
        let graph = network.graph.read();
        let mut centrality = HashMap::new();

        for user in &network.users {
            let degree = graph.degree(&user.id) as f64;
            let max_possible_degree = (network.users.len() - 1) as f64;
            centrality.insert(user.name.clone(), degree / max_possible_degree);
        }

        centrality
    }

    /// クラスタリング係数を計算
    pub fn calculate_clustering_coefficient(network: &SocialNetwork) -> HashMap<String, f64> {
        let graph = network.graph.read();
        let mut coefficients = HashMap::new();

        for user in &network.users {
            let empty_set = HashSet::new();
            let neighbors_set = graph.adj_out.get(&user.id).unwrap_or(&empty_set);
            let neighbors: HashSet<_> = neighbors_set.iter().collect();

            if neighbors.len() < 2 {
                coefficients.insert(user.name.clone(), 0.0);
                continue;
            }

            // 隣接ノード間のエッジ数をカウント
            let mut triangles = 0;
            for &neighbor1 in &neighbors {
                for &neighbor2 in &neighbors {
                    if neighbor1 != neighbor2 {
                        if graph.adj_out.get(neighbor1)
                            .unwrap_or(&HashSet::new())
                            .contains(neighbor2) {
                            triangles += 1;
                        }
                    }
                }
            }

            // クラスタリング係数を計算
            let possible_triangles = neighbors.len() * (neighbors.len() - 1);
            let coefficient = if possible_triangles > 0 {
                triangles as f64 / possible_triangles as f64
            } else {
                0.0
            };

            coefficients.insert(user.name.clone(), coefficient);
        }

        coefficients
    }

    /// コミュニティ検出（簡易版：連結成分ベース）
    pub fn detect_communities(network: &SocialNetwork) -> Vec<Vec<String>> {
        let graph = network.graph.read();
        let mut visited = HashSet::new();
        let mut communities = Vec::new();

        // 各ユーザーIDを名前マッピング
        let mut id_to_name = HashMap::new();
        for user in &network.users {
            id_to_name.insert(user.id, user.name.clone());
        }

        for user in &network.users {
            if !visited.contains(&user.id) {
                // DFSで連結成分を探索
                let mut community = Vec::new();
                let mut stack = vec![user.id];

                while let Some(current) = stack.pop() {
                    if visited.contains(&current) {
                        continue;
                    }
                    visited.insert(current);

                    if let Some(name) = id_to_name.get(&current) {
                        community.push(name.clone());
                    }

                    // 隣接ノードをスタックに追加
                    if let Some(neighbors) = graph.adj_out.get(&current) {
                        for neighbor in neighbors {
                            if !visited.contains(neighbor) {
                                stack.push(*neighbor);
                            }
                        }
                    }
                }

                if !community.is_empty() {
                    communities.push(community);
                }
            }
        }

        communities
    }

    /// ネットワークの密度を計算
    pub fn calculate_network_density(network: &SocialNetwork) -> f64 {
        let graph = network.graph.read();
        let node_count = network.users.len() as f64;
        let edge_count = graph.edge_count() as f64 / 2.0; // 有向グラフなので2で割る

        if node_count < 2.0 {
            0.0
        } else {
            let max_possible_edges = node_count * (node_count - 1.0) / 2.0;
            edge_count / max_possible_edges
        }
    }

    /// 平均経路長を計算（簡易版）
    pub fn calculate_average_path_length(network: &SocialNetwork, max_depth: usize) -> f64 {
        let graph = network.graph.read();
        let mut total_distance = 0.0;
        let mut total_pairs = 0.0;

        for (i, user1) in network.users.iter().enumerate() {
            for user2 in &network.users[i + 1..] {
                let distance = Self::shortest_path_distance(&graph, user1.id, user2.id, max_depth);
                if let Some(dist) = distance {
                    total_distance += dist as f64;
                    total_pairs += 1.0;
                }
            }
        }

        if total_pairs > 0.0 {
            total_distance / total_pairs
        } else {
            0.0
        }
    }

    /// 2つのノード間の最短経路距離を計算
    fn shortest_path_distance(graph: &Graph, start: VertexId, end: VertexId, max_depth: usize) -> Option<usize> {
        if start == end {
            return Some(0);
        }

        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        visited.insert(start);
        queue.push_back((start, 0));

        while let Some((current, distance)) = queue.pop_front() {
            if distance >= max_depth {
                continue;
            }

            if let Some(neighbors) = graph.adj_out.get(&current) {
                for neighbor in neighbors {
                    if *neighbor == end {
                        return Some(distance + 1);
                    }

                    if !visited.contains(neighbor) {
                        visited.insert(*neighbor);
                        queue.push_back((*neighbor, distance + 1));
                    }
                }
            }
        }

        None
    }

    /// 次数分布を分析
    pub fn analyze_degree_distribution(network: &SocialNetwork) -> HashMap<usize, usize> {
        let graph = network.graph.read();
        let mut distribution = HashMap::new();

        for user in &network.users {
            let degree = graph.degree(&user.id);
            *distribution.entry(degree).or_insert(0) += 1;
        }

        distribution
    }

    /// インフルエンサーを特定（高い次数を持つノード）
    pub fn identify_influencers(network: &SocialNetwork, top_k: usize) -> Vec<(String, usize)> {
        let graph = network.graph.read();
        let mut degrees: Vec<(String, usize)> = network.users.iter()
            .map(|user| (user.name.clone(), graph.degree(&user.id)))
            .collect();

        degrees.sort_by(|a, b| b.1.cmp(&a.1));
        degrees.into_iter().take(top_k).collect()
    }

    /// ネットワークの健全性をチェック
    pub fn check_network_health(network: &SocialNetwork) -> NetworkHealthReport {
        let graph = network.graph.read();

        let mut isolated_nodes = 0;
        let mut highly_connected_nodes = 0;
        let mut total_degree = 0;

        for user in &network.users {
            let degree = graph.degree(&user.id);
            total_degree += degree;

            if degree == 0 {
                isolated_nodes += 1;
            } else if degree > network.users.len() / 10 {
                highly_connected_nodes += 1;
            }
        }

        let avg_degree = if !network.users.is_empty() {
            total_degree as f64 / network.users.len() as f64
        } else {
            0.0
        };

        NetworkHealthReport {
            total_users: network.users.len(),
            total_posts: network.posts.len(),
            total_edges: graph.edge_count(),
            isolated_nodes,
            highly_connected_nodes,
            average_degree: avg_degree,
            network_density: Self::calculate_network_density(network),
        }
    }
}

/// ネットワーク健全性レポート
#[derive(Debug, Clone)]
pub struct NetworkHealthReport {
    pub total_users: usize,
    pub total_posts: usize,
    pub total_edges: usize,
    pub isolated_nodes: usize,
    pub highly_connected_nodes: usize,
    pub average_degree: f64,
    pub network_density: f64,
}

impl NetworkHealthReport {
    /// レポートを表示
    pub fn print(&self) {
        println!("\n=== Network Health Report ===");
        println!("Total Users: {}", self.total_users);
        println!("Total Posts: {}", self.total_posts);
        println!("Total Edges: {}", self.total_edges);
        println!("Isolated Nodes: {} ({:.1}%)",
                 self.isolated_nodes,
                 self.isolated_nodes as f64 / self.total_users as f64 * 100.0);
        println!("Highly Connected Nodes: {} ({:.1}%)",
                 self.highly_connected_nodes,
                 self.highly_connected_nodes as f64 / self.total_users as f64 * 100.0);
        println!("Average Degree: {:.2}", self.average_degree);
        println!("Network Density: {:.4}", self.network_density);

        // 健全性の評価
        if self.network_density > 0.1 {
            println!("Network Health: 🟢 Dense and well-connected");
        } else if self.network_density > 0.01 {
            println!("Network Health: 🟡 Moderately connected");
        } else {
            println!("Network Health: 🔴 Sparse network");
        }

        if self.isolated_nodes as f64 / self.total_users as f64 > 0.5 {
            println!("Warning: High number of isolated nodes");
        }

        println!("=============================\n");
    }
}

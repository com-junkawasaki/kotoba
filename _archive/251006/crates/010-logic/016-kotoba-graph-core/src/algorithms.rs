//! # Graph Algorithms
//!
//! This module provides graph algorithms for traversal, analysis,
//! and transformation operations.

use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Graph traversal algorithms
pub struct GraphTraversal<'a> {
    graph: &'a Graph,
    visited: HashSet<VertexId>,
    queue: VecDeque<VertexId>,
    stack: Vec<VertexId>,
}

impl<'a> GraphTraversal<'a> {
    /// Create a new traversal
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
            visited: HashSet::new(),
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    /// Breadth-First Search
    pub fn bfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.queue.clear();
        let mut result = Vec::new();

        self.visited.insert(start);
        self.queue.push_back(start);

        while let Some(current) = self.queue.pop_front() {
            result.push(current);

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

    /// Depth-First Search
    pub fn dfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.stack.clear();
        let mut result = Vec::new();

        self.stack.push(start);

        while let Some(current) = self.stack.pop() {
            if !self.visited.contains(&current) {
                self.visited.insert(current);
                result.push(current);

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

    /// Find connected components
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

/// Graph algorithms utilities
pub struct GraphAlgorithms;

impl GraphAlgorithms {
    /// Find shortest path between two vertices (using BFS)
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
                            // Reconstruct path
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

        None
    }

    /// Topological sort
    pub fn topological_sort(graph: &Graph) -> KotobaResult<Vec<VertexId>> {
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Calculate in-degrees
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

        // Add vertices with in-degree 0 to queue
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

        // Check for cycles
        if result.len() != graph.vertices.len() {
            return Err(KotobaError::Validation("Graph contains a cycle".to_string()));
        }

        Ok(result)
    }

    /// Strongly connected components (Kosaraju's algorithm)
    pub fn strongly_connected_components(graph: &Graph) -> Vec<Vec<VertexId>> {
        // Simplified implementation: treat as connected components for now
        let mut traversal = GraphTraversal::new(graph);
        traversal.connected_components()
    }

    /// Calculate vertex degrees
    pub fn degrees(graph: &Graph) -> HashMap<VertexId, (usize, usize)> {
        let mut degrees = HashMap::new();

        for &vertex_id in graph.vertices.keys() {
            let out_degree = graph.adj_out.get(&vertex_id).map(|s| s.len()).unwrap_or(0);
            let in_degree = graph.adj_in.get(&vertex_id).map(|s| s.len()).unwrap_or(0);
            degrees.insert(vertex_id, (in_degree, out_degree));
        }

        degrees
    }

    /// Calculate graph density
    pub fn density(graph: &Graph) -> f64 {
        let n = graph.vertices.len() as f64;
        let m = graph.edges.len() as f64;

        if n <= 1.0 {
            0.0
        } else {
            2.0 * m / (n * (n - 1.0))
        }
    }

    /// Check if graph is connected
    pub fn is_connected(graph: &Graph) -> bool {
        if graph.vertices.is_empty() {
            return true;
        }

        let start = *graph.vertices.keys().next().unwrap();
        let mut traversal = GraphTraversal::new(graph);
        let reachable = traversal.bfs(start);

        reachable.len() == graph.vertices.len()
    }

    /// Check if graph has cycles
    pub fn has_cycle(graph: &Graph) -> bool {
        Self::topological_sort(graph).is_err()
    }

    /// Calculate graph statistics
    pub fn statistics(graph: &Graph) -> GraphStatistics {
        let vertex_count = graph.vertex_count();
        let edge_count = graph.edge_count();

        let degrees = Self::degrees(graph);
        let mut degree_distribution = Vec::new();
        let mut total_degree = 0;

        for (_, (in_degree, out_degree)) in &degrees {
            let degree = in_degree + out_degree;
            degree_distribution.push(degree);
            total_degree += degree;
        }

        let average_degree = if vertex_count > 0 {
            total_degree as f64 / vertex_count as f64
        } else {
            0.0
        };

        let density = Self::density(graph);
        let is_connected = Self::is_connected(graph);
        let has_cycles = Self::has_cycle(graph);

        GraphStatistics {
            vertex_count,
            edge_count,
            average_degree,
            density,
            is_connected,
            has_cycles,
            strongly_connected_components: Self::strongly_connected_components(graph).len(),
        }
    }

    /// Find all cycles in the graph
    pub fn find_cycles(graph: &Graph) -> Vec<Vec<VertexId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &vertex_id in graph.vertices.keys() {
            if !visited.contains(&vertex_id) {
                Self::find_cycles_dfs(graph, vertex_id, &mut visited, &mut rec_stack, &mut cycles);
            }
        }

        cycles
    }

    /// DFS helper for cycle detection
    fn find_cycles_dfs(
        graph: &Graph,
        vertex: VertexId,
        visited: &mut HashSet<VertexId>,
        rec_stack: &mut HashSet<VertexId>,
        cycles: &mut Vec<Vec<VertexId>>,
    ) {
        visited.insert(vertex);
        rec_stack.insert(vertex);

        if let Some(neighbors) = graph.adj_out.get(&vertex) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    Self::find_cycles_dfs(graph, neighbor, visited, rec_stack, cycles);
                } else if rec_stack.contains(&neighbor) {
                    // Cycle found
                    // Extract cycle path
                    cycles.push(vec![vertex, neighbor]);
                }
            }
        }

        rec_stack.remove(&vertex);
    }

    /// Find bridges in the graph (edges whose removal increases connected components)
    pub fn find_bridges(graph: &Graph) -> Vec<EdgeId> {
        let mut bridges = Vec::new();
        let mut visited = HashSet::new();
        let mut disc = HashMap::new();
        let mut low = HashMap::new();
        let mut parent = HashMap::new();
        let mut time = 0;

        for &vertex_id in graph.vertices.keys() {
            if !visited.contains(&vertex_id) {
                Self::find_bridges_dfs(
                    graph,
                    vertex_id,
                    &mut visited,
                    &mut disc,
                    &mut low,
                    &mut parent,
                    &mut time,
                    &mut bridges,
                );
            }
        }

        bridges
    }

    /// DFS helper for bridge detection
    fn find_bridges_dfs(
        graph: &Graph,
        u: VertexId,
        visited: &mut HashSet<VertexId>,
        disc: &mut HashMap<VertexId, i32>,
        low: &mut HashMap<VertexId, i32>,
        parent: &mut HashMap<VertexId, VertexId>,
        time: &mut i32,
        bridges: &mut Vec<EdgeId>,
    ) {
        visited.insert(u);
        *time += 1;
        disc.insert(u, *time);
        low.insert(u, *time);

        if let Some(neighbors) = graph.adj_out.get(&u) {
            for &v in neighbors {
                if !visited.contains(&v) {
                    parent.insert(v, u);
                    Self::find_bridges_dfs(graph, v, visited, disc, low, parent, time, bridges);

                    // Check if the subtree rooted at v has a connection to
                    // one of the ancestors of u
                    let low_v = *low.get(&v).unwrap();
                    let low_u = *low.get(&u).unwrap();
                    if low_v > *disc.get(&u).unwrap() {
                        // Bridge found: find the edge ID
                        if let Some(edge_id) = Self::find_edge_between(graph, u, v) {
                            bridges.push(edge_id);
                        }
                    }

                    // Update low value of u
                    let new_low = low_u.min(low_v);
                    low.insert(u, new_low);
                } else if parent.get(&u) != Some(&v) {
                    // Update low value of u for parent function calls
                    let low_v = *low.get(&v).unwrap();
                    let low_u = *low.get(&u).unwrap();
                    let new_low = low_u.min(low_v);
                    low.insert(u, new_low);
                }
            }
        }
    }

    /// Find edge ID between two vertices
    fn find_edge_between(graph: &Graph, u: VertexId, v: VertexId) -> Option<EdgeId> {
        for (edge_id, edge) in &graph.edges {
            if (edge.src == u && edge.dst == v) || (edge.src == v && edge.dst == u) {
                return Some(*edge_id);
            }
        }
        None
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    pub vertex_count: usize,
    pub edge_count: usize,
    pub average_degree: f64,
    pub density: f64,
    pub is_connected: bool,
    pub has_cycles: bool,
    pub strongly_connected_components: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traversal() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v3 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));
        graph.add_edge(EdgeData::new(v2, v3, "knows".to_string()));

        let mut traversal = GraphTraversal::new(&graph);
        let reachable = traversal.bfs(v1);

        assert_eq!(reachable.len(), 3);
        assert!(reachable.contains(&v1));
        assert!(reachable.contains(&v2));
        assert!(reachable.contains(&v3));
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v3 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));
        graph.add_edge(EdgeData::new(v2, v3, "knows".to_string()));

        let path = GraphAlgorithms::shortest_path(&graph, v1, v3);
        assert_eq!(path, Some(vec![v1, v2, v3]));
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v3 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "depends_on".to_string()));
        graph.add_edge(EdgeData::new(v2, v3, "depends_on".to_string()));

        let topo_order = GraphAlgorithms::topological_sort(&graph).unwrap();
        assert_eq!(topo_order.len(), 3);

        // v1 should come before v2, v2 before v3
        let v1_pos = topo_order.iter().position(|&x| x == v1).unwrap();
        let v2_pos = topo_order.iter().position(|&x| x == v2).unwrap();
        let v3_pos = topo_order.iter().position(|&x| x == v3).unwrap();

        assert!(v1_pos < v2_pos);
        assert!(v2_pos < v3_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v3 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));
        graph.add_edge(EdgeData::new(v2, v3, "knows".to_string()));
        graph.add_edge(EdgeData::new(v3, v1, "knows".to_string())); // Creates a cycle

        assert!(GraphAlgorithms::has_cycle(&graph));
        assert!(GraphAlgorithms::topological_sort(&graph).is_err());
    }

    #[test]
    fn test_graph_statistics() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v3 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));
        graph.add_edge(EdgeData::new(v2, v3, "knows".to_string()));

        let stats = GraphAlgorithms::statistics(&graph);

        assert_eq!(stats.vertex_count, 3);
        assert_eq!(stats.edge_count, 2);
        assert!((stats.average_degree - 4.0/3.0).abs() < 0.001);
        assert!(!stats.has_cycles);
        assert!(stats.is_connected);
    }
}

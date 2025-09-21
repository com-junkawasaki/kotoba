//! # Knuth-Bendix Ordering
//!
//! This module provides implementation of Knuth-Bendix ordering for term rewriting.

use super::*;
use std::cmp::Ordering;

/// Knuth-Bendix ordering implementation
#[derive(Debug, Clone)]
pub struct KBO {
    /// Precedence relation on function symbols
    pub precedence: HashMap<DefRef, i32>,
    /// Weight function for symbols
    pub weights: HashMap<DefRef, i32>,
    /// Status of function symbols (multiset or lexicographic)
    pub status: HashMap<DefRef, SymbolStatus>,
}

impl KBO {
    /// Create a new KBO with default settings
    pub fn new() -> Self {
        Self {
            precedence: HashMap::new(),
            weights: HashMap::new(),
            status: HashMap::new(),
        }
    }

    /// Set precedence for a function symbol
    pub fn set_precedence(&mut self, symbol: DefRef, precedence: i32) {
        self.precedence.insert(symbol, precedence);
    }

    /// Set weight for a function symbol
    pub fn set_weight(&mut self, symbol: DefRef, weight: i32) {
        self.weights.insert(symbol, weight);
    }

    /// Set status for a function symbol
    pub fn set_status(&mut self, symbol: DefRef, status: SymbolStatus) {
        self.status.insert(symbol, status);
    }

    /// Compare two expressions using KBO
    pub fn compare(&self, expr1: &Expression, expr2: &Expression) -> Ordering {
        match (expr1, expr2) {
            (Expression::Variable(v1), Expression::Variable(v2)) => v1.cmp(v2),
            (Expression::Variable(_), _) => Ordering::Less,
            (_, Expression::Variable(_)) => Ordering::Greater,
            (Expression::Literal(v1), Expression::Literal(v2)) => {
                // Simple comparison for literals
                self.compare_values(v1, v2)
            },
            (Expression::Literal(_), _) => Ordering::Less,
            (_, Expression::Literal(_)) => Ordering::Greater,
            (Expression::Function { function: f1, arguments: args1 },
             Expression::Function { function: f2, arguments: args2 }) => {
                self.compare_functions(f1, args1, f2, args2)
            },
        }
    }

    /// Compare two function applications
    fn compare_functions(
        &self,
        f1: &DefRef,
        args1: &[usize],
        f2: &DefRef,
        args2: &[usize],
    ) -> Ordering {
        // Get precedence values
        let p1 = self.precedence.get(f1).copied().unwrap_or(0);
        let p2 = self.precedence.get(f2).copied().unwrap_or(0);

        match p1.cmp(&p2) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => {
                // Same precedence, compare weights and arguments
                let w1 = self.weights.get(f1).copied().unwrap_or(1);
                let w2 = self.weights.get(f2).copied().unwrap_or(1);

                if w1 != w2 {
                    return w1.cmp(&w2);
                }

                // Compare arguments lexicographically
                let len = args1.len().min(args2.len());
                for i in 0..len {
                    // In a real implementation, we'd need access to the actual expressions
                    // For now, just compare indices
                    match args1[i].cmp(&args2[i]) {
                        Ordering::Less => return Ordering::Less,
                        Ordering::Greater => return Ordering::Greater,
                        Ordering::Equal => continue,
                    }
                }

                // Same arguments up to common length, compare arities
                args1.len().cmp(&args2.len())
            }
        }
    }

    /// Compare two values
    fn compare_values(&self, _v1: &Value, _v2: &Value) -> Ordering {
        // Simple value comparison - in practice would handle different value types
        Ordering::Equal
    }
}

/// Symbol status in KBO
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolStatus {
    /// Lexicographic ordering of arguments
    Lexicographic,
    /// Multiset ordering of arguments
    Multiset,
}

/// KBO with specific settings for term rewriting
pub struct KBOTermOrdering {
    /// The underlying KBO
    pub kbo: KBO,
    /// Weight of variables
    pub variable_weight: i32,
}

impl KBOTermOrdering {
    /// Create a new KBO term ordering
    pub fn new(variable_weight: i32) -> Self {
        Self {
            kbo: KBO::new(),
            variable_weight,
        }
    }

    /// Check if one term is greater than another
    pub fn greater_than(&self, term1: &Expression, term2: &Expression) -> bool {
        self.kbo.compare(term1, term2) == Ordering::Greater
    }

    /// Check if one term is less than another
    pub fn less_than(&self, term1: &Expression, term2: &Expression) -> bool {
        self.kbo.compare(term1, term2) == Ordering::Less
    }

    /// Check if two terms are equal in KBO
    pub fn equal(&self, term1: &Expression, term2: &Expression) -> bool {
        self.kbo.compare(term1, term2) == Ordering::Equal
    }

    /// Check if a rule is oriented correctly (left > right)
    pub fn is_oriented(&self, left: &Expression, right: &Expression) -> bool {
        self.greater_than(left, right)
    }
}

/// Precedence graph for managing symbol precedences
#[derive(Debug, Clone)]
pub struct PrecedenceGraph {
    /// Adjacency list representation
    pub graph: HashMap<DefRef, HashSet<DefRef>>,
    /// Transitive closure
    pub closure: HashMap<DefRef, HashSet<DefRef>>,
}

impl PrecedenceGraph {
    /// Create a new precedence graph
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            closure: HashMap::new(),
        }
    }

    /// Add a precedence relation: a > b
    pub fn add_precedence(&mut self, greater: DefRef, lesser: DefRef) {
        self.graph.entry(greater.clone()).or_insert_with(HashSet::new).insert(lesser.clone());
        self.graph.entry(lesser).or_insert_with(HashSet::new);

        // Update transitive closure
        self.update_closure();
    }

    /// Check if a > b in the precedence
    pub fn precedes(&self, a: &DefRef, b: &DefRef) -> bool {
        if let Some(lesser_set) = self.closure.get(a) {
            lesser_set.contains(b)
        } else {
            false
        }
    }

    /// Update transitive closure
    fn update_closure(&mut self) {
        // Simple transitive closure computation
        for &symbol in self.graph.keys() {
            let mut closure = HashSet::new();
            let mut to_visit = vec![symbol.clone()];

            while let Some(current) = to_visit.pop() {
                if let Some(neighbors) = self.graph.get(&current) {
                    for neighbor in neighbors {
                        if !closure.contains(neighbor) {
                            closure.insert(neighbor.clone());
                            to_visit.push(neighbor.clone());
                        }
                    }
                }
            }

            self.closure.insert(symbol, closure);
        }
    }
}

/// Weight assignment for KBO
#[derive(Debug, Clone)]
pub struct WeightAssignment {
    /// Symbol weights
    pub symbol_weights: HashMap<DefRef, i32>,
    /// Variable weight
    pub variable_weight: i32,
}

impl WeightAssignment {
    /// Create a new weight assignment
    pub fn new(variable_weight: i32) -> Self {
        Self {
            symbol_weights: HashMap::new(),
            variable_weight,
        }
    }

    /// Set weight for a symbol
    pub fn set_symbol_weight(&mut self, symbol: DefRef, weight: i32) {
        self.symbol_weights.insert(symbol, weight);
    }

    /// Get weight for a symbol
    pub fn get_symbol_weight(&self, symbol: &DefRef) -> i32 {
        self.symbol_weights.get(symbol).copied().unwrap_or(1)
    }

    /// Compute total weight of an expression
    pub fn compute_weight(&self, expr: &Expression) -> i32 {
        match expr {
            Expression::Variable(_) => self.variable_weight,
            Expression::Literal(_) => 1,
            Expression::Function { function, arguments: _ } => {
                self.get_symbol_weight(function)
            },
        }
    }
}

/// KBO satisfiability checker
pub struct KBOSatisfiabilityChecker {
    /// The precedence graph
    pub precedence_graph: PrecedenceGraph,
    /// Weight assignment
    pub weight_assignment: WeightAssignment,
    /// Status assignments
    pub status_assignments: HashMap<DefRef, SymbolStatus>,
}

impl KBOSatisfiabilityChecker {
    /// Create a new satisfiability checker
    pub fn new() -> Self {
        Self {
            precedence_graph: PrecedenceGraph::new(),
            weight_assignment: WeightAssignment::new(1),
            status_assignments: HashMap::new(),
        }
    }

    /// Check if the current KBO setting is satisfiable
    pub fn is_satisfiable(&self) -> bool {
        // Check for cycles in precedence graph
        for (symbol, lesser_symbols) in &self.precedence_graph.closure {
            if lesser_symbols.contains(symbol) {
                return false; // Cycle detected
            }
        }
        true
    }

    /// Check if a rule is compatible with KBO
    pub fn is_rule_compatible(&self, left: &Expression, right: &Expression) -> bool {
        // Check if left > right in KBO
        if let Ok(kbo) = self.build_kbo() {
            kbo.greater_than(left, right)
        } else {
            false
        }
    }

    /// Build KBO from current settings
    fn build_kbo(&self) -> Result<KBO, String> {
        if !self.is_satisfiable() {
            return Err("KBO is not satisfiable".to_string());
        }

        let mut kbo = KBO::new();

        // Copy precedence
        for (greater, lesser_symbols) in &self.precedence_graph.closure {
            if let Some(lesser_set) = lesser_symbols.get(greater) {
                // This should not happen if satisfiable
                continue;
            }

            // Assign precedence based on topological order
            let precedence = self.precedence_graph.closure.keys().position(|s| s == greater)
                .unwrap_or(0) as i32;
            kbo.set_precedence(greater.clone(), precedence);
        }

        // Copy weights
        for (symbol, weight) in &self.weight_assignment.symbol_weights {
            kbo.set_weight(symbol.clone(), *weight);
        }

        // Copy status
        for (symbol, status) in &self.status_assignments {
            kbo.set_status(symbol.clone(), status.clone());
        }

        Ok(kbo)
    }
}

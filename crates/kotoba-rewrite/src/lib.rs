//! kotoba-rewrite - Kotoba Rewrite Components

pub mod rewrite;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::rewrite::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_core::{types::*, ir::*};
    use kotoba_graph::graph::GraphRef;

    #[test]
    fn test_rewrite_engine_creation() {
        // Test that RewriteEngine can be created
        let engine = RewriteEngine::new();
        // Just check that it can be created
        assert!(true);
    }

    #[test]
    fn test_rule_matcher_creation() {
        // Test that RuleMatcher can be created
        let matcher = RuleMatcher::new();
        assert!(true);
    }

    #[test]
    fn test_rule_applier_creation() {
        // Test that RuleApplier can be created
        let applier = RuleApplier::new();
        assert!(true);
    }

    #[test]
    fn test_rule_ir_creation() {
        // Test creating a basic rule
        let rule = RuleIR {
            name: "test_rule".to_string(),
            lhs: Pattern::empty(),
            rhs: Pattern::empty(),
            conditions: vec![],
        };
        assert_eq!(rule.name, "test_rule");
        assert!(rule.lhs.nodes.is_empty());
        assert!(rule.rhs.nodes.is_empty());
    }

    #[test]
    fn test_strategy_ir_creation() {
        // Test creating a basic strategy
        let strategy = StrategyIR {
            name: "test_strategy".to_string(),
            strategy: StrategyOp::Once {
                rule: "test_rule".to_string(),
            },
        };
        assert_eq!(strategy.name, "test_strategy");

        if let StrategyOp::Once { rule } = &strategy.strategy {
            assert_eq!(rule, "test_rule");
        } else {
            panic!("Expected Once strategy");
        }
    }

    #[test]
    fn test_pattern_creation() {
        // Test pattern creation
        let mut pattern = Pattern::empty();
        assert!(pattern.nodes.is_empty());
        assert!(pattern.edges.is_empty());

        // Add a node to pattern
        let node_id = pattern.add_node("Person".to_string(), HashMap::new());
        assert_eq!(pattern.nodes.len(), 1);
        assert!(pattern.nodes.contains_key(&node_id));
    }

    #[test]
    fn test_patch_creation() {
        // Test patch creation
        let patch = Patch::empty();
        assert!(patch.add_nodes.is_empty());
        assert!(patch.del_nodes.is_empty());
        assert!(patch.add_edges.is_empty());
        assert!(patch.del_edges.is_empty());
    }

    #[test]
    fn test_match_creation() {
        // Test match creation
        let mut mapping = HashMap::new();
        mapping.insert("x".to_string(), VertexId::new_v4());

        let match_result = Match { mapping };
        assert_eq!(match_result.mapping.len(), 1);
        assert!(match_result.mapping.contains_key("x"));
    }

    #[test]
    fn test_catalog_creation() {
        // Test catalog creation
        let catalog = Catalog::empty();
        assert!(catalog.schemas.is_empty());
    }

    #[test]
    fn test_rewrite_engine_with_empty_graph() {
        // Test rewrite engine with empty graph
        let engine = RewriteEngine::new();
        let graph = GraphRef::new(Graph::empty());
        let rule = RuleIR {
            name: "empty_rule".to_string(),
            lhs: Pattern::empty(),
            rhs: Pattern::empty(),
            conditions: vec![],
        };
        let catalog = Catalog::empty();

        // Test matching (should work with empty rule)
        let matches = engine.match_rule(&graph, &rule, &catalog);
        assert!(matches.is_ok());
    }

    #[test]
    fn test_rewrite_engine_with_strategy() {
        // Test rewrite with strategy
        let engine = RewriteEngine::new();
        let graph = GraphRef::new(Graph::empty());
        let rule = RuleIR {
            name: "test_rule".to_string(),
            lhs: Pattern::empty(),
            rhs: Pattern::empty(),
            conditions: vec![],
        };
        let strategy = StrategyIR {
            name: "once_strategy".to_string(),
            strategy: StrategyOp::Once {
                rule: "test_rule".to_string(),
            },
        };

        // Test rewrite application
        let result = engine.rewrite(&graph, &rule, &strategy);
        assert!(result.is_ok());
    }
}

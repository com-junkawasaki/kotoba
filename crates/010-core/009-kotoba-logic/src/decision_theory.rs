//! Decision Theory - Decision procedures and algorithms

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{LogicError, LogicResult, PredicateFormula};

/// Decision theory system for automated decision making
#[derive(Debug)]
pub struct DecisionTheory {
    /// Decision procedures
    procedures: HashMap<String, DecisionProcedure>,
    /// Heuristics
    heuristics: Vec<DecisionHeuristic>,
    /// Optimization strategies
    strategies: Vec<OptimizationStrategy>,
}

impl DecisionTheory {
    /// Create a new decision theory system
    pub fn new() -> Self {
        Self {
            procedures: HashMap::new(),
            heuristics: Vec::new(),
            strategies: Vec::new(),
        }
    }

    /// Add a decision procedure
    pub fn add_procedure(&mut self, name: String, procedure: DecisionProcedure) -> Result<(), LogicError> {
        if self.procedures.contains_key(&name) {
            return Err(LogicError::Reasoning(format!("Decision procedure {} already exists", name)));
        }
        self.procedures.insert(name, procedure);
        Ok(())
    }

    /// Add a heuristic
    pub fn add_heuristic(&mut self, heuristic: DecisionHeuristic) {
        self.heuristics.push(heuristic);
    }

    /// Add an optimization strategy
    pub fn add_strategy(&mut self, strategy: OptimizationStrategy) {
        self.strategies.push(strategy);
    }

    /// Make a decision based on available information
    pub fn decide(&self, problem: &DecisionProblem) -> Result<Decision, LogicError> {
        // Try different decision procedures
        for procedure_name in &problem.preferred_procedures {
            if let Some(procedure) = self.procedures.get(procedure_name) {
                if let Ok(decision) = procedure.decide(problem) {
                    return Ok(decision);
                }
            }
        }

        // Try heuristics if no procedure worked
        for heuristic in &self.heuristics {
            if let Ok(decision) = heuristic.decide(problem) {
                return Ok(decision);
            }
        }

        Err(LogicError::Reasoning("No decision procedure or heuristic could solve the problem".to_string()))
    }

    /// Optimize a decision using available strategies
    pub fn optimize(&self, decision: &Decision) -> Result<Decision, LogicError> {
        let mut best_decision = decision.clone();
        let mut best_score = f64::NEG_INFINITY;

        for strategy in &self.strategies {
            if let Ok(optimized) = strategy.optimize(decision) {
                let score = strategy.evaluate(&optimized);
                if score > best_score {
                    best_decision = optimized;
                    best_score = score;
                }
            }
        }

        Ok(best_decision)
    }

    /// Check if a problem is decidable
    pub fn is_decidable(&self, problem: &DecisionProblem) -> bool {
        // Check if any procedure can handle this problem
        for procedure in self.procedures.values() {
            if procedure.can_handle(problem) {
                return true;
            }
        }
        false
    }

    /// Get available procedures
    pub fn procedures(&self) -> &HashMap<String, DecisionProcedure> {
        &self.procedures
    }

    /// Get heuristics
    pub fn heuristics(&self) -> &[DecisionHeuristic] {
        &self.heuristics
    }

    /// Get optimization strategies
    pub fn strategies(&self) -> &[OptimizationStrategy] {
        &self.strategies
    }
}

impl Default for DecisionTheory {
    fn default() -> Self {
        Self::new()
    }
}

/// Decision problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionProblem {
    /// Problem name
    pub name: String,
    /// Problem description
    pub description: String,
    /// Initial state
    pub initial_state: ProblemState,
    /// Goal state
    pub goal_state: ProblemState,
    /// Available actions
    pub actions: Vec<Action>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Preferred decision procedures
    pub preferred_procedures: Vec<String>,
}

impl DecisionProblem {
    /// Create a new decision problem
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            initial_state: ProblemState::new(),
            goal_state: ProblemState::new(),
            actions: Vec::new(),
            constraints: Vec::new(),
            preferred_procedures: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set initial state
    pub fn with_initial_state(mut self, state: ProblemState) -> Self {
        self.initial_state = state;
        self
    }

    /// Set goal state
    pub fn with_goal_state(mut self, state: ProblemState) -> Self {
        self.goal_state = state;
        self
    }

    /// Add an action
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Add a constraint
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add preferred procedure
    pub fn with_preferred_procedure(mut self, procedure: String) -> Self {
        self.preferred_procedures.push(procedure);
        self
    }
}

/// Problem state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemState {
    /// State variables
    pub variables: HashMap<String, String>,
    /// State description
    pub description: String,
}

impl ProblemState {
    /// Create a new problem state
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            description: String::new(),
        }
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// Action that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Action name
    pub name: String,
    /// Action description
    pub description: String,
    /// Preconditions
    pub preconditions: Vec<Condition>,
    /// Effects
    pub effects: Vec<Effect>,
    /// Cost
    pub cost: f64,
}

impl Action {
    /// Create a new action
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            preconditions: Vec::new(),
            effects: Vec::new(),
            cost: 1.0,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Add precondition
    pub fn with_precondition(mut self, precondition: Condition) -> Self {
        self.preconditions.push(precondition);
        self
    }

    /// Add effect
    pub fn with_effect(mut self, effect: Effect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }
}

/// Condition for action preconditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Condition formula
    pub formula: PredicateFormula,
    /// Condition description
    pub description: String,
}

impl Condition {
    /// Create a new condition
    pub fn new(formula: PredicateFormula) -> Self {
        Self {
            formula,
            description: String::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// Effect of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Effect formula
    pub formula: PredicateFormula,
    /// Effect description
    pub description: String,
}

impl Effect {
    /// Create a new effect
    pub fn new(formula: PredicateFormula) -> Self {
        Self {
            formula,
            description: String::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// Constraint on the decision problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint formula
    pub formula: PredicateFormula,
    /// Constraint description
    pub description: String,
}

impl Constraint {
    /// Create a new constraint
    pub fn new(formula: PredicateFormula) -> Self {
        Self {
            formula,
            description: String::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// Decision made by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Decision name
    pub name: String,
    /// Decision description
    pub description: String,
    /// Actions to take
    pub actions: Vec<String>,
    /// Expected outcome
    pub expected_outcome: String,
    /// Confidence level
    pub confidence: f64,
    /// Reasoning used
    pub reasoning: String,
}

impl Decision {
    /// Create a new decision
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            actions: Vec::new(),
            expected_outcome: String::new(),
            confidence: 0.5,
            reasoning: String::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Add action
    pub fn with_action(mut self, action: String) -> Self {
        self.actions.push(action);
        self
    }

    /// Set expected outcome
    pub fn with_expected_outcome(mut self, outcome: String) -> Self {
        self.expected_outcome = outcome;
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set reasoning
    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = reasoning;
        self
    }
}

/// Decision procedure
#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionProcedure {
    /// Procedure name
    pub name: String,
    /// Procedure description
    pub description: String,
    /// Implementation
    pub implementation: ProcedureImplementation,
}

impl DecisionProcedure {
    /// Create a new decision procedure
    pub fn new(name: String, implementation: ProcedureImplementation) -> Self {
        Self {
            name,
            description: String::new(),
            implementation,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Make a decision
    pub fn decide(&self, problem: &DecisionProblem) -> Result<Decision, LogicError> {
        match &self.implementation {
            ProcedureImplementation::Algorithm(algorithm) => algorithm.decide(problem),
            ProcedureImplementation::Custom(code) => {
                // Execute custom code
                Err(LogicError::Reasoning("Custom procedure not implemented".to_string()))
            }
        }
    }

    /// Check if this procedure can handle the problem
    pub fn can_handle(&self, _problem: &DecisionProblem) -> bool {
        // Check if procedure can handle the problem type
        // This would be expanded with actual capability checking
        true
    }
}

/// Procedure implementation
#[derive(Debug, Serialize, Deserialize)]
pub enum ProcedureImplementation {
    /// Algorithm-based implementation
    Algorithm(Box<dyn DecisionAlgorithm>),
    /// Custom implementation
    Custom(String),
}

/// Decision algorithm trait
pub trait DecisionAlgorithm: Send + Sync + ::std::fmt::Debug {
    /// Make a decision
    fn decide(&self, problem: &DecisionProblem) -> Result<Decision, LogicError>;

    /// Get algorithm name
    fn name(&self) -> &str;

    /// Get algorithm description
    fn description(&self) -> &str;
}

/// Decision heuristic
#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionHeuristic {
    /// Heuristic name
    pub name: String,
    /// Heuristic description
    pub description: String,
    /// Implementation
    pub implementation: HeuristicImplementation,
}

impl DecisionHeuristic {
    /// Create a new decision heuristic
    pub fn new(name: String, implementation: HeuristicImplementation) -> Self {
        Self {
            name,
            description: String::new(),
            implementation,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Make a decision using this heuristic
    pub fn decide(&self, problem: &DecisionProblem) -> Result<Decision, LogicError> {
        match &self.implementation {
            HeuristicImplementation::Function(function) => function.decide(problem),
            HeuristicImplementation::Custom(code) => {
                Err(LogicError::Reasoning("Custom heuristic not implemented".to_string()))
            }
        }
    }
}

/// Heuristic implementation
#[derive(Debug, Serialize, Deserialize)]
pub enum HeuristicImplementation {
    /// Function-based implementation
    Function(Box<dyn DecisionFunction>),
    /// Custom implementation
    Custom(String),
}

/// Decision function trait
pub trait DecisionFunction: Send + Sync + ::std::fmt::Debug {
    /// Make a decision
    fn decide(&self, problem: &DecisionProblem) -> Result<Decision, LogicError>;

    /// Get function name
    fn name(&self) -> &str;

    /// Get function description
    fn description(&self) -> &str;
}

/// Optimization strategy
#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    /// Strategy name
    pub name: String,
    /// Strategy description
    pub description: String,
    /// Implementation
    pub implementation: StrategyImplementation,
}

impl OptimizationStrategy {
    /// Create a new optimization strategy
    pub fn new(name: String, implementation: StrategyImplementation) -> Self {
        Self {
            name,
            description: String::new(),
            implementation,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Optimize a decision
    pub fn optimize(&self, decision: &Decision) -> Result<Decision, LogicError> {
        match &self.implementation {
            StrategyImplementation::Optimizer(optimizer) => optimizer.optimize(decision),
            StrategyImplementation::Custom(code) => {
                Err(LogicError::Reasoning("Custom strategy not implemented".to_string()))
            }
        }
    }

    /// Evaluate a decision
    pub fn evaluate(&self, decision: &Decision) -> f64 {
        match &self.implementation {
            StrategyImplementation::Optimizer(optimizer) => optimizer.evaluate(decision),
            StrategyImplementation::Custom(_) => 0.0,
        }
    }
}

/// Strategy implementation
#[derive(Debug, Serialize, Deserialize)]
pub enum StrategyImplementation {
    /// Optimizer-based implementation
    Optimizer(Box<dyn DecisionOptimizer>),
    /// Custom implementation
    Custom(String),
}

/// Decision optimizer trait
pub trait DecisionOptimizer: Send + Sync + ::std::fmt::Debug {
    /// Optimize a decision
    fn optimize(&self, decision: &Decision) -> Result<Decision, LogicError>;

    /// Evaluate a decision
    fn evaluate(&self, decision: &Decision) -> f64;

    /// Get optimizer name
    fn name(&self) -> &str;

    /// Get optimizer description
    fn description(&self) -> &str;
}

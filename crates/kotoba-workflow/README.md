# Kotoba Workflow Engine (Itonami)

Temporal-inspired workflow engine built on top of Kotoba's graph rewriting system.

## Overview

Itonami provides a powerful workflow execution engine that combines:

- **Temporal Patterns**: Sequence, Parallel, Decision, Wait, Saga, Activity, Sub-workflow
- **Graph-based Execution**: Declarative workflow definition using graph transformations
- **MVCC Persistence**: Workflow state management with Merkle DAG
- **Activity System**: Extensible activity execution framework
- **Event Sourcing**: Complete audit trail of workflow execution

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Workflow Engine Layer                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Workflow Definition (.kotoba)                          │ │
│  │  - WorkflowIR: Temporalパターンの宣言的定義             │ │
│  │  - StrategyIR: 拡張（Parallel, Wait, Compensation）     │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Execution Engine                                       │ │
│  │  - WorkflowExecutor: ワークフロー実行器                 │ │
│  │  - ActivityExecutor: Activity実行器                     │ │
│  │  - StateManager: MVCCベース状態管理                     │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Persistence Layer                                      │ │
│  │  - WorkflowStore: ワークフロー永続化                    │ │
│  │  - EventStore: イベント/メッセージ永続化                │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
    │
    ▼ (extends)
┌─────────────────────────────────────────────────────────────┐
│                  Kotoba Core Engine                         │
│  - Graph Store (MVCC + Merkle)                             │
│  - Rule Engine (DPO)                                       │
│  - Query Engine (GQL)                                      │
│  - Distributed Execution                                   │
└─────────────────────────────────────────────────────────────┘
```

## Features

### Workflow Patterns

- **Sequence**: Execute activities in order
- **Parallel**: Execute activities concurrently
- **Decision**: Conditional branching based on data
- **Wait**: Wait for events, timers, or signals
- **Saga**: Long-running transactions with compensation
- **Activity**: Execute external tasks (HTTP, DB, functions)
- **Sub-workflow**: Call other workflows

### Persistence

- **MVCC-based State**: Immutable workflow state with versioning
- **Merkle DAG**: Content-addressable state snapshots
- **Event Sourcing**: Complete audit trail of execution history
- **Snapshots**: Performance optimization for long-running workflows

### Activity System

- **Extensible**: Easy to add new activity types
- **Timeout Support**: Configurable timeouts per activity
- **Retry Policies**: Exponential backoff and custom retry logic
- **Built-in Activities**: HTTP, Database, Function calls

## Phase 2 Features

### MVCC-based State Management

Workflow executions now use Multi-Version Concurrency Control (MVCC) for:
- **Versioned State**: Each state change creates a new version with TxId
- **Point-in-Time Queries**: Query workflow state at any transaction point
- **Concurrent Access**: Multiple readers can access different versions simultaneously
- **Conflict Resolution**: Optimistic concurrency control for state updates

```rust
// Get workflow state at specific transaction
let execution_at_tx = engine.get_execution_at_tx(&execution_id, tx_id).await;

// Get complete version history
let history = engine.get_execution_history(&execution_id).await;
```

### Event Sourcing

Complete audit trail with event sourcing:
- **Immutable Events**: All state changes recorded as events
- **Event Replay**: Rebuild workflow state from event history
- **Event Types**: Started, ActivityScheduled, Completed, Failed, etc.
- **Performance Optimization**: Automatic snapshot creation

```rust
// Get full event history
let events = engine.get_event_history(&execution_id).await?;

// Rebuild execution from events (for recovery)
let execution = engine.rebuild_execution_from_events(&execution_id).await?;
```

### Distributed Execution

Cluster-wide workflow distribution:
- **Load Balancing**: Round-robin and least-loaded strategies
- **Node Management**: Automatic node discovery and health monitoring
- **Failover**: Automatic task reassignment on node failure
- **Cluster Health**: Real-time monitoring of cluster status

```rust
// Enable distributed execution
engine.enable_distributed_execution(
    "node-1".to_string(),
    Arc::new(LeastLoadedBalancer::new())
);

// Submit workflow for distributed execution
let task_id = engine.submit_distributed_workflow(execution_id).await?;

// Check cluster health
let health = engine.get_cluster_health().await?;
```

### Snapshot Optimization

Performance optimization for long-running workflows:
- **Automatic Snapshots**: Periodic state snapshots to reduce replay time
- **Configurable Intervals**: Customize snapshot frequency
- **Fast Recovery**: Restore from snapshots + recent events
- **Storage Efficiency**: Automatic cleanup of old snapshots

## Usage

### Basic Example

```rust
use kotoba_workflow::{WorkflowEngine, WorkflowIR};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create workflow engine
    let engine = WorkflowEngine::builder()
        .with_storage("memory")
        .build()
        .await?;

    // Load workflow definition
    let workflow_ir = WorkflowIR::from_jsonnet("workflow.kotoba")?;

    // Start workflow execution
    let execution_id = engine.start_workflow(&workflow_ir, inputs).await?;

    // Wait for completion
    let result = engine.wait_for_completion(execution_id).await?;

    println!("Workflow completed with result: {:?}", result);
    Ok(())
}
```

### Workflow Definition (.kotoba)

```jsonnet
{
  workflow: {
    id: "order_processing",
    name: "Order Processing Workflow",
    version: "1.0.0",

    inputs: [
      { name: "orderId", type: "string", required: true },
      { name: "customerId", type: "string", required: true },
      { name: "amount", type: "number", required: true },
    ],

    outputs: [
      { name: "processed", type: "boolean" },
      { name: "confirmationId", type: "string" },
    ],

    strategy: {
      op: "saga",
      main_flow: {
        op: "seq",
        strategies: [
          {
            op: "activity",
            activity_ref: "validate_order",
            input_mapping: {
              order_id: "$.inputs.orderId",
              customer_id: "$.inputs.customerId",
            },
          },
          {
            op: "parallel",
            branches: [
              {
                op: "activity",
                activity_ref: "process_payment",
                input_mapping: { amount: "$.inputs.amount" },
              },
              {
                op: "activity",
                activity_ref: "reserve_inventory",
                input_mapping: { order_id: "$.inputs.orderId" },
              },
            ],
          },
          {
            op: "activity",
            activity_ref: "send_confirmation",
          },
        ],
      },
      compensation: {
        op: "seq",
        strategies: [
          { op: "activity", activity_ref: "cancel_payment" },
          { op: "activity", activity_ref: "release_inventory" },
          { op: "activity", activity_ref: "send_failure_notification" },
        ],
      },
    },

    timeout: "PT30M",
  },
}
```

### Activity Implementation

```rust
use kotoba_workflow::activity::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ActivityRegistry::new();

    // Register HTTP activity
    let http_activity = ActivityBuilder::new("validate_order")
        .http("https://api.example.com/validate", "POST")
        .build();
    registry.register(http_activity).await;

    // Register custom function activity
    let db_activity = DatabaseActivity::new("reserve_inventory",
        "UPDATE inventory SET reserved = true WHERE item_id = $1");
    registry.register(Arc::new(db_activity)).await;

    // Register function activity
    let send_email = FunctionActivity::new("send_confirmation", |inputs| {
        let order_id = inputs.get("order_id").unwrap().as_str().unwrap();
        // Send confirmation email logic
        let mut outputs = HashMap::new();
        outputs.insert("confirmation_id".to_string(), json!("CONF-123"));
        Ok(outputs)
    });
    registry.register(Arc::new(send_email)).await;

    Ok(())
}
```

## Comparison with Temporal

| Aspect | Temporal | Itonami |
|--------|----------|---------|
| **Execution Model** | Strict workflow control | Graph-based declarative execution |
| **Persistence** | Event sourcing + snapshots | MVCC + Merkle DAG |
| **Language** | Go | Rust (with .kotoba DSL) |
| **Activity Types** | SDK-based | Extensible trait system |
| **Deployment** | Dedicated server | Embedded in Kotoba |
| **Query Language** | SQL-like | GQL integration |
| **State Management** | Temporal server | Kotoba graph store |

## Roadmap

### Phase 1: Core Implementation ✅
- [x] WorkflowIR definition
- [x] StrategyIR extensions (Temporal patterns)
- [x] Activity system
- [x] Basic execution engine

### Phase 2: Persistence & Distribution ✅
- [x] MVCC-based state management
- [x] Event sourcing implementation
- [x] Distributed execution support
- [x] Snapshot optimization

### Phase 3: Advanced Features 📋
- [ ] Saga pattern full implementation
- [ ] Monitoring and observability
- [ ] Workflow optimization
- [ ] External system integrations

### Phase 4: Ecosystem 🌟
- [ ] Workflow designer UI
- [ ] Pre-built activity libraries
- [ ] Kubernetes operator
- [ ] Cloud-native integrations

## Contributing

Contributions are welcome! Please see the main Kotoba repository for contribution guidelines.

## License

Licensed under MIT OR Apache-2.0

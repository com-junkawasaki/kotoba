// These types are based on the Rust structs in `kotoba-workflow/src/ir.rs`.
// For a robust solution, these could be generated from the Rust source
// using a tool like `ts-rs`.

export type ExecutionStatus = 
  | 'Running' 
  | 'Completed' 
  | 'Failed' 
  | 'Cancelled' 
  | 'TimedOut' 
  | 'Compensating';

export interface WorkflowExecution {
  id: { 0: string };
  workflow_id: string;
  status: ExecutionStatus;
  start_time: string; // ISO 8601 date string
  end_time?: string; // ISO 8601 date string
  inputs: Record<string, any>;
  outputs?: Record<string, any>;
  current_graph: any; // Simplified for now
  execution_history: any[]; // Simplified for now
  retry_count: number;
  timeout_at?: string; // ISO 8601 date string
}

// This is a simplified version of WorkflowIR for the client.
// A complete definition would be much more complex and should match the Rust struct.
export interface WorkflowIR {
  id: string;
  name: string;
  version: string;
  description?: string;
  inputs: any[]; // Simplified
  outputs: any[]; // Simplified
  strategy: any; // The strategy op structure
  timeout?: string; // Duration string e.g., "10s"
  retry_policy?: any; // Simplified
  metadata: Record<string, any>;
}

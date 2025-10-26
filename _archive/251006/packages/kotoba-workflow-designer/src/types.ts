// Type definitions for Kotoba Workflow Designer

import { Node, Edge, Connection, EdgeChange, NodeChange } from 'react-flow-renderer';

export interface WorkflowNode extends Node {
  type: 'activity' | 'decision' | 'parallel' | 'saga' | 'start' | 'end';
  data: WorkflowNodeData;
}

export interface WorkflowNodeData {
  label: string;
  activityType?: ActivityType;
  config?: ActivityConfig;
  metadata?: Record<string, any>;
}

export interface WorkflowEdge extends Edge {
  type: 'default' | 'conditional' | 'compensation';
  data?: {
    condition?: string;
    compensationRef?: string;
  };
}

export interface ActivityType {
  id: string;
  name: string;
  category: ActivityCategory;
  icon: string;
  description: string;
  inputs: ActivityParam[];
  outputs: ActivityParam[];
  configSchema: Record<string, any>;
}

export interface ActivityParam {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'object' | 'array';
  required: boolean;
  defaultValue?: any;
  description?: string;
}

export interface ActivityConfig {
  timeout?: number;
  retryPolicy?: RetryPolicy;
  inputMapping?: Record<string, string>;
  outputMapping?: Record<string, string>;
  metadata?: Record<string, any>;
}

export interface RetryPolicy {
  maxAttempts: number;
  initialInterval: number;
  backoffCoefficient: number;
  maximumInterval?: number;
  nonRetryableErrors?: string[];
}

export enum ActivityCategory {
  GENERAL = 'general',
  HTTP = 'http',
  DATABASE = 'database',
  MESSAGE_QUEUE = 'message_queue',
  CLOUD_STORAGE = 'cloud_storage',
  EMAIL = 'email',
  WEBHOOK = 'webhook',
  CUSTOM = 'custom',
}

export interface WorkflowConfig {
  id: string;
  name: string;
  description?: string;
  version: string;
  inputs: ActivityParam[];
  outputs: ActivityParam[];
  timeout?: number;
  retryPolicy?: RetryPolicy;
  metadata?: Record<string, any>;
}

export interface WorkflowState {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  selectedNodeId?: string;
  selectedEdgeId?: string;
  config: WorkflowConfig;
  isDirty: boolean;
  validationErrors: ValidationError[];
}

export interface ValidationError {
  id: string;
  type: 'error' | 'warning';
  message: string;
  nodeId?: string;
  edgeId?: string;
}

export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  thumbnail?: string;
  workflow: {
    nodes: WorkflowNode[];
    edges: WorkflowEdge[];
    config: WorkflowConfig;
  };
}

export interface DesignerTheme {
  colors: {
    primary: string;
    secondary: string;
    success: string;
    warning: string;
    error: string;
    background: string;
    surface: string;
    text: string;
    textSecondary: string;
    border: string;
  };
  spacing: {
    xs: string;
    sm: string;
    md: string;
    lg: string;
    xl: string;
  };
  borderRadius: string;
  fontSize: {
    xs: string;
    sm: string;
    md: string;
    lg: string;
    xl: string;
  };
}

export interface WorkflowContextValue {
  state: WorkflowState;
  actions: {
    addNode: (node: Omit<WorkflowNode, 'id'>) => void;
    updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
    deleteNode: (nodeId: string) => void;
    addEdge: (edge: Omit<WorkflowEdge, 'id'>) => void;
    updateEdge: (edgeId: string, updates: Partial<WorkflowEdge>) => void;
    deleteEdge: (edgeId: string) => void;
    selectNode: (nodeId: string | null) => void;
    selectEdge: (edgeId: string | null) => void;
    updateConfig: (config: Partial<WorkflowConfig>) => void;
    validateWorkflow: () => void;
    saveWorkflow: () => Promise<void>;
    loadWorkflow: (workflow: any) => void;
    clearWorkflow: () => void;
  };
}

export interface ActivityLibrary {
  categories: {
    [key in ActivityCategory]: ActivityType[];
  };
  customActivities: ActivityType[];
}

export interface WorkflowDesignerSettings {
  snapToGrid: boolean;
  showGrid: boolean;
  showMiniMap: boolean;
  showControls: boolean;
  autoLayout: boolean;
  theme: 'light' | 'dark' | 'auto';
  zoom: number;
  pan: { x: number; y: number };
}

export interface DragItem {
  type: 'activity';
  activityType: ActivityType;
}

export interface WorkflowValidationResult {
  isValid: boolean;
  errors: ValidationError[];
  warnings: ValidationError[];
}

export interface WorkflowExecutionPreview {
  nodeId: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  startTime?: Date;
  endTime?: Date;
  error?: string;
  output?: any;
}

export interface WorkflowSimulationState {
  isRunning: boolean;
  currentNodeId?: string;
  executionHistory: WorkflowExecutionPreview[];
  variables: Record<string, any>;
}

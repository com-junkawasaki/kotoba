import React, { useCallback, useMemo } from 'react';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  ReactFlowProvider,
  Panel,
} from 'react-flow-renderer';
import styled from 'styled-components';
import { useWorkflow } from '../context/WorkflowContext';
import { useTheme } from '../context/ThemeContext';
import { WorkflowNode, WorkflowEdge } from '../types';

// Import custom node types
import { ActivityNode } from './nodes/ActivityNode';
import { DecisionNode } from './nodes/DecisionNode';
import { ParallelNode } from './nodes/ParallelNode';
import { SagaNode } from './nodes/SagaNode';
import { StartNode } from './nodes/StartNode';
import { EndNode } from './nodes/EndNode';

const CanvasContainer = styled.div<{ theme: any }>`
  width: 100%;
  height: 100%;
  position: relative;

  .react-flow__node {
    border-radius: ${props => props.theme.borderRadius};
    border: 2px solid ${props => props.theme.colors.border};
    background-color: ${props => props.theme.colors.surface};
    color: ${props => props.theme.colors.text};

    &.selected {
      border-color: ${props => props.theme.colors.primary};
      box-shadow: 0 0 0 2px ${props => props.theme.colors.primary}33;
    }
  }

  .react-flow__edge {
    &.selected {
      .react-flow__edge-path {
        stroke: ${props => props.theme.colors.primary};
        stroke-width: 3;
      }
    }
  }

  .react-flow__edge-path {
    stroke: ${props => props.theme.colors.border};
    stroke-width: 2;
  }

  .react-flow__controls {
    background-color: ${props => props.theme.colors.surface};
    border: 1px solid ${props => props.theme.colors.border};
    border-radius: ${props => props.theme.borderRadius};

    button {
      background-color: transparent;
      border: none;
      color: ${props => props.theme.colors.text};
      border-radius: 4px;

      &:hover {
        background-color: ${props => props.theme.colors.primary};
        color: white;
      }
    }
  }

  .react-flow__minimap {
    background-color: ${props => props.theme.colors.surface};
    border: 1px solid ${props => props.theme.colors.border};
  }

  .react-flow__background {
    background-color: ${props => props.theme.colors.background};
  }
`;

const CanvasToolbar = styled.div<{ theme: any }>`
  position: absolute;
  top: ${props => props.theme.spacing.md};
  right: ${props => props.theme.spacing.md};
  z-index: 5;
  display: flex;
  gap: ${props => props.theme.spacing.sm};
`;

const ToolbarButton = styled.button<{ theme: any }>`
  background-color: ${props => props.theme.colors.surface};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  color: ${props => props.theme.colors.text};
  cursor: pointer;
  font-size: ${props => props.theme.fontSize.sm};
  transition: all 0.2s ease;

  &:hover {
    background-color: ${props => props.theme.colors.primary};
    color: white;
    border-color: ${props => props.theme.colors.primary};
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;

const ValidationPanel = styled.div<{ theme: any }>`
  position: absolute;
  bottom: ${props => props.theme.spacing.md};
  left: ${props => props.theme.spacing.md};
  right: ${props => props.theme.spacing.md};
  background-color: ${props => props.theme.colors.error};
  color: white;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border-radius: ${props => props.theme.borderRadius};
  font-size: ${props => props.theme.fontSize.sm};
  z-index: 5;
`;

export interface WorkflowCanvasProps {
  readOnly?: boolean;
  showMiniMap?: boolean;
  showControls?: boolean;
  showBackground?: boolean;
  snapToGrid?: boolean;
  className?: string;
}

export const WorkflowCanvas: React.FC<WorkflowCanvasProps> = ({
  readOnly = false,
  showMiniMap = true,
  showControls = true,
  showBackground = true,
  snapToGrid = true,
  className,
}) => {
  const { state, actions } = useWorkflow();
  const { theme } = useTheme();

  const [nodes, setNodes, onNodesChange] = useNodesState(state.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(state.edges);

  // Define custom node types
  const nodeTypes = useMemo(() => ({
    activity: ActivityNode,
    decision: DecisionNode,
    parallel: ParallelNode,
    saga: SagaNode,
    start: StartNode,
    end: EndNode,
  }), []);

  // Handle connections between nodes
  const onConnect = useCallback(
    (params: Connection) => {
      if (readOnly) return;

      const newEdge: WorkflowEdge = {
        id: `edge_${params.source}_${params.target}`,
        source: params.source!,
        target: params.target!,
        type: 'default',
        ...params,
      };

      setEdges((eds) => addEdge(newEdge, eds));
      actions.addEdge(newEdge);
    },
    [readOnly, setEdges, actions]
  );

  // Handle node selection
  const onSelectionChange = useCallback(
    (elements: { nodes: Node[]; edges: Edge[] }) => {
      if (elements.nodes.length === 1) {
        actions.selectNode(elements.nodes[0].id);
      } else if (elements.edges.length === 1) {
        actions.selectEdge(elements.edges[0].id);
      } else {
        actions.selectNode(null);
        actions.selectEdge(null);
      }
    },
    [actions]
  );

  // Handle node drag end
  const onNodeDragStop = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      if (readOnly) return;
      actions.updateNode(node.id, { position: node.position });
    },
    [readOnly, actions]
  );

  // Auto-layout function
  const handleAutoLayout = useCallback(() => {
    // Simple auto-layout algorithm
    const layoutNodes = nodes.map((node, index) => ({
      ...node,
      position: {
        x: (index % 3) * 200 + 100,
        y: Math.floor(index / 3) * 150 + 100,
      },
    }));
    setNodes(layoutNodes);
  }, [nodes, setNodes]);

  // Validation function
  const handleValidate = useCallback(() => {
    actions.validateWorkflow();
  }, [actions]);

  // Clear canvas function
  const handleClear = useCallback(() => {
    if (window.confirm('Are you sure you want to clear the workflow? This action cannot be undone.')) {
      actions.clearWorkflow();
      setNodes([]);
      setEdges([]);
    }
  }, [actions, setNodes, setEdges]);

  return (
    <CanvasContainer theme={theme} className={className}>
      <ReactFlowProvider>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={readOnly ? undefined : onNodesChange}
          onEdgesChange={readOnly ? undefined : onEdgesChange}
          onConnect={onConnect}
          onSelectionChange={onSelectionChange}
          onNodeDragStop={onNodeDragStop}
          nodeTypes={nodeTypes}
          snapToGrid={snapToGrid}
          snapGrid={[15, 15]}
          fitView
          attributionPosition="bottom-left"
        >
          {showBackground && (
            <Background
              color={theme.colors.border}
              gap={20}
              size={1}
            />
          )}

          {showControls && <Controls />}

          {showMiniMap && (
            <MiniMap
              nodeColor={theme.colors.primary}
              maskColor={theme.colors.background + '80'}
            />
          )}

          <Panel position="top-right">
            <CanvasToolbar theme={theme}>
              <ToolbarButton
                theme={theme}
                onClick={handleAutoLayout}
                disabled={readOnly}
                title="Auto Layout"
              >
                📐 Layout
              </ToolbarButton>
              <ToolbarButton
                theme={theme}
                onClick={handleValidate}
                title="Validate Workflow"
              >
                ✅ Validate
              </ToolbarButton>
              <ToolbarButton
                theme={theme}
                onClick={handleClear}
                disabled={readOnly}
                title="Clear Workflow"
              >
                🗑️ Clear
              </ToolbarButton>
            </CanvasToolbar>
          </Panel>
        </ReactFlow>
      </ReactFlowProvider>

      {/* Validation errors display */}
      {state.validationErrors.length > 0 && (
        <ValidationPanel theme={theme}>
          <strong>Validation Errors:</strong>
          <ul>
            {state.validationErrors.slice(0, 3).map((error, index) => (
              <li key={index}>{error.message}</li>
            ))}
            {state.validationErrors.length > 3 && (
              <li>... and {state.validationErrors.length - 3} more</li>
            )}
          </ul>
        </ValidationPanel>
      )}
    </CanvasContainer>
  );
};

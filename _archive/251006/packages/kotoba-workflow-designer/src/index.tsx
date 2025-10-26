import React from 'react';
import { WorkflowDesigner } from './components/WorkflowDesigner';
import { WorkflowCanvas } from './components/WorkflowCanvas';
import { ActivityPalette } from './components/ActivityPalette';
import { PropertyPanel } from './components/PropertyPanel';
import { Toolbar } from './components/Toolbar';
import { WorkflowProvider } from './context/WorkflowContext';
import { ThemeProvider } from './context/ThemeContext';

// Export main components
export { WorkflowDesigner, WorkflowCanvas, ActivityPalette, PropertyPanel, Toolbar };
export { WorkflowProvider, ThemeProvider };

// Export types
export type { WorkflowNode, WorkflowEdge, ActivityType, WorkflowConfig } from './types';

// Export utilities
export { workflowToJsonnet, jsonnetToWorkflow } from './utils/workflowSerializer';

// Main Workflow Designer component with all features
export interface WorkflowDesignerProps {
  initialWorkflow?: any;
  onWorkflowChange?: (workflow: any) => void;
  onSave?: (workflow: any) => void;
  onLoad?: () => Promise<any>;
  theme?: 'light' | 'dark' | 'auto';
  readOnly?: boolean;
  showToolbar?: boolean;
  showPropertyPanel?: boolean;
  showActivityPalette?: boolean;
}

export const KotobaWorkflowDesigner: React.FC<WorkflowDesignerProps> = ({
  initialWorkflow,
  onWorkflowChange,
  onSave,
  onLoad,
  theme = 'auto',
  readOnly = false,
  showToolbar = true,
  showPropertyPanel = true,
  showActivityPalette = true,
}) => {
  return (
    <ThemeProvider theme={theme}>
      <WorkflowProvider
        initialWorkflow={initialWorkflow}
        onWorkflowChange={onWorkflowChange}
        onSave={onSave}
        onLoad={onLoad}
      >
        <WorkflowDesigner
          readOnly={readOnly}
          showToolbar={showToolbar}
          showPropertyPanel={showPropertyPanel}
          showActivityPalette={showActivityPalette}
        />
      </WorkflowProvider>
    </ThemeProvider>
  );
};

// Default export
export default KotobaWorkflowDesigner;

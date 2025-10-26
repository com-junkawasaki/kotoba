import React from 'react';
import styled from 'styled-components';
import { useWorkflow } from '../context/WorkflowContext';
import { useTheme } from '../context/ThemeContext';

const ToolbarContainer = styled.div<{ theme: any }>`
  display: flex;
  align-items: center;
  gap: ${props => props.theme.spacing.sm};
  padding: 0 ${props => props.theme.spacing.md};
`;

const ToolbarButton = styled.button<{ theme: any; variant?: 'primary' | 'secondary' }>`
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.variant === 'primary' ? props.theme.colors.primary : props.theme.colors.surface};
  color: ${props => props.variant === 'primary' ? 'white' : props.theme.colors.text};
  font-size: ${props => props.theme.fontSize.sm};
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: ${props => props.theme.spacing.xs};

  &:hover {
    background-color: ${props => props.variant === 'primary' ? props.theme.colors.primary : props.theme.colors.background};
    border-color: ${props => props.theme.colors.primary};
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &:focus {
    outline: none;
    box-shadow: 0 0 0 2px ${props => props.theme.colors.primary}33;
  }
`;

const Separator = styled.div<{ theme: any }>`
  width: 1px;
  height: 24px;
  background-color: ${props => props.theme.colors.border};
  margin: 0 ${props => props.theme.spacing.sm};
`;

const WorkflowInfo = styled.div<{ theme: any }>`
  margin-left: auto;
  font-size: ${props => props.theme.fontSize.sm};
  color: ${props => props.theme.colors.textSecondary};
  display: flex;
  align-items: center;
  gap: ${props => props.theme.spacing.sm};
`;

const StatusIndicator = styled.div<{ status: 'saved' | 'modified' | 'error'; theme: any }>`
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: ${props => {
    switch (props.status) {
      case 'saved': return props.theme.colors.success;
      case 'modified': return props.theme.colors.warning;
      case 'error': return props.theme.colors.error;
      default: return props.theme.colors.textSecondary;
    }
  }};
`;

export interface ToolbarProps {
  readOnly?: boolean;
  onNew?: () => void;
  onOpen?: () => void;
  onImport?: () => void;
  onExport?: () => void;
}

export const Toolbar: React.FC<ToolbarProps> = ({
  readOnly = false,
  onNew,
  onOpen,
  onImport,
  onExport,
}) => {
  const { state, actions } = useWorkflow();
  const { theme } = useTheme();

  const handleNew = () => {
    if (window.confirm('Create new workflow? Any unsaved changes will be lost.')) {
      actions.clearWorkflow();
      onNew?.();
    }
  };

  const handleSave = async () => {
    try {
      await actions.saveWorkflow();
    } catch (error) {
      console.error('Failed to save workflow:', error);
    }
  };

  const handleUndo = () => {
    // TODO: Implement undo functionality
    console.log('Undo not implemented yet');
  };

  const handleRedo = () => {
    // TODO: Implement redo functionality
    console.log('Redo not implemented yet');
  };

  const handleZoomIn = () => {
    // TODO: Implement zoom functionality
    console.log('Zoom in not implemented yet');
  };

  const handleZoomOut = () => {
    // TODO: Implement zoom functionality
    console.log('Zoom out not implemented yet');
  };

  const handleFitView = () => {
    // TODO: Implement fit view functionality
    console.log('Fit view not implemented yet');
  };

  const getStatus = () => {
    if (state.validationErrors.some(e => e.type === 'error')) {
      return 'error';
    }
    if (state.isDirty) {
      return 'modified';
    }
    return 'saved';
  };

  return (
    <ToolbarContainer theme={theme}>
      {/* File Operations */}
      <ToolbarButton
        theme={theme}
        onClick={handleNew}
        disabled={readOnly}
        title="New Workflow"
      >
        📄 New
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={onOpen}
        title="Open Workflow"
      >
        📂 Open
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={handleSave}
        disabled={readOnly || !state.isDirty}
        variant={state.isDirty ? 'primary' : undefined}
        title="Save Workflow"
      >
        💾 Save
      </ToolbarButton>

      <Separator theme={theme} />

      {/* Import/Export */}
      <ToolbarButton
        theme={theme}
        onClick={onImport}
        title="Import Workflow"
      >
        📥 Import
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={onExport}
        title="Export Workflow"
      >
        📤 Export
      </ToolbarButton>

      <Separator theme={theme} />

      {/* Edit Operations */}
      <ToolbarButton
        theme={theme}
        onClick={handleUndo}
        disabled={readOnly}
        title="Undo"
      >
        ↶ Undo
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={handleRedo}
        disabled={readOnly}
        title="Redo"
      >
        ↷ Redo
      </ToolbarButton>

      <Separator theme={theme} />

      {/* View Operations */}
      <ToolbarButton
        theme={theme}
        onClick={handleZoomIn}
        title="Zoom In"
      >
        🔍+
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={handleZoomOut}
        title="Zoom Out"
      >
        🔍-
      </ToolbarButton>

      <ToolbarButton
        theme={theme}
        onClick={handleFitView}
        title="Fit to View"
      >
        📐 Fit
      </ToolbarButton>

      <Separator theme={theme} />

      {/* Validation */}
      <ToolbarButton
        theme={theme}
        onClick={() => actions.validateWorkflow()}
        title="Validate Workflow"
      >
        ✅ Validate
      </ToolbarButton>

      {/* Workflow Info */}
      <WorkflowInfo theme={theme}>
        <StatusIndicator theme={theme} status={getStatus()} />
        <span>
          {state.config.name || 'Untitled Workflow'} v{state.config.version}
          {state.isDirty && ' *'}
        </span>
        <span>•</span>
        <span>{state.nodes.length} nodes, {state.edges.length} connections</span>
        {state.validationErrors.length > 0 && (
          <>
            <span>•</span>
            <span style={{ color: theme.colors.error }}>
              {state.validationErrors.filter(e => e.type === 'error').length} errors
            </span>
          </>
        )}
      </WorkflowInfo>
    </ToolbarContainer>
  );
};

import React from 'react';
import styled from 'styled-components';
import { useWorkflow } from '../context/WorkflowContext';
import { useTheme } from '../context/ThemeContext';

const PanelContainer = styled.div<{ theme: any }>`
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: ${props => props.theme.colors.surface};
  border-left: 1px solid ${props => props.theme.colors.border};
`;

const PanelHeader = styled.div<{ theme: any }>`
  padding: ${props => props.theme.spacing.md};
  border-bottom: 1px solid ${props => props.theme.colors.border};
  background-color: ${props => props.theme.colors.background};
`;

const PanelTitle = styled.h3<{ theme: any }>`
  margin: 0;
  font-size: ${props => props.theme.fontSize.md};
  color: ${props => props.theme.colors.text};
  font-weight: 600;
`;

const PanelContent = styled.div<{ theme: any }>`
  flex: 1;
  overflow-y: auto;
  padding: ${props => props.theme.spacing.md};
`;

const PropertySection = styled.div<{ theme: any }>`
  margin-bottom: ${props => props.theme.spacing.lg};
`;

const SectionTitle = styled.h4<{ theme: any }>`
  margin: 0 0 ${props => props.theme.spacing.md} 0;
  font-size: ${props => props.theme.fontSize.sm};
  color: ${props => props.theme.colors.text};
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
`;

const PropertyGroup = styled.div<{ theme: any }>`
  margin-bottom: ${props => props.theme.spacing.md};
`;

const PropertyLabel = styled.label<{ theme: any }>`
  display: block;
  font-size: ${props => props.theme.fontSize.sm};
  color: ${props => props.theme.colors.text};
  font-weight: 500;
  margin-bottom: ${props => props.theme.spacing.xs};
`;

const PropertyInput = styled.input<{ theme: any }>`
  width: 100%;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.theme.colors.background};
  color: ${props => props.theme.colors.text};
  font-size: ${props => props.theme.fontSize.sm};

  &:focus {
    outline: none;
    border-color: ${props => props.theme.colors.primary};
    box-shadow: 0 0 0 2px ${props => props.theme.colors.primary}33;
  }
`;

const PropertySelect = styled.select<{ theme: any }>`
  width: 100%;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.theme.colors.background};
  color: ${props => props.theme.colors.text};
  font-size: ${props => props.theme.fontSize.sm};

  &:focus {
    outline: none;
    border-color: ${props => props.theme.colors.primary};
  }
`;

const PropertyTextarea = styled.textarea<{ theme: any }>`
  width: 100%;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.theme.colors.background};
  color: ${props => props.theme.colors.text};
  font-size: ${props => props.theme.fontSize.sm};
  font-family: inherit;
  resize: vertical;
  min-height: 80px;

  &:focus {
    outline: none;
    border-color: ${props => props.theme.colors.primary};
    box-shadow: 0 0 0 2px ${props => props.theme.colors.primary}33;
  }
`;

const EmptyState = styled.div<{ theme: any }>`
  text-align: center;
  color: ${props => props.theme.colors.textSecondary};
  padding: ${props => props.theme.spacing.xl};
  font-size: ${props => props.theme.fontSize.sm};
`;

export const PropertyPanel: React.FC = () => {
  const { state, actions } = useWorkflow();
  const { theme } = useTheme();

  const selectedNode = state.nodes.find(node => node.id === state.selectedNodeId);
  const selectedEdge = state.edges.find(edge => edge.id === state.selectedEdgeId);

  const handleWorkflowPropertyChange = (property: string, value: any) => {
    actions.updateConfig({ [property]: value });
  };

  const handleNodePropertyChange = (property: string, value: any) => {
    if (selectedNode) {
      const updatedData = { ...selectedNode.data, [property]: value };
      actions.updateNode(selectedNode.id, { data: updatedData });
    }
  };

  if (selectedNode) {
    return (
      <PanelContainer theme={theme}>
        <PanelHeader theme={theme}>
          <PanelTitle theme={theme}>
            {selectedNode.data?.label || selectedNode.type} Properties
          </PanelTitle>
        </PanelHeader>

        <PanelContent theme={theme}>
          <PropertySection theme={theme}>
            <SectionTitle theme={theme}>Basic Properties</SectionTitle>

            <PropertyGroup theme={theme}>
              <PropertyLabel theme={theme}>Label</PropertyLabel>
              <PropertyInput
                theme={theme}
                type="text"
                value={selectedNode.data?.label || ''}
                onChange={(e) => handleNodePropertyChange('label', e.target.value)}
                placeholder="Enter node label"
              />
            </PropertyGroup>

            <PropertyGroup theme={theme}>
              <PropertyLabel theme={theme}>Type</PropertyLabel>
              <PropertySelect
                theme={theme}
                value={selectedNode.type}
                onChange={(e) => actions.updateNode(selectedNode.id, { type: e.target.value })}
              >
                <option value="activity">Activity</option>
                <option value="decision">Decision</option>
                <option value="parallel">Parallel</option>
                <option value="saga">Saga</option>
              </PropertySelect>
            </PropertyGroup>
          </PropertySection>

          {selectedNode.type === 'activity' && (
            <PropertySection theme={theme}>
              <SectionTitle theme={theme}>Activity Configuration</SectionTitle>

              <PropertyGroup theme={theme}>
                <PropertyLabel theme={theme}>Activity Type</PropertyLabel>
                <PropertySelect
                  theme={theme}
                  value={selectedNode.data?.activityType?.id || ''}
                  onChange={(e) => handleNodePropertyChange('activityType', e.target.value)}
                >
                  <option value="">Select activity type</option>
                  <option value="http_get">HTTP GET</option>
                  <option value="http_post">HTTP POST</option>
                  <option value="db_query">Database Query</option>
                  <option value="mq_publish">Message Publish</option>
                </PropertySelect>
              </PropertyGroup>

              <PropertyGroup theme={theme}>
                <PropertyLabel theme={theme}>Timeout (ms)</PropertyLabel>
                <PropertyInput
                  theme={theme}
                  type="number"
                  value={selectedNode.data?.config?.timeout || ''}
                  onChange={(e) => handleNodePropertyChange('config', {
                    ...selectedNode.data?.config,
                    timeout: parseInt(e.target.value) || undefined
                  })}
                  placeholder="30000"
                />
              </PropertyGroup>
            </PropertySection>
          )}
        </PanelContent>
      </PanelContainer>
    );
  }

  if (selectedEdge) {
    return (
      <PanelContainer theme={theme}>
        <PanelHeader theme={theme}>
          <PanelTitle theme={theme}>Edge Properties</PanelTitle>
        </PanelHeader>

        <PanelContent theme={theme}>
          <PropertySection theme={theme}>
            <SectionTitle theme={theme}>Connection Properties</SectionTitle>

            <PropertyGroup theme={theme}>
              <PropertyLabel theme={theme}>Type</PropertyLabel>
              <PropertySelect
                theme={theme}
                value={selectedEdge.type || 'default'}
                onChange={(e) => actions.updateEdge(selectedEdge.id, { type: e.target.value })}
              >
                <option value="default">Default</option>
                <option value="conditional">Conditional</option>
                <option value="compensation">Compensation</option>
              </PropertySelect>
            </PropertyGroup>

            {(selectedEdge.type === 'conditional' || selectedEdge.type === 'compensation') && (
              <PropertyGroup theme={theme}>
                <PropertyLabel theme={theme}>
                  {selectedEdge.type === 'conditional' ? 'Condition' : 'Compensation Reference'}
                </PropertyLabel>
                <PropertyTextarea
                  theme={theme}
                  value={selectedEdge.data?.condition || selectedEdge.data?.compensationRef || ''}
                  onChange={(e) => actions.updateEdge(selectedEdge.id, {
                    data: {
                      ...selectedEdge.data,
                      [selectedEdge.type === 'conditional' ? 'condition' : 'compensationRef']: e.target.value
                    }
                  })}
                  placeholder={selectedEdge.type === 'conditional' ? 'Enter condition expression' : 'Enter compensation reference'}
                />
              </PropertyGroup>
            )}
          </PropertySection>
        </PanelContent>
      </PanelContainer>
    );
  }

  return (
    <PanelContainer theme={theme}>
      <PanelHeader theme={theme}>
        <PanelTitle theme={theme}>Workflow Properties</PanelTitle>
      </PanelHeader>

      <PanelContent theme={theme}>
        <PropertySection theme={theme}>
          <SectionTitle theme={theme}>Basic Information</SectionTitle>

          <PropertyGroup theme={theme}>
            <PropertyLabel theme={theme}>Workflow Name</PropertyLabel>
            <PropertyInput
              theme={theme}
              type="text"
              value={state.config.name}
              onChange={(e) => handleWorkflowPropertyChange('name', e.target.value)}
              placeholder="Enter workflow name"
            />
          </PropertyGroup>

          <PropertyGroup theme={theme}>
            <PropertyLabel theme={theme}>Description</PropertyLabel>
            <PropertyTextarea
              theme={theme}
              value={state.config.description || ''}
              onChange={(e) => handleWorkflowPropertyChange('description', e.target.value)}
              placeholder="Enter workflow description"
            />
          </PropertyGroup>

          <PropertyGroup theme={theme}>
            <PropertyLabel theme={theme}>Version</PropertyLabel>
            <PropertyInput
              theme={theme}
              type="text"
              value={state.config.version}
              onChange={(e) => handleWorkflowPropertyChange('version', e.target.value)}
              placeholder="1.0.0"
            />
          </PropertyGroup>

          <PropertyGroup theme={theme}>
            <PropertyLabel theme={theme}>Timeout (ms)</PropertyLabel>
            <PropertyInput
              theme={theme}
              type="number"
              value={state.config.timeout || ''}
              onChange={(e) => handleWorkflowPropertyChange('timeout', parseInt(e.target.value) || undefined)}
              placeholder="300000"
            />
          </PropertyGroup>
        </PropertySection>
      </PanelContent>
    </PanelContainer>
  );
};

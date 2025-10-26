import React, { useState } from 'react';
import styled from 'styled-components';
import { WorkflowCanvas } from './WorkflowCanvas';
import { ActivityPalette } from './ActivityPalette';
import { PropertyPanel } from './PropertyPanel';
import { Toolbar } from './Toolbar';
import { useWorkflow } from '../context/WorkflowContext';
import { useTheme } from '../context/ThemeContext';

const DesignerContainer = styled.div<{ theme: any }>`
  display: flex;
  flex-direction: column;
  height: 100vh;
  background-color: ${props => props.theme.colors.background};
  color: ${props => props.theme.colors.text};
`;

const DesignerHeader = styled.div<{ theme: any }>`
  height: 50px;
  border-bottom: 1px solid ${props => props.theme.colors.border};
  display: flex;
  align-items: center;
  padding: 0 ${props => props.theme.spacing.md};
  background-color: ${props => props.theme.colors.surface};
`;

const DesignerContent = styled.div`
  flex: 1;
  display: flex;
  overflow: hidden;
`;

const LeftPanel = styled.div<{ collapsed: boolean; theme: any }>`
  width: ${props => props.collapsed ? '60px' : '300px'};
  border-right: 1px solid ${props => props.theme.colors.border};
  background-color: ${props => props.theme.colors.surface};
  transition: width 0.3s ease;
  display: flex;
  flex-direction: column;
`;

const MainArea = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  position: relative;
`;

const CanvasContainer = styled.div`
  flex: 1;
  position: relative;
`;

const RightPanel = styled.div<{ collapsed: boolean; theme: any }>`
  width: ${props => props.collapsed ? '0' : '350px'};
  border-left: 1px solid ${props => props.theme.colors.border};
  background-color: ${props => props.theme.colors.surface};
  transition: width 0.3s ease;
  overflow: hidden;
`;

const CollapseButton = styled.button<{ theme: any }>`
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  background-color: ${props => props.theme.colors.surface};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: 50%;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 10;
  font-size: 12px;
  color: ${props => props.theme.colors.textSecondary};

  &:hover {
    background-color: ${props => props.theme.colors.primary};
    color: white;
  }
`;

const LeftCollapseButton = styled(CollapseButton)`
  right: -12px;
`;

const RightCollapseButton = styled(CollapseButton)`
  left: -12px;
`;

export interface WorkflowDesignerProps {
  readOnly?: boolean;
  showToolbar?: boolean;
  showPropertyPanel?: boolean;
  showActivityPalette?: boolean;
  className?: string;
}

export const WorkflowDesigner: React.FC<WorkflowDesignerProps> = ({
  readOnly = false,
  showToolbar = true,
  showPropertyPanel = true,
  showActivityPalette = true,
  className,
}) => {
  const { state } = useWorkflow();
  const { theme } = useTheme();

  const [leftPanelCollapsed, setLeftPanelCollapsed] = useState(false);
  const [rightPanelCollapsed, setRightPanelCollapsed] = useState(false);

  return (
    <DesignerContainer theme={theme} className={className}>
      {showToolbar && (
        <DesignerHeader theme={theme}>
          <Toolbar readOnly={readOnly} />
        </DesignerHeader>
      )}

      <DesignerContent>
        {showActivityPalette && (
          <LeftPanel theme={theme} collapsed={leftPanelCollapsed}>
            <ActivityPalette />
            <LeftCollapseButton
              theme={theme}
              onClick={() => setLeftPanelCollapsed(!leftPanelCollapsed)}
              title={leftPanelCollapsed ? "Expand Activity Palette" : "Collapse Activity Palette"}
            >
              {leftPanelCollapsed ? '→' : '←'}
            </LeftCollapseButton>
          </LeftPanel>
        )}

        <MainArea>
          <CanvasContainer>
            <WorkflowCanvas readOnly={readOnly} />
          </CanvasContainer>
        </MainArea>

        {showPropertyPanel && (
          <RightPanel theme={theme} collapsed={rightPanelCollapsed}>
            <PropertyPanel />
            <RightCollapseButton
              theme={theme}
              onClick={() => setRightPanelCollapsed(!rightPanelCollapsed)}
              title={rightPanelCollapsed ? "Expand Property Panel" : "Collapse Property Panel"}
            >
              {rightPanelCollapsed ? '←' : '→'}
            </RightCollapseButton>
          </RightPanel>
        )}
      </DesignerContent>
    </DesignerContainer>
  );
};

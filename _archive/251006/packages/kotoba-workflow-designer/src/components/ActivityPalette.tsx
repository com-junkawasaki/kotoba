import React, { useState } from 'react';
import styled from 'styled-components';
import { useTheme } from '../context/ThemeContext';
import { ActivityType, ActivityCategory } from '../types';

const PaletteContainer = styled.div<{ theme: any }>`
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: ${props => props.theme.colors.surface};
  border-right: 1px solid ${props => props.theme.colors.border};
`;

const PaletteHeader = styled.div<{ theme: any }>`
  padding: ${props => props.theme.spacing.md};
  border-bottom: 1px solid ${props => props.theme.colors.border};
  background-color: ${props => props.theme.colors.background};
`;

const PaletteTitle = styled.h3<{ theme: any }>`
  margin: 0;
  font-size: ${props => props.theme.fontSize.md};
  color: ${props => props.theme.colors.text};
  font-weight: 600;
`;

const SearchInput = styled.input<{ theme: any }>`
  width: 100%;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  margin-top: ${props => props.theme.spacing.sm};
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

  &::placeholder {
    color: ${props => props.theme.colors.textSecondary};
  }
`;

const CategoryTabs = styled.div<{ theme: any }>`
  display: flex;
  border-bottom: 1px solid ${props => props.theme.colors.border};
  background-color: ${props => props.theme.colors.background};
`;

const CategoryTab = styled.button<{ active: boolean; theme: any }>`
  flex: 1;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  border: none;
  background-color: ${props => props.active ? props.theme.colors.primary : 'transparent'};
  color: ${props => props.active ? 'white' : props.theme.colors.text};
  font-size: ${props => props.theme.fontSize.sm};
  font-weight: ${props => props.active ? '600' : '400'};
  cursor: pointer;
  transition: all 0.2s ease;

  &:hover {
    background-color: ${props => props.active ? props.theme.colors.primary : props.theme.colors.background};
  }

  &:first-child {
    border-radius: ${props => props.theme.borderRadius} 0 0 0;
  }

  &:last-child {
    border-radius: 0 ${props => props.theme.borderRadius} 0 0;
  }
`;

const ActivityList = styled.div<{ theme: any }>`
  flex: 1;
  overflow-y: auto;
  padding: ${props => props.theme.spacing.sm};
`;

const ActivityItem = styled.div<{ theme: any; isDragging: boolean }>`
  display: flex;
  align-items: center;
  padding: ${props => props.theme.spacing.sm} ${props => props.theme.spacing.md};
  margin-bottom: ${props => props.theme.spacing.xs};
  border: 1px solid ${props => props.theme.colors.border};
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.theme.colors.surface};
  cursor: grab;
  transition: all 0.2s ease;

  &:hover {
    border-color: ${props => props.theme.colors.primary};
    box-shadow: 0 2px 4px ${props => props.theme.colors.primary}33;
  }

  ${props => props.isDragging && `
    opacity: 0.5;
    transform: rotate(5deg);
    cursor: grabbing;
  `}
`;

const ActivityIcon = styled.div<{ theme: any }>`
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: ${props => props.theme.spacing.sm};
  font-size: 16px;
  border-radius: ${props => props.theme.borderRadius};
  background-color: ${props => props.theme.colors.primary};
  color: white;
`;

const ActivityInfo = styled.div`
  flex: 1;
`;

const ActivityName = styled.div<{ theme: any }>`
  font-size: ${props => props.theme.fontSize.sm};
  font-weight: 600;
  color: ${props => props.theme.colors.text};
  margin-bottom: 2px;
`;

const ActivityDescription = styled.div<{ theme: any }>`
  font-size: ${props => props.theme.fontSize.xs};
  color: ${props => props.theme.colors.textSecondary};
  line-height: 1.3;
`;

// Predefined activity types
const DEFAULT_ACTIVITIES: { [key in ActivityCategory]: ActivityType[] } = {
  [ActivityCategory.GENERAL]: [
    {
      id: 'start',
      name: 'Start',
      category: ActivityCategory.GENERAL,
      icon: '🚀',
      description: 'Workflow start point',
      inputs: [],
      outputs: [{ name: 'output', type: 'object', required: true }],
      configSchema: {},
    },
    {
      id: 'end',
      name: 'End',
      category: ActivityCategory.GENERAL,
      icon: '🏁',
      description: 'Workflow end point',
      inputs: [{ name: 'input', type: 'object', required: false }],
      outputs: [],
      configSchema: {},
    },
  ],
  [ActivityCategory.HTTP]: [
    {
      id: 'http_get',
      name: 'HTTP GET',
      category: ActivityCategory.HTTP,
      icon: '🌐',
      description: 'Make HTTP GET request',
      inputs: [
        { name: 'url', type: 'string', required: true },
        { name: 'headers', type: 'object', required: false },
      ],
      outputs: [
        { name: 'status', type: 'number', required: true },
        { name: 'response', type: 'object', required: true },
      ],
      configSchema: {
        timeout: { type: 'number', default: 30000 },
        retry: { type: 'boolean', default: true },
      },
    },
    {
      id: 'http_post',
      name: 'HTTP POST',
      category: ActivityCategory.HTTP,
      icon: '📤',
      description: 'Make HTTP POST request',
      inputs: [
        { name: 'url', type: 'string', required: true },
        { name: 'body', type: 'object', required: true },
        { name: 'headers', type: 'object', required: false },
      ],
      outputs: [
        { name: 'status', type: 'number', required: true },
        { name: 'response', type: 'object', required: true },
      ],
      configSchema: {
        timeout: { type: 'number', default: 30000 },
        retry: { type: 'boolean', default: true },
      },
    },
  ],
  [ActivityCategory.DATABASE]: [
    {
      id: 'db_query',
      name: 'Database Query',
      category: ActivityCategory.DATABASE,
      icon: '🗄️',
      description: 'Execute database query',
      inputs: [
        { name: 'query', type: 'string', required: true },
        { name: 'params', type: 'object', required: false },
      ],
      outputs: [
        { name: 'result', type: 'array', required: true },
        { name: 'affectedRows', type: 'number', required: false },
      ],
      configSchema: {
        connection: { type: 'string', required: true },
        timeout: { type: 'number', default: 30000 },
      },
    },
    {
      id: 'db_insert',
      name: 'Database Insert',
      category: ActivityCategory.DATABASE,
      icon: '💾',
      description: 'Insert data into database',
      inputs: [
        { name: 'table', type: 'string', required: true },
        { name: 'data', type: 'object', required: true },
      ],
      outputs: [
        { name: 'insertedId', type: 'string', required: false },
        { name: 'affectedRows', type: 'number', required: true },
      ],
      configSchema: {
        connection: { type: 'string', required: true },
        timeout: { type: 'number', default: 30000 },
      },
    },
  ],
  [ActivityCategory.MESSAGE_QUEUE]: [
    {
      id: 'mq_publish',
      name: 'Publish Message',
      category: ActivityCategory.MESSAGE_QUEUE,
      icon: '📨',
      description: 'Publish message to queue',
      inputs: [
        { name: 'queue', type: 'string', required: true },
        { name: 'message', type: 'object', required: true },
      ],
      outputs: [
        { name: 'messageId', type: 'string', required: true },
      ],
      configSchema: {
        exchange: { type: 'string', default: '' },
        routingKey: { type: 'string', default: '' },
      },
    },
    {
      id: 'mq_consume',
      name: 'Consume Message',
      category: ActivityCategory.MESSAGE_QUEUE,
      icon: '📬',
      description: 'Consume message from queue',
      inputs: [
        { name: 'queue', type: 'string', required: true },
      ],
      outputs: [
        { name: 'message', type: 'object', required: true },
        { name: 'messageId', type: 'string', required: true },
      ],
      configSchema: {
        timeout: { type: 'number', default: 30000 },
        ack: { type: 'boolean', default: true },
      },
    },
  ],
  [ActivityCategory.CLOUD_STORAGE]: [
    {
      id: 's3_upload',
      name: 'S3 Upload',
      category: ActivityCategory.CLOUD_STORAGE,
      icon: '☁️',
      description: 'Upload file to S3',
      inputs: [
        { name: 'bucket', type: 'string', required: true },
        { name: 'key', type: 'string', required: true },
        { name: 'data', type: 'object', required: true },
      ],
      outputs: [
        { name: 'url', type: 'string', required: true },
        { name: 'etag', type: 'string', required: false },
      ],
      configSchema: {
        region: { type: 'string', default: 'us-east-1' },
        acl: { type: 'string', default: 'private' },
      },
    },
  ],
  [ActivityCategory.EMAIL]: [
    {
      id: 'send_email',
      name: 'Send Email',
      category: ActivityCategory.EMAIL,
      icon: '📧',
      description: 'Send email message',
      inputs: [
        { name: 'to', type: 'string', required: true },
        { name: 'subject', type: 'string', required: true },
        { name: 'body', type: 'string', required: true },
        { name: 'from', type: 'string', required: false },
      ],
      outputs: [
        { name: 'messageId', type: 'string', required: true },
      ],
      configSchema: {
        smtpServer: { type: 'string', required: true },
        smtpPort: { type: 'number', default: 587 },
        useTLS: { type: 'boolean', default: true },
      },
    },
  ],
  [ActivityCategory.WEBHOOK]: [
    {
      id: 'webhook_call',
      name: 'Webhook Call',
      category: ActivityCategory.WEBHOOK,
      icon: '🔗',
      description: 'Call webhook endpoint',
      inputs: [
        { name: 'url', type: 'string', required: true },
        { name: 'method', type: 'string', required: false, defaultValue: 'POST' },
        { name: 'payload', type: 'object', required: false },
        { name: 'headers', type: 'object', required: false },
      ],
      outputs: [
        { name: 'status', type: 'number', required: true },
        { name: 'response', type: 'object', required: true },
      ],
      configSchema: {
        timeout: { type: 'number', default: 30000 },
        retry: { type: 'boolean', default: true },
      },
    },
  ],
  [ActivityCategory.CUSTOM]: [],
};

export const ActivityPalette: React.FC = () => {
  const { theme } = useTheme();
  const [selectedCategory, setSelectedCategory] = useState<ActivityCategory>(ActivityCategory.GENERAL);
  const [searchQuery, setSearchQuery] = useState('');
  const [draggingItem, setDraggingItem] = useState<ActivityType | null>(null);

  const categories = Object.keys(DEFAULT_ACTIVITIES) as ActivityCategory[];
  const activities = DEFAULT_ACTIVITIES[selectedCategory] || [];

  const filteredActivities = activities.filter(activity =>
    activity.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    activity.description.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleDragStart = (activity: ActivityType) => {
    setDraggingItem(activity);
  };

  const handleDragEnd = () => {
    setDraggingItem(null);
  };

  return (
    <PaletteContainer theme={theme}>
      <PaletteHeader theme={theme}>
        <PaletteTitle theme={theme}>Activities</PaletteTitle>
        <SearchInput
          theme={theme}
          type="text"
          placeholder="Search activities..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </PaletteHeader>

      <CategoryTabs theme={theme}>
        {categories.map((category) => (
          <CategoryTab
            key={category}
            theme={theme}
            active={selectedCategory === category}
            onClick={() => setSelectedCategory(category)}
          >
            {category.charAt(0).toUpperCase() + category.slice(1).replace('_', ' ')}
          </CategoryTab>
        ))}
      </CategoryTabs>

      <ActivityList theme={theme}>
        {filteredActivities.map((activity) => (
          <ActivityItem
            key={activity.id}
            theme={theme}
            isDragging={draggingItem?.id === activity.id}
            draggable
            onDragStart={() => handleDragStart(activity)}
            onDragEnd={handleDragEnd}
            title={activity.description}
          >
            <ActivityIcon theme={theme}>
              {activity.icon}
            </ActivityIcon>
            <ActivityInfo>
              <ActivityName theme={theme}>{activity.name}</ActivityName>
              <ActivityDescription theme={theme}>
                {activity.description}
              </ActivityDescription>
            </ActivityInfo>
          </ActivityItem>
        ))}

        {filteredActivities.length === 0 && (
          <div style={{
            textAlign: 'center',
            color: theme.colors.textSecondary,
            padding: theme.spacing.lg,
            fontSize: theme.fontSize.sm,
          }}>
            No activities found
          </div>
        )}
      </ActivityList>
    </PaletteContainer>
  );
};

import React from 'react';
import { KotobaComponent, KotobaConfig } from './kotobaParser';

// コンポーネントタイプのマッピング
const ComponentTypeMap: Record<string, React.ComponentType<any>> = {
  layout: ({ children, ...props }: any) => <div className="app" {...props}>{children}</div>,
  header: ({ title, subtitle, children, ...props }: any) => (
    <header {...props}>
      <div className="header-content">
        <div>
          <h1>{title}</h1>
          {subtitle && <p className="header-subtitle">{subtitle}</p>}
        </div>
        {children}
      </div>
    </header>
  ),
  main: ({ children, ...props }: any) => <main {...props}>{children}</main>,
  section: ({ title, children, ...props }: any) => (
    <section {...props}>
      {title && <h2>{title}</h2>}
      {children}
    </section>
  ),
  button: ({ text, onClick, ...props }: any) => (
    <button onClick={onClick} {...props}>{text}</button>
  ),
  input: (props: any) => <input {...props} />,
  textarea: (props: any) => <textarea {...props} />,
  form: ({ children, ...props }: any) => <form {...props}>{children}</form>,
  div: ({ children, text, ...props }: any) => <div {...props}>{text || children}</div>,
  aside: ({ children, ...props }: any) => <aside {...props}>{children}</aside>,
  nav: ({ children, ...props }: any) => <nav {...props}>{children}</nav>,
  'nav-item': ({ text, active, onClick, ...props }: any) => (
    <button
      className={`nav-item ${active ? 'active' : ''}`}
      onClick={onClick}
      {...props}
    >
      {text}
    </button>
  ),
  stat: ({ label, value, icon, ...props }: any) => (
    <div {...props}>
      {icon && <div className="stat-icon">{icon}</div>}
      <h3>{value}</h3>
      <p>{label}</p>
    </div>
  ),
  footer: ({ children, ...props }: any) => <footer {...props}>{children}</footer>,
  status: ({ text, status, ...props }: any) => (
    <div className={`status status-${status}`} {...props}>
      <span className="status-indicator"></span>
      {text}
    </div>
  ),
};

export class KotobaComponentBuilder {
  private config: KotobaConfig;
  private handlers: Map<string, Function> = new Map();
  private states: Map<string, any> = new Map();

  constructor(config: KotobaConfig) {
    this.config = config;
  }

  /**
   * イベントハンドラーを登録
   */
  registerHandler(name: string, handler: Function) {
    this.handlers.set(name, handler);
  }

  /**
   * ステートを登録
   */
  registerState(name: string, initialValue: any) {
    this.states.set(name, initialValue);
  }

  /**
   * ステートを取得
   */
  getState(name: string) {
    return this.states.get(name);
  }

  /**
   * ステートを更新
   */
  setState(name: string, value: any) {
    this.states.set(name, value);
  }

  /**
   * KotobaコンポーネントをReactコンポーネントに変換
   */
  buildComponent(componentName: string): React.ReactElement | null {
    const component = this.config.components.get(componentName);
    if (!component) {
      console.warn(`Component ${componentName} not found`);
      return null;
    }

    const ComponentType = ComponentTypeMap[component.component_type || 'div'];
    if (!ComponentType) {
      console.warn(`Unknown component type: ${component.component_type}`);
      return React.createElement('div', { key: componentName }, `Unknown component: ${componentName}`);
    }

    // propsの処理
    const props = { ...component.props, key: componentName };

    // イベントハンドラーの処理
    if (props.onClick && typeof props.onClick === 'string') {
      const handlerName = props.onClick;
      const handler = this.handlers.get(handlerName);
      if (handler) {
        props.onClick = handler;
      } else {
        console.warn(`Handler ${handlerName} not found`);
        props.onClick = () => console.log(`Handler ${handlerName} not implemented`);
      }
    }

    // 子コンポーネントの処理
    let children: React.ReactNode[] = [];
    if (component.children) {
      children = component.children
        .map(childName => this.buildComponent(childName))
        .filter(Boolean) as React.ReactElement[];
    }

    return React.createElement(ComponentType, props, ...children);
  }

  /**
   * アプリケーション全体を構築
   */
  buildApp(): React.ReactElement | null {
    const rootComponent = this.config.components.get('App');
    if (!rootComponent) {
      console.error('Root App component not found');
      return null;
    }

    return this.buildComponent('App');
  }

  /**
   * 動的ステート更新用のフック
   */
  useKotobaState(stateName: string) {
    const [value, setValue] = React.useState(this.getState(stateName));

    React.useEffect(() => {
      const interval = setInterval(() => {
        const currentValue = this.getState(stateName);
        if (currentValue !== value) {
          setValue(currentValue);
        }
      }, 100);

      return () => clearInterval(interval);
    }, [stateName, value]);

    return [value, (newValue: any) => this.setState(stateName, newValue)];
  }
}

// Kotobaアプリケーションのメインコンポーネント
export const KotobaApp: React.FC<{ config: KotobaConfig }> = ({ config }) => {
  const [builder] = React.useState(() => new KotobaComponentBuilder(config));
  const [app, setApp] = React.useState<React.ReactElement | null>(null);

  React.useEffect(() => {
    // 初期ステートの設定
    config.states.forEach((value, key) => {
      builder.registerState(key, value);
    });

    // ハンドラーの登録
    builder.registerHandler('createGraph', async () => {
      try {
        setLoading(true);
        const result = await (window as any).__TAURI__.invoke('create_graph');
        const stats = await (window as any).__TAURI__.invoke('get_graph_stats');
        builder.setState('stats', {
          vertices: stats.vertices,
          edges: stats.edges,
          density: stats.edges > 0 && stats.vertices > 1 ? (stats.edges / (stats.vertices * (stats.vertices - 1) / 2)) : 0,
          lastUpdate: stats.last_update
        });
        builder.setState('notification', { type: 'success', message: result });
        setApp(builder.buildApp());
      } catch (error) {
        console.error('Failed to create graph:', error);
        builder.setState('notification', { type: 'error', message: 'Failed to create graph' });
        setApp(builder.buildApp());
      } finally {
        setLoading(false);
      }
    });

    builder.registerHandler('clearGraph', async () => {
      try {
        setLoading(true);
        const result = await (window as any).__TAURI__.invoke('clear_graph');
        const stats = await (window as any).__TAURI__.invoke('get_graph_stats');
        builder.setState('stats', {
          vertices: stats.vertices,
          edges: stats.edges,
          density: 0.0,
          lastUpdate: stats.last_update
        });
        builder.setState('notification', { type: 'success', message: result });
        setApp(builder.buildApp());
      } catch (error) {
        console.error('Failed to clear graph:', error);
        builder.setState('notification', { type: 'error', message: 'Failed to clear graph' });
        setApp(builder.buildApp());
      } finally {
        setLoading(false);
      }
    });

    builder.registerHandler('addVertex', async () => {
      try {
        setLoading(true);
        // フォームから値を取得
        const labelInput = document.querySelector('input[placeholder*="Vertex label"]') as HTMLInputElement;
        const propsInput = document.querySelector('textarea[placeholder*="Properties"]') as HTMLTextAreaElement;

        const label = labelInput?.value || 'DefaultVertex';
        let properties = {};

        if (propsInput?.value) {
          try {
            properties = JSON.parse(propsInput.value);
          } catch (e) {
            properties = { raw_input: propsInput.value };
          }
        }

        const result = await (window as any).__TAURI__.invoke('add_vertex', { label, properties });
        const stats = await (window as any).__TAURI__.invoke('get_graph_stats');
        builder.setState('stats', {
          vertices: stats.vertices,
          edges: stats.edges,
          density: stats.edges > 0 && stats.vertices > 1 ? (stats.edges / (stats.vertices * (stats.vertices - 1) / 2)) : 0,
          lastUpdate: stats.last_update
        });
        builder.setState('notification', { type: 'success', message: result });
        setApp(builder.buildApp());

        // フォームをクリア
        if (labelInput) labelInput.value = '';
        if (propsInput) propsInput.value = '';
      } catch (error) {
        console.error('Failed to add vertex:', error);
        builder.setState('notification', { type: 'error', message: 'Failed to add vertex' });
        setApp(builder.buildApp());
      } finally {
        setLoading(false);
      }
    });

    builder.registerHandler('exportGraph', async () => {
      try {
        // TODO: グラフエクスポート機能を実装
        builder.setState('notification', { type: 'info', message: 'Export feature coming soon!' });
        setApp(builder.buildApp());
      } catch (error) {
        console.error('Failed to export graph:', error);
      }
    });

    builder.registerHandler('toggleTheme', () => {
      const currentTheme = builder.getState('theme');
      const newTheme = currentTheme === 'light' ? 'dark' : 'light';
      builder.setState('theme', newTheme);
      document.documentElement.setAttribute('data-theme', newTheme);
      setApp(builder.buildApp());
    });

    builder.registerHandler('openSettings', () => {
      builder.setState('notification', { type: 'info', message: 'Settings panel coming soon!' });
      setApp(builder.buildApp());
    });

    builder.registerHandler('showGraphPanel', () => {
      builder.setState('currentPanel', 'graph');
      setApp(builder.buildApp());
    });

    builder.registerHandler('showQueryPanel', () => {
      builder.setState('currentPanel', 'query');
      setApp(builder.buildApp());
    });

    builder.registerHandler('showAnalyticsPanel', () => {
      builder.setState('currentPanel', 'analytics');
      setApp(builder.buildApp());
    });

    setApp(builder.buildApp());
  }, [config, builder]);

  return app;
};

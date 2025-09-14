// Kotobaファイルパーサー
// .kotobaファイルを解析してReactコンポーネント構造に変換

export interface KotobaComponent {
  type: 'component' | 'config' | 'handler' | 'state';
  name: string;
  component_type?: string;
  props?: Record<string, any>;
  children?: string[];
  function?: string;
  initial?: any;
  [key: string]: any;
}

export interface KotobaConfig {
  name: string;
  version: string;
  theme: string;
  components: Map<string, KotobaComponent>;
  handlers: Map<string, KotobaComponent>;
  states: Map<string, any>;
}

export class KotobaParser {
  private components: Map<string, KotobaComponent> = new Map();
  private handlers: Map<string, KotobaComponent> = new Map();
  private states: Map<string, any> = new Map();
  private config: Partial<KotobaConfig> = {};

  /**
   * .kotobaファイルを解析
   */
  async parseFile(filePath: string): Promise<KotobaConfig> {
    try {
      const response = await fetch(filePath);
      const text = await response.text();
      return this.parse(text);
    } catch (error) {
      console.error('Failed to parse kotoba file:', error);
      throw error;
    }
  }

  /**
   * .kotobaテキストを解析
   */
  parse(content: string): KotobaConfig {
    const lines = content.split('\n').filter(line => line.trim() && !line.startsWith('#'));

    for (const line of lines) {
      try {
        const item: KotobaComponent = JSON.parse(line.trim());

        switch (item.type) {
          case 'config':
            this.config = { ...this.config, ...item };
            break;
          case 'component':
            this.components.set(item.name, item);
            break;
          case 'handler':
            this.handlers.set(item.name, item);
            break;
          case 'state':
            this.states.set(item.name, item.initial);
            break;
        }
      } catch (error) {
        console.warn('Failed to parse line:', line, error);
      }
    }

    return {
      name: this.config.name || 'KotobaApp',
      version: this.config.version || '0.1.0',
      theme: this.config.theme || 'light',
      components: this.components,
      handlers: this.handlers,
      states: this.states,
    };
  }

  /**
   * コンポーネントの依存関係を解決
   */
  resolveDependencies(componentName: string): string[] {
    const component = this.components.get(componentName);
    if (!component || !component.children) {
      return [];
    }

    const dependencies: string[] = [];
    for (const childName of component.children) {
      dependencies.push(childName);
      dependencies.push(...this.resolveDependencies(childName));
    }

    return [...new Set(dependencies)]; // 重複を除去
  }

  /**
   * コンポーネントツリーを構築
   */
  buildComponentTree(rootName: string): KotobaComponent | null {
    const component = this.components.get(rootName);
    if (!component) {
      return null;
    }

    if (component.children) {
      component.children = component.children.map(childName => {
        const childComponent = this.buildComponentTree(childName);
        return childComponent ? childComponent.name : childName;
      }).filter(Boolean);
    }

    return component;
  }
}

// デフォルトのKotobaパーサーインスタンス
export const kotobaParser = new KotobaParser();

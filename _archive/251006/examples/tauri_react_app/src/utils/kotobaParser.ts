// Kotobaファイルパーサー
// .kotobaファイル（Jsonnet形式）を解析してReactコンポーネント構造に変換

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
   * .kotobaファイルを解析（Jsonnet形式）
   */
  async parseFile(filePath: string): Promise<KotobaConfig> {
    try {
      const response = await fetch(filePath);
      const text = await response.text();
      return this.parseJsonnet(text);
    } catch (error) {
      console.error('Failed to parse kotoba file:', error);
      throw error;
    }
  }

  /**
   * .kotobaテキストを解析（Jsonnet形式）
   */
  parseJsonnet(content: string): KotobaConfig {
    try {
      // Jsonnetを評価してJSONに変換（実際の環境ではjsonnetコマンドやライブラリを使用）
      const jsonContent = this.evaluateJsonnet(content);
      return this.parseJsonObject(jsonContent);
    } catch (error) {
      console.error('Failed to parse Jsonnet:', error);
      throw error;
    }
  }

  /**
   * Jsonnetコンテンツを評価（簡易実装）
   * 実際の環境ではjsonnetコマンドやライブラリを使用
   */
  private evaluateJsonnet(content: string): any {
    // 簡易的なJsonnet評価（実際には外部プロセスやライブラリを使用）
    // ここでは基本的なJsonnet構文のみサポート
    try {
      // 基本的なJsonnetをJavaScriptオブジェクトに変換
      const processedContent = content
        .replace(/\/\/.*$/gm, '') // コメント除去
        .replace(/\s+/g, ' ') // 余分な空白除去
        .trim();

      // 簡易的なJsonnet評価（実際のアプリケーションでは適切なライブラリを使用）
      return JSON.parse(processedContent);
    } catch (error) {
      console.warn('Jsonnet evaluation failed, falling back to direct JSON parsing');
      return JSON.parse(content);
    }
  }

  /**
   * JSONオブジェクトからKotoba設定を構築
   */
  private parseJsonObject(jsonContent: any): KotobaConfig {
    // 設定の初期化
    if (jsonContent.config) {
      this.config = { ...this.config, ...jsonContent.config };
    }

    // コンポーネントの処理
    if (jsonContent.components) {
      for (const [name, component] of Object.entries(jsonContent.components)) {
        this.components.set(name, component as KotobaComponent);
      }
    }

    // ハンドラーの処理
    if (jsonContent.handlers) {
      for (const [name, handler] of Object.entries(jsonContent.handlers)) {
        this.handlers.set(name, handler as KotobaComponent);
      }
    }

    // 状態の処理
    if (jsonContent.states) {
      for (const [name, state] of Object.entries(jsonContent.states)) {
        this.states.set(name, (state as any).initial);
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

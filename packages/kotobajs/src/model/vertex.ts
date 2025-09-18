// MERKLE: d4e5f6a7 (Base Vertex Model)

import { KotobaSchema, infer } from '../validation/schema';
import { KotobaClient } from '../client';
import { QueryBuilder } from '../query/builder';

export class Vertex<T extends KotobaSchema<any>> {
  protected static client: KotobaClient;

  // Set the client for all models that extend Vertex.
  public static use(client: KotobaClient) {
    this.client = client;
  }

  public readonly id: string;
  public readonly props: infer<T>;

  constructor(id: string, properties: infer<T>) {
    this.id = id;
    this.props = properties;
  }

  // --- Static methods for data fetching ---

  /**
   * Starts a new type-safe query.
   * @returns A QueryBuilder instance for this model.
   */
  static query<T extends typeof Vertex>(this: T): QueryBuilder<T> {
    if (!this.client) {
      throw new Error(`KotobaClient not initialized for ${this.name}. Call ${this.name}.use(client).`);
    }
    return new QueryBuilder(this, this.client);
  }

  static async findById<T extends typeof Vertex>(this: T, id: string): Promise<InstanceType<T> | null> {
    const modelName = this.name;
    if (!this.client) throw new Error(`KotobaClient not initialized for ${modelName}. Call ${modelName}.use(client).`);

    console.log(`[Vertex.findById] Fetching ${modelName} with id: ${id}`);

    // Example GQL Query
    const query = `
      query Get${modelName}($id: ID!) {
        get${modelName}(id: $id) {
          id
          # ...all properties defined in the schema would be listed here
        }
      }
    `;

    // const result = await this.client.query(query, { id });
    // if (!result || !result[`get${modelName}`]) {
    //   return null;
    // }
    // return new this(id, result[`get${modelName}`]) as InstanceType<T>;
    
    // For now, returning null as we don't have a live server.
    return null;
  }

  static async create<T extends typeof Vertex>(this: T, data: any): Promise<InstanceType<T>> {
    const modelName = this.name;
    if (!this.client) throw new Error(`KotobaClient not initialized for ${modelName}. Call ${modelName}.use(client).`);

    // In a real implementation, you would validate the data first.
    // (this.schema as any).parse(data);

    console.log(`[Vertex.create] Creating a new ${modelName} with data:`, data);

    // Example GQL Mutation
    const mutation = `
      mutation Create${modelName}($input: Create${modelName}Input!) {
        create${modelName}(input: $input) {
          id
          # ...all properties would be returned here
        }
      }
    `;

    // const result = await this.client.query(mutation, { input: data });
    // const newProperties = result[`create${modelName}`];
    // return new this(newProperties.id, newProperties) as InstanceType<T>;

    // For now, returning a mock instance.
    const newId = `new-${modelName}-id-${Math.random()}`;
    return new this(newId, data) as InstanceType<T>;
  }

  // --- Instance methods for graph traversal ---

  async addEdge<E extends Edge<any, any, any>>(
    EdgeModel: { new(...args: any[]): E; create(source: this, dest: Vertex<any>, props: any): Promise<E> },
    destination: Vertex<any>,
    properties: E['props']
  ): Promise<E> {
    console.log(`[Vertex.addEdge] Creating edge from ${this.id} to ${destination.id}`);
    return EdgeModel.create(this, destination, properties);
  }

  async out<V extends Vertex<any>>(edgeLabel: string): Promise<V[]> {
    console.log(`[Vertex.out] Traversing out via '${edgeLabel}' from ${this.id}`);
    // Placeholder for GQL query to get outgoing neighbors.
    return [];
  }

  async in<V extends Vertex<any>>(edgeLabel: string): Promise<V[]> {
    console.log(`[Vertex.in] Traversing in via '${edgeLabel}' to ${this.id}`);
    // Placeholder for GQL query to get incoming neighbors.
    return [];
  }
}

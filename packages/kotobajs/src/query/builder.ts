// MERKLE: b1c2d3e4 (Type-Safe Query Builder)

import { Vertex } from '../model/vertex';
import { KotobaClient } from '../client';

// Define the structure for where clauses to be type-safe.
// e.g., { name: { eq: 'Alice' }, age: { gt: 20 } }
type WhereClause<T> = {
  [P in keyof T]?: T[P] | { eq?: T[P]; gt?: T[P]; lt?: T[P]; gte?: T[P]; lte?: T[P]; contains?: string };
};

export class QueryBuilder<M extends typeof Vertex, T = InstanceType<M>> {
  private client: KotobaClient;
  private modelClass: M;
  private filters: WhereClause<T['props']> = {};
  private limitCount?: number;

  constructor(modelClass: M, client: KotobaClient) {
    this.modelClass = modelClass;
    this.client = client;
  }

  /**
   * Adds a filter to the query.
   * This is type-safe and only allows properties of the model.
   */
  where(filters: WhereClause<T['props']>): this {
    this.filters = { ...this.filters, ...filters };
    return this;
  }

  /**
   * Sets the maximum number of results to return.
   */
  limit(count: number): this {
    this.limitCount = count;
    return this;
  }

  /**
   * Executes the query and returns the results.
   */
  async exec(): Promise<T[]> {
    const modelName = this.modelClass.name;
    // This is a simplified GQL query builder. A real implementation would be more robust.
    const whereString = JSON.stringify(this.filters); // Placeholder for actual GQL filter syntax
    
    console.log(`[QueryBuilder] Executing query for ${modelName}`);
    console.log(`   - Filters: ${whereString}`);
    console.log(`   - Limit: ${this.limitCount}`);

    const query = `
      query Find${modelName}s($where: JSON, $limit: Int) {
        find${modelName}s(where: $where, limit: $limit) {
          id
          # ...all properties
        }
      }
    `;

    // const result = await this.client.query(query, {
    //   where: this.filters,
    //   limit: this.limitCount,
    // });
    
    // const items = result[`find${modelName}s`] || [];
    // return items.map((item: any) => new this.modelClass(item.id, item));
    
    // Returning empty array as we don't have a live server.
    return [];
  }
}

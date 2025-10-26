// MERKLE: a7b8c9d0 (Base Edge Model)

import { KotobaSchema, infer } from '../validation/schema';
import { Vertex } from './vertex';
import { KotobaClient } from '../client';

export class Edge<T extends KotobaSchema<any>, S extends Vertex<any>, D extends Vertex<any>> {
  protected static client: KotobaClient;

  public static use(client: KotobaClient) {
    this.client = client;
  }

  public readonly id: string;
  public readonly source: S;
  public readonly destination: D;
  public readonly props: infer<T>;

  constructor(id: string, source: S, destination: D, properties: infer<T>) {
    this.id = id;
    this.source = source;
    this.destination = destination;
    this.props = properties;
  }
}

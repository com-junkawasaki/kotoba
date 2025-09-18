// MERKLE: e8a9b0c1 (Kotoba API Client)

import axios, { AxiosInstance } from 'axios';

export interface KotobaClientOptions {
  /** The URL of the kotoba-server's GraphQL endpoint. */
  serverUrl: string;
  /** Optional authentication token. */
  authToken?: string;
}

export class KotobaClient {
  private axiosInstance: AxiosInstance;

  constructor(options: KotobaClientOptions) {
    if (!options.serverUrl) {
      throw new Error('serverUrl is required to initialize the KotobaClient.');
    }

    this.axiosInstance = axios.create({
      baseURL: options.serverUrl,
      headers: {
        'Content-Type': 'application/json',
        ...(options.authToken && { 'Authorization': `Bearer ${options.authToken}` }),
      },
    });
  }

  /**
   * Sends a GraphQL query to the kotoba-server.
   * @param query The GraphQL query string.
   * @param variables Optional variables for the query.
   * @returns The data returned from the server.
   */
  public async query<T = any>(query: string, variables?: Record<string, any>): Promise<T> {
    try {
      console.log(`[KotobaClient] Sending query:`, { query, variables });
      const response = await this.axiosInstance.post('', {
        query,
        variables,
      });

      if (response.data.errors) {
        throw new Error(`GraphQL Error: ${JSON.stringify(response.data.errors)}`);
      }

      return response.data.data;
    } catch (error) {
      console.error('[KotobaClient] Query failed:', error);
      // Re-throw the error to be handled by the caller
      throw error;
    }
  }
}

import { WorkflowExecution, WorkflowIR } from './types';

export class KotobaWorkflowClient {
  private baseUrl: string;

  constructor(baseUrl: string = 'http://localhost:3000') {
    // Ensure baseUrl doesn't have a trailing slash
    this.baseUrl = baseUrl.endsWith('/') ? baseUrl.slice(0, -1) : baseUrl;
  }

  private async request<T>(
    path: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}/api/v1/workflows${path}`;
    
    const response = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ error: 'Request failed with status ' + response.status }));
      throw new Error(errorData.error || 'An unknown error occurred');
    }

    // Handle empty response body for certain statuses
    if (response.status === 204 || response.headers.get('content-length') === '0') {
      return null as T;
    }

    return response.json() as Promise<T>;
  }

  /**
   * Starts a new workflow execution.
   * @param workflowIr The Intermediate Representation of the workflow to start.
   * @returns An object containing the execution ID of the started workflow.
   */
  async start(workflowIr: WorkflowIR): Promise<{ execution_id: string }> {
    return this.request<{ execution_id: string }>('/', {
      method: 'POST',
      body: JSON.stringify(workflowIr),
    });
  }

  /**
   * Retrieves the status and details of a specific workflow execution.
   * @param executionId The ID of the workflow execution.
   * @returns The workflow execution details.
   */
  async getStatus(executionId: string): Promise<WorkflowExecution> {
    return this.request<WorkflowExecution>(`/${executionId}`, {
      method: 'GET',
    });
  }
}

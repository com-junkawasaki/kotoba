// MERKLE: c5d8e9f2 (Validation Schema Core - Enhanced)

// --- Custom Error Handling ---
export interface ValidationErrorIssue {
  path: (string | number)[];
  message: string;
}

export class ValidationError extends Error {
  public readonly issues: ValidationErrorIssue[] = [];

  constructor(issues: ValidationErrorIssue[]) {
    const message = `Validation failed with ${issues.length} issue(s):\n` +
      issues.map(i => `  - at path '${i.path.join('.')}': ${i.message}`).join('\n');
    super(message);
    this.name = 'ValidationError';
    this.issues = issues;
  }
}

// --- Base Schema Class ---
export abstract class KotobaSchema<T> {
  protected isOptional: boolean = false;

  abstract _parse(input: unknown, path: (string | number)[]): { success: true, value: T } | { success: false, issues: ValidationErrorIssue[] };

  public parse(input: unknown): T {
    const result = this._parse(input, []);
    if (result.success) {
      return result.value;
    }
    throw new ValidationError(result.issues);
  }

  public optional(): this {
    this.isOptional = true;
    return this;
  }

  get _type(): T { return null as any; }
}

// --- String Schema ---
class KotobaString extends KotobaSchema<string> {
  private checks: ({ type: 'email' | 'min'; message?: string; } | { type: 'min'; length: number; message?: string; })[] = [];

  email(message?: string) { this.checks.push({ type: 'email', message: message || 'Invalid email format' }); return this; }
  min(length: number, message?: string) { this.checks.push({ type: 'min', length, message: message || `Must be at least ${length} characters long` }); return this; }

  _parse(input: unknown, path: (string | number)[]) {
    if (this.isOptional && (input === undefined || input === null)) {
      return { success: true, value: input as any };
    }
    if (typeof input !== 'string') {
      return { success: false, issues: [{ path, message: 'Expected a string' }] };
    }

    const issues: ValidationErrorIssue[] = [];
    for (const check of this.checks) {
      if (check.type === 'min' && input.length < check.length) {
        issues.push({ path, message: check.message! });
      }
      if (check.type === 'email' && !/.+@.+\..+/.test(input)) {
        issues.push({ path, message: check.message! });
      }
    }
    return issues.length > 0 ? { success: false, issues } : { success: true, value: input };
  }
}

// --- Number Schema ---
class KotobaNumber extends KotobaSchema<number> {
  private checks: ({ type: 'int' | 'min'; message?: string; } | { type: 'min'; value: number; message?: string; })[] = [];
  
  int(message?: string) { this.checks.push({ type: 'int', message: message || 'Expected an integer' }); return this; }
  min(value: number, message?: string) { this.checks.push({ type: 'min', value, message: message || `Must be at least ${value}` }); return this; }

  _parse(input: unknown, path: (string | number)[]) {
    if (this.isOptional && (input === undefined || input === null)) {
      return { success: true, value: input as any };
    }
    if (typeof input !== 'number') {
      return { success: false, issues: [{ path, message: 'Expected a number' }] };
    }

    const issues: ValidationErrorIssue[] = [];
    for (const check of this.checks) {
        if (check.type === 'int' && !Number.isInteger(input)) {
            issues.push({ path, message: check.message! });
        }
        if (check.type === 'min' && input < check.value) {
            issues.push({ path, message: check.message! });
        }
    }
    return issues.length > 0 ? { success: false, issues } : { success: true, value: input };
  }
}

// --- Object Schema ---
type Shape = Record<string, KotobaSchema<any>>;
class KotobaObject<T extends Shape> extends KotobaSchema<{ [K in keyof T]: T[K]['_type'] }> {
  constructor(private shape: T) { super(); }

  _parse(input: unknown, path: (string | number)[]) {
    if (this.isOptional && (input === undefined || input === null)) {
      return { success: true, value: input as any };
    }
    if (typeof input !== 'object' || input === null) {
      return { success: false, issues: [{ path, message: 'Expected an object' }] };
    }

    const issues: ValidationErrorIssue[] = [];
    const output: any = {};
    
    for (const key in this.shape) {
      const fieldSchema = this.shape[key];
      const fieldValue = (input as any)[key];
      const result = fieldSchema._parse(fieldValue, [...path, key]);

      if (result.success) {
        if (result.value !== undefined) {
          output[key] = result.value;
        }
      } else {
        issues.push(...result.issues);
      }
    }

    return issues.length > 0 ? { success: false, issues } : { success: true, value: output };
  }
}

// --- Public Interface ---
export const k = {
  string: () => new KotobaString(),
  number: () => new KotobaNumber(),
  object: <T extends Shape>(shape: T) => new KotobaObject(shape),
};

// --- Type Inference ---
export type infer<T extends KotobaSchema<any>> = T['_type'];

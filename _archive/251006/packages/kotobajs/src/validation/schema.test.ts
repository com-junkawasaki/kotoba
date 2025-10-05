import { describe, it, expect } from 'vitest';
import { k, ValidationError } from './schema';

describe('Kotoba Validation Schema', () => {
  describe('k.string()', () => {
    it('should parse a valid string', () => {
      const schema = k.string();
      expect(schema.parse('hello world')).toBe('hello world');
    });

    it('should throw a ValidationError for non-string input', () => {
      const schema = k.string();
      expect(() => schema.parse(123)).toThrow(ValidationError);
      expect(() => schema.parse(null)).toThrow(ValidationError);
    });

    it('should validate min length with a custom message', () => {
      const schema = k.string().min(5, 'Too short!');
      expect(() => schema.parse('hi')).toThrow('Too short!');
    });

    it('should validate email format', () => {
      const schema = k.string().email();
      expect(schema.parse('test@example.com')).toBe('test@example.com');
      expect(() => schema.parse('not-a-valid-email')).toThrow('Invalid email format');
    });
  });

  describe('k.object()', () => {
    const userSchema = k.object({
      name: k.string().min(1),
      email: k.string().email(),
      age: k.number().min(0).optional(),
    });

    it('should parse a valid object', () => {
      const user = { name: 'Alice', email: 'alice@example.com', age: 30 };
      expect(userSchema.parse(user)).toEqual(user);
    });

    it('should correctly handle optional fields', () => {
      const user = { name: 'Bob', email: 'bob@example.com' };
      expect(userSchema.parse(user)).toEqual(user);
    });

    it('should throw ValidationError with multiple issues', () => {
      const invalidUser = { name: '', email: 'not-an-email', age: -5 };
      try {
        userSchema.parse(invalidUser);
        // This should not be reached
        expect(true).toBe(false);
      } catch (error) {
        expect(error).toBeInstanceOf(ValidationError);
        const issues = (error as ValidationError).issues;
        expect(issues).toHaveLength(3);
        expect(issues).toEqual(
          expect.arrayContaining([
            expect.objectContaining({ path: ['name'], message: 'Must be at least 1 characters long' }),
            expect.objectContaining({ path: ['email'], message: 'Invalid email format' }),
            expect.objectContaining({ path: ['age'], message: 'Must be at least 0' }),
          ])
        );
      }
    });

    it('should throw if a required field is null or undefined', () => {
        expect(() => userSchema.parse({ name: 'Valid', email: null })).toThrow();
        expect(() => userSchema.parse({ name: undefined, email: 'valid@e.mail' })).toThrow();
    });
  });
});

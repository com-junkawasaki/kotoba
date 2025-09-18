import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true, // Allows using `describe`, `it`, `expect` without importing them.
    environment: 'node', // Specifies the test environment.
    coverage: {
      provider: 'v8', // or 'istanbul'
      reporter: ['text', 'json', 'html'],
    },
  },
});

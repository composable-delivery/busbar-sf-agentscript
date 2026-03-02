import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    testTimeout: 15000,
    include: ['test/**/*.test.ts'],
    globalSetup: ['test/helpers/global-setup.ts'],
    setupFiles: ['test/helpers/setup.ts'],
  },
});

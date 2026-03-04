import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    globals: false,
    poolOptions: {
      forks: {
        execArgv: ['--experimental-wasm-type-reflection'],
      },
    },
  },
});

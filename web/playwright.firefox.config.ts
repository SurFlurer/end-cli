import { defineConfig } from '@playwright/test';

const e2eBaseUrl = process.env.PW_E2E_BASE_URL ?? 'http://127.0.0.1:4173';

export default defineConfig({
  testDir: 'e2e',
  timeout: 60_000,
  expect: {
    timeout: 10_000
  },
  use: {
    baseURL: e2eBaseUrl,
    headless: true
  },
  projects: [
    {
      name: 'firefox',
      use: {
        browserName: 'firefox'
      }
    }
  ]
});

import { defineConfig, devices } from '@playwright/test';

const only = process.env.ENGINES?.split(',');
const engines = [
  { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
  { name: 'webkit', use: { ...devices['Desktop Safari'] } },
].filter((p) => !only || only.includes(p.name));

export default defineConfig({
  testDir: 'tests',
  outputDir: 'results',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'report' }]],
  use: { baseURL: 'http://127.0.0.1:4317', trace: 'off' },
  projects: engines,
  webServer: { command: 'node serve.mjs 4317', url: 'http://127.0.0.1:4317/newtab/', reuseExistingServer: !process.env.CI },
});

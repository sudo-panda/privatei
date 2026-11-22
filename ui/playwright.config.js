import { defineConfig, devices } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  fullyParallel: true,
  reporter: process.env.CI ? 'list' : 'line',
  use: { baseURL: 'http://127.0.0.1:4173', viewport: { width: 480, height: 900 } },
  projects: [
    { name: 'webkit', use: { ...devices['Desktop Safari'], viewport: { width: 480, height: 900 } } },
    { name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 480, height: 900 } } },
  ],
  webServer: {
    command: 'npx vite preview --config ui/vite.config.js --mode mock --port 4173 --strictPort --host 127.0.0.1',
    cwd: root,
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: !process.env.CI,
  },
});

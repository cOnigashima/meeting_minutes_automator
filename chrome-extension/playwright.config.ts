/**
 * Playwright Configuration for Chrome Extension E2E Testing
 *
 * Chrome拡張機能のE2Eテスト設定。
 * 拡張機能をロードした状態でChromiumを起動する。
 */

import { defineConfig, devices } from '@playwright/test';
import * as path from 'path';

const EXTENSION_PATH = path.join(__dirname, 'dist');

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false, // Chrome extension tests must run sequentially
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Single worker for extension tests
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
  ],
  timeout: 60000, // 60s per test (OAuth flows can be slow)

  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    {
      name: 'chromium-extension',
      use: {
        ...devices['Desktop Chrome'],
        // Chrome extension requires persistent context
        // This is configured in the test fixtures
      },
    },
  ],

  // Build extension before running tests
  webServer: {
    command: 'npm run build',
    cwd: __dirname,
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});

// Export extension path for use in fixtures
export { EXTENSION_PATH };

// The UI loop's browser gates (story:dashboard-ui-loop): L2 (ux/layout.spec.ts) and L3
// (ux/screen-only.spec.ts) in headless Chromium against the committed build, dist/, which
// `vite preview` serves on 127.0.0.1. Every request of the page is answered from the fixtures
// (ux/hermetic.ts): no network, no keys.
import { defineConfig, devices } from '@playwright/test';

const PORT = 4319;

export default defineConfig({
  testDir: 'ux',
  testMatch: /.*\.spec\.ts$/,
  outputDir: 'ux/.out/results',
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  reporter: [['dot']],
  timeout: 30_000,
  use: {
    ...devices['Desktop Chrome'],
    baseURL: `http://127.0.0.1:${PORT}`,
    deviceScaleFactor: 1,
    locale: 'en-US',
    timezoneId: 'UTC',
    serviceWorkers: 'block',
  },
  webServer: {
    command: `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});

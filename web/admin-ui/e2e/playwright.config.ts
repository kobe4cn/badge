import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E 测试配置
 */
export default defineConfig({
  testDir: './specs',

  // 并行运行
  fullyParallel: true,

  // CI 模式下禁止 .only
  forbidOnly: !!process.env.CI,

  // 重试次数
  retries: process.env.CI ? 2 : 0,

  // 并发工作进程
  workers: process.env.CI ? 1 : undefined,

  // 报告器配置
  reporter: [
    ['list'],
    ['html', { outputFolder: 'playwright-report', open: 'never' }],
    ['json', { outputFile: 'test-results/results.json' }],
    ['junit', { outputFile: 'test-results/junit.xml' }],
  ],

  // 全局配置
  use: {
    baseURL: process.env.BASE_URL || 'http://localhost:3001',

    // 截图配置
    screenshot: 'only-on-failure',

    // 视频配置
    video: process.env.CI ? 'retain-on-failure' : 'off',

    // 追踪配置
    trace: 'retain-on-failure',

    // 超时
    actionTimeout: 10000,
    navigationTimeout: 30000,
  },

  // 全局超时
  timeout: 60000,

  // 期望超时
  expect: {
    timeout: 10000,
  },

  // 浏览器配置
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    // 移动端测试
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 12'] },
    },
  ],

  // 开发服务器
  webServer: process.env.CI ? undefined : {
    command: 'npm run dev',
    url: 'http://localhost:3001',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },

  // 输出目录
  outputDir: 'test-results/',
});

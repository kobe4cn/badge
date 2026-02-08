import { defineConfig, devices } from '@playwright/test';

// 所有 project 共享的 baseURL，确保 page.goto('/path') 能正确解析
const baseURL = process.env.BASE_URL || 'http://localhost:3001';

// CI 默认同时跑 chromium 和 firefox 以保证跨浏览器兼容；本地只跑 chromium 加快反馈
const enabledProjects = (
  process.env.PLAYWRIGHT_PROJECTS || (process.env.CI ? 'chromium,firefox' : 'chromium')
)
  .split(',')
  .map((s) => s.trim());

// 所有可用的浏览器 project 定义
const allProjects = [
  {
    name: 'chromium',
    use: { ...devices['Desktop Chrome'], baseURL },
  },
  {
    name: 'firefox',
    use: { ...devices['Desktop Firefox'], baseURL },
  },
  {
    name: 'webkit',
    use: { ...devices['Desktop Safari'], baseURL },
  },
  {
    name: 'mobile-chrome',
    use: { ...devices['Pixel 5'], baseURL },
  },
  {
    name: 'mobile-safari',
    use: { ...devices['iPhone 12'], baseURL },
  },
];

/**
 * Playwright E2E 测试配置
 */
export default defineConfig({
  testDir: './specs',

  // 忽略 vitest 单元测试文件，避免模块解析冲突
  testIgnore: ['**/__tests__/**', '**/*.test.ts', '!**/*.spec.ts'],

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
    baseURL,

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

  // 根据 PLAYWRIGHT_PROJECTS 环境变量筛选要运行的浏览器
  projects: allProjects.filter((p) => enabledProjects.includes(p.name)),

  // 开发服务器（集成测试模式下禁用 mock）
  webServer: process.env.CI ? undefined : {
    command: process.env.VITE_DISABLE_MOCK === 'true' ? 'npm run dev:real' : 'npm run dev',
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },

  // 输出目录
  outputDir: 'test-results/',
});

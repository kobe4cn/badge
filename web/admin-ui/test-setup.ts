// Vitest 测试环境设置
// 此文件仅在 vitest 运行时通过 setupFiles 配置加载
// Playwright 不会加载此文件

// 确保只在 vitest 环境中运行
// VITEST 环境变量由 vitest 自动设置
if (process.env.VITEST) {
  // 使用动态 import 加载 jest-dom matchers
  // 这避免了在 Playwright 运行时触发 vitest 模块加载
  import('@testing-library/jest-dom/vitest');
}

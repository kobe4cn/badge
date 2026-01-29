/**
 * 环境变量配置
 *
 * 统一导出 Vite 环境变量，提供类型安全和默认值
 */

/**
 * 环境配置接口
 */
interface EnvConfig {
  /** API 基础地址 */
  apiBaseUrl: string;
  /** 应用标题 */
  appTitle: string;
  /** 是否显示开发工具 */
  showDevtools: boolean;
  /** 当前环境 */
  mode: 'development' | 'production';
  /** 是否为开发环境 */
  isDev: boolean;
  /** 是否为生产环境 */
  isProd: boolean;
}

/**
 * 从 Vite 环境变量中读取配置
 *
 * 使用 import.meta.env 访问 VITE_ 前缀的环境变量
 */
export const env: EnvConfig = {
  apiBaseUrl: import.meta.env.VITE_API_BASE_URL || '/api',
  appTitle: import.meta.env.VITE_APP_TITLE || '徽章管理系统',
  showDevtools: import.meta.env.VITE_SHOW_DEVTOOLS === 'true',
  mode: import.meta.env.MODE as 'development' | 'production',
  isDev: import.meta.env.DEV,
  isProd: import.meta.env.PROD,
};

export default env;

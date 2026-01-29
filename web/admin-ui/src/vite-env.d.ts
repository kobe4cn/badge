/// <reference types="vite/client" />

/**
 * Vite 环境变量类型定义
 *
 * 为 import.meta.env 提供类型安全
 */
interface ImportMetaEnv {
  /** API 基础地址 */
  readonly VITE_API_BASE_URL: string;
  /** 应用标题 */
  readonly VITE_APP_TITLE: string;
  /** 是否显示开发工具 */
  readonly VITE_SHOW_DEVTOOLS: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

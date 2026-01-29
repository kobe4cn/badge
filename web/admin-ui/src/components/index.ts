/**
 * 通用组件统一导出
 *
 * 遵循按需导出原则，组件在后续任务中逐步实现
 */

// Layout 组件
export { default as AdminLayout, AvatarDropdown, PageTransition } from './Layout';

// Charts 组件
export { LineChart, PieChart, BarChart } from './Charts';
export type {
  LineChartProps,
  PieChartProps,
  PieChartDataItem,
  BarChartProps,
  BarChartDataItem,
} from './Charts';

// Loading 组件
export { PageLoading } from './Loading';

// ErrorBoundary 组件
export { default as ErrorBoundary } from './ErrorBoundary';

// Auth 组件
export { AuthGuard, isAuthenticated, hasPermission, getUserRoles } from './Auth';

/**
 * 认证组件统一导出
 */

export { default as AuthGuard } from './AuthGuard';
export { isAuthenticated, hasPermission, getUserRoles } from './authUtils';

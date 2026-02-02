/**
 * 认证工具函数
 *
 * 提供认证状态检查和权限验证相关的工具函数
 * 与 authStore 集成，统一认证状态管理
 */

import { getAuthState } from '@/stores/authStore';

/**
 * 权限配置
 */
interface Permission {
  /** 路由路径 */
  path: string;
  /** 所需角色 */
  roles?: string[];
}

/**
 * 路由权限映射表
 *
 * 配置各路由所需的角色，未配置的路由默认允许所有已登录用户访问
 */
const ROUTE_PERMISSIONS: Permission[] = [
  // 示例：管理员专属功能
  // { path: '/settings', roles: ['admin'] },
];

/**
 * 检查用户是否已登录
 *
 * 从 authStore 获取认证状态，兼容 localStorage 中的 token
 */
export function isAuthenticated(): boolean {
  const { isAuthenticated: storeAuth } = getAuthState();

  // 优先使用 store 中的状态
  if (storeAuth) {
    return true;
  }

  // 降级检查 localStorage（用于页面刷新后的首次检查）
  const token = localStorage.getItem('auth_token');
  return !!token;
}

/**
 * 获取当前用户角色
 */
export function getUserRoles(): string[] {
  const { user } = getAuthState();

  if (user?.role) {
    return [user.role];
  }

  // 降级从 localStorage 读取
  try {
    const userInfo = localStorage.getItem('user_info');
    if (userInfo) {
      const parsed = JSON.parse(userInfo);
      return parsed.role ? [parsed.role] : ['user'];
    }
  } catch {
    // 解析失败返回空数组
  }

  return [];
}

/**
 * 检查用户是否有指定路由的访问权限
 */
export function hasPermission(path: string): boolean {
  const userRoles = getUserRoles();
  const routePermission = ROUTE_PERMISSIONS.find((p) => path.startsWith(p.path));

  // 未配置权限的路由，所有已登录用户可访问
  if (!routePermission || !routePermission.roles) {
    return true;
  }

  // 检查用户是否具有所需角色
  return routePermission.roles.some((role) => userRoles.includes(role));
}

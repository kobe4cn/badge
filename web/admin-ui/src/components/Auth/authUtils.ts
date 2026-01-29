/**
 * 认证工具函数
 *
 * 提供认证状态检查和权限验证相关的工具函数
 */

import { env } from '@/config';

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
 */
export function isAuthenticated(): boolean {
  // 开发模式下默认已登录，方便调试
  if (env.isDev) {
    return true;
  }

  const token = localStorage.getItem('auth_token');
  return !!token;
}

/**
 * 获取当前用户角色
 */
export function getUserRoles(): string[] {
  // 开发模式下模拟管理员角色
  if (env.isDev) {
    return ['admin', 'user'];
  }

  try {
    const userInfo = localStorage.getItem('user_info');
    if (userInfo) {
      const parsed = JSON.parse(userInfo);
      return parsed.roles || ['user'];
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

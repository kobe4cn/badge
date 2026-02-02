/**
 * 认证工具函数
 *
 * 提供认证状态检查和权限验证相关的工具函数
 * 与 authStore 集成，统一认证状态管理
 */

import { getAuthState } from '@/stores/authStore';

/**
 * 路由权限配置
 */
interface RoutePermission {
  /** 路由路径 */
  path: string;
  /** 所需权限码（满足任一即可） */
  permissions?: string[];
  /** 所需角色（满足任一即可） */
  roles?: string[];
}

/**
 * 路由权限映射表
 *
 * 配置各路由所需的权限或角色，未配置的路由默认允许所有已登录用户访问
 */
const ROUTE_PERMISSIONS: RoutePermission[] = [
  // 系统管理模块
  { path: '/system/users', permissions: ['system:user:view'] },
  { path: '/system/roles', permissions: ['system:role:view'] },
  // 徽章管理模块
  { path: '/badges', permissions: ['badge:view'] },
  { path: '/badges/categories', permissions: ['badge:category:view'] },
  { path: '/badges/series', permissions: ['badge:series:view'] },
  { path: '/badges/definitions', permissions: ['badge:view'] },
  // 规则管理模块
  { path: '/rules', permissions: ['rule:view'] },
  // 发放管理模块
  { path: '/grants', permissions: ['grant:view'] },
  // 会员视图模块
  { path: '/members', permissions: ['user:view'] },
  // 权益管理模块
  { path: '/benefits', permissions: ['benefit:view'] },
  // 兑换管理模块
  { path: '/redemptions', permissions: ['redemption:view'] },
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
 * 获取当前用户角色列表
 */
export function getUserRoles(): string[] {
  const { user } = getAuthState();

  if (user?.roles && user.roles.length > 0) {
    return user.roles;
  }

  // 降级从 localStorage 读取
  try {
    const userInfo = localStorage.getItem('user_info');
    if (userInfo) {
      const parsed = JSON.parse(userInfo);
      if (parsed.roles && Array.isArray(parsed.roles)) {
        return parsed.roles;
      }
    }
  } catch {
    // 解析失败返回空数组
  }

  return [];
}

/**
 * 获取当前用户权限列表
 */
export function getUserPermissions(): string[] {
  const { user } = getAuthState();

  if (user?.permissions && user.permissions.length > 0) {
    return user.permissions;
  }

  // 降级从 localStorage 读取
  try {
    const userInfo = localStorage.getItem('user_info');
    if (userInfo) {
      const parsed = JSON.parse(userInfo);
      if (parsed.permissions && Array.isArray(parsed.permissions)) {
        return parsed.permissions;
      }
    }
  } catch {
    // 解析失败返回空数组
  }

  return [];
}

/**
 * 检查用户是否是管理员
 *
 * admin 角色拥有所有权限
 */
export function isAdmin(): boolean {
  const roles = getUserRoles();
  return roles.includes('admin');
}

/**
 * 检查用户是否拥有指定权限
 *
 * @param permission 权限码
 */
export function hasPermissionCode(permission: string): boolean {
  // 管理员拥有所有权限
  if (isAdmin()) {
    return true;
  }

  const permissions = getUserPermissions();
  return permissions.includes(permission);
}

/**
 * 检查用户是否拥有任一指定权限
 *
 * @param permissions 权限码列表
 */
export function hasAnyPermission(permissions: string[]): boolean {
  // 管理员拥有所有权限
  if (isAdmin()) {
    return true;
  }

  const userPermissions = getUserPermissions();
  return permissions.some((p) => userPermissions.includes(p));
}

/**
 * 检查用户是否拥有所有指定权限
 *
 * @param permissions 权限码列表
 */
export function hasAllPermissions(permissions: string[]): boolean {
  // 管理员拥有所有权限
  if (isAdmin()) {
    return true;
  }

  const userPermissions = getUserPermissions();
  return permissions.every((p) => userPermissions.includes(p));
}

/**
 * 检查用户是否有指定路由的访问权限
 */
export function hasRoutePermission(path: string): boolean {
  // 管理员可以访问所有路由
  if (isAdmin()) {
    return true;
  }

  const routePermission = ROUTE_PERMISSIONS.find((p) => path.startsWith(p.path));

  // 未配置权限的路由，所有已登录用户可访问
  if (!routePermission) {
    return true;
  }

  // 检查权限码
  if (routePermission.permissions && routePermission.permissions.length > 0) {
    if (hasAnyPermission(routePermission.permissions)) {
      return true;
    }
  }

  // 检查角色
  if (routePermission.roles && routePermission.roles.length > 0) {
    const userRoles = getUserRoles();
    if (routePermission.roles.some((role) => userRoles.includes(role))) {
      return true;
    }
  }

  // 未配置具体要求时，默认拒绝
  if (routePermission.permissions || routePermission.roles) {
    return false;
  }

  return true;
}

/**
 * 旧版兼容：检查用户是否有指定路由的访问权限
 * @deprecated 使用 hasRoutePermission 替代
 */
export const hasPermission = hasRoutePermission;

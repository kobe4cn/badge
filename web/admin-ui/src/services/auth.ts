/**
 * 认证 API 服务
 *
 * 提供登录、登出、获取用户信息等认证相关的 API 调用
 */

import { post, get } from './api';
import type { AdminUser } from '@/stores/authStore';

/**
 * 登录请求参数
 */
export interface LoginRequest {
  username: string;
  password: string;
}

/**
 * 后端登录响应原始格式
 *
 * permissions 和 roles 在 user 对象外部，需要合并到 AdminUser 中
 */
interface LoginResponseRaw {
  token: string;
  user: {
    id: number | string;
    username: string;
    displayName: string;
    email?: string | null;
    avatarUrl?: string | null;
    status?: string;
    lastLoginAt?: string;
    createdAt?: string;
  };
  permissions: string[];
  expiresAt?: number;
}

/**
 * 登录响应数据（标准化后）
 */
export interface LoginResponse {
  token: string;
  user: AdminUser;
}

/**
 * API 基础路径
 */
const AUTH_BASE_URL = '/admin/auth';

/**
 * 用户登录
 *
 * @param username 用户名
 * @param password 密码
 * @returns 登录响应（包含 token 和用户信息）
 */
export async function login(username: string, password: string): Promise<LoginResponse> {
  const raw = await post<LoginResponseRaw>(`${AUTH_BASE_URL}/login`, {
    username,
    password,
  });

  // 将后端扁平结构转换为前端所需的嵌套格式
  const user: AdminUser = {
    id: String(raw.user.id),
    username: raw.user.username,
    displayName: raw.user.displayName,
    roles: extractRolesFromPermissions(raw.permissions),
    permissions: raw.permissions || [],
    avatar: raw.user.avatarUrl || undefined,
  };

  return { token: raw.token, user };
}

/**
 * 从权限列表中推导角色
 *
 * 后端 JWT 中包含 roles，但登录 API 返回的是 permissions 列表
 * 通过 system 模块权限判断是否是管理员
 */
function extractRolesFromPermissions(permissions: string[]): string[] {
  const hasSystemWrite = permissions.some(p => p.startsWith('system:') && p.endsWith(':write'));
  const hasGrantWrite = permissions.some(p => p.startsWith('grant:') && p.endsWith(':write'));
  const hasAnyWrite = permissions.some(p => p.endsWith(':write'));

  if (hasSystemWrite) return ['admin'];
  if (hasGrantWrite || hasAnyWrite) return ['operator'];
  return ['viewer'];
}

/**
 * 用户登出
 *
 * 调用后端 API 使当前 token 失效
 */
export async function logout(): Promise<void> {
  return post<void>(`${AUTH_BASE_URL}/logout`);
}

/**
 * 后端 /me 接口的原始响应格式
 */
interface MeResponseRaw {
  user: {
    id: number | string;
    username: string;
    displayName: string;
    email?: string | null;
    avatarUrl?: string | null;
    status?: string;
  };
  permissions: string[];
  roles: Array<{ id: number; code: string; name: string; description?: string }>;
}

/**
 * 获取当前用户信息
 *
 * 通过 token 获取当前登录用户的详细信息，将后端格式转换为前端格式
 */
export async function getCurrentUser(): Promise<AdminUser> {
  const raw = await get<MeResponseRaw>(`${AUTH_BASE_URL}/me`);

  return {
    id: String(raw.user.id),
    username: raw.user.username,
    displayName: raw.user.displayName,
    roles: raw.roles.map(r => r.code),
    permissions: raw.permissions || [],
    avatar: raw.user.avatarUrl || undefined,
  };
}

/**
 * 刷新 token
 *
 * 在 token 即将过期时调用，获取新的 token
 */
export async function refreshToken(): Promise<{ token: string }> {
  return post<{ token: string }>(`${AUTH_BASE_URL}/refresh`);
}

/**
 * 验证 token 是否有效
 *
 * 用于页面加载时检查登录状态
 */
export async function validateToken(): Promise<boolean> {
  try {
    await getCurrentUser();
    return true;
  } catch {
    return false;
  }
}

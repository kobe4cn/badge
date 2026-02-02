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
 * 登录响应数据
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
  return post<LoginResponse>(`${AUTH_BASE_URL}/login`, {
    username,
    password,
  });
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
 * 获取当前用户信息
 *
 * 通过 token 获取当前登录用户的详细信息
 */
export async function getCurrentUser(): Promise<AdminUser> {
  return get<AdminUser>(`${AUTH_BASE_URL}/me`);
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

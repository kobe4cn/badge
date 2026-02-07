/**
 * 系统管理 API 服务
 *
 * 提供用户、角色、权限管理相关的 API 调用
 */

import { get, post, put, del, patch, getList } from './api';
import type { PaginatedResponse } from '@/types';

// =============== 类型定义 ===============

/**
 * 系统用户
 */
export interface SystemUser {
  id: number;
  username: string;
  displayName: string;
  email?: string;
  phone?: string;
  status: 'ACTIVE' | 'DISABLED' | 'LOCKED';
  lastLoginAt?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 用户详情（含角色和权限）
 */
export interface SystemUserDetail extends SystemUser {
  roles: RoleInfo[];
  permissions: string[];
}

/**
 * 角色信息
 */
export interface RoleInfo {
  id: number;
  code: string;
  name: string;
}

/**
 * 角色
 */
export interface SystemRole {
  id: number;
  code: string;
  name: string;
  description?: string;
  userCount: number;
  permissionCount: number;
  builtIn: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 角色详情（含权限列表）
 */
export interface SystemRoleDetail extends Omit<SystemRole, 'userCount' | 'permissionCount'> {
  permissions: PermissionInfo[];
}

/**
 * 权限信息
 */
export interface PermissionInfo {
  id: number;
  code: string;
  name: string;
  module: string;
  description?: string;
}

/**
 * 权限树节点
 */
export interface PermissionTreeNode {
  module: string;
  moduleName: string;
  permissions: PermissionInfo[];
}

/**
 * API Key 列表项
 */
export interface ApiKeyDto {
  id: number;
  name: string;
  keyPrefix: string;
  permissions: string[];
  rateLimit?: number;
  expiresAt?: string;
  enabled: boolean;
  lastUsedAt?: string;
  createdAt: string;
}

/**
 * 创建 API Key 响应（含完整密钥，仅返回一次）
 */
export interface CreateApiKeyResponse {
  id: number;
  name: string;
  apiKey: string;
  permissions: string[];
  rateLimit?: number;
  expiresAt?: string;
  createdAt: string;
}

/**
 * 创建 API Key 请求
 */
export interface CreateApiKeyRequest {
  name: string;
  permissions: string[];
  rateLimit?: number;
  expiresAt?: string;
}

/**
 * API Key 列表查询参数
 */
export interface ApiKeyListParams {
  page?: number;
  pageSize?: number;
  name?: string;
  [key: string]: unknown;
}

// =============== 请求参数类型 ===============

export interface UserListParams {
  page?: number;
  pageSize?: number;
  username?: string;
  status?: 'ACTIVE' | 'DISABLED' | 'LOCKED';
  roleId?: number;
  [key: string]: unknown;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  displayName: string;
  email?: string;
  phone?: string;
  roleIds: number[];
}

export interface UpdateUserRequest {
  displayName?: string;
  email?: string;
  phone?: string;
  status?: 'ACTIVE' | 'DISABLED';
  roleIds?: number[];
}

export interface ResetPasswordRequest {
  newPassword: string;
}

export interface RoleListParams {
  page?: number;
  pageSize?: number;
  name?: string;
  [key: string]: unknown;
}

export interface CreateRoleRequest {
  code: string;
  name: string;
  description?: string;
  permissionIds: number[];
}

export interface UpdateRoleRequest {
  name?: string;
  description?: string;
  permissionIds?: number[];
}

// =============== API 基础路径 ===============

const SYSTEM_BASE_URL = '/admin/system';

// =============== 用户管理 API ===============

/**
 * 获取用户列表
 */
export async function listUsers(params?: UserListParams): Promise<PaginatedResponse<SystemUser>> {
  return getList<SystemUser>(`${SYSTEM_BASE_URL}/users`, params);
}

/**
 * 获取用户详情
 */
export async function getUser(id: number): Promise<SystemUserDetail> {
  return get<SystemUserDetail>(`${SYSTEM_BASE_URL}/users/${id}`);
}

/**
 * 创建用户
 */
export async function createUser(data: CreateUserRequest): Promise<SystemUserDetail> {
  return post<SystemUserDetail>(`${SYSTEM_BASE_URL}/users`, data);
}

/**
 * 更新用户
 */
export async function updateUser(id: number, data: UpdateUserRequest): Promise<SystemUserDetail> {
  return put<SystemUserDetail>(`${SYSTEM_BASE_URL}/users/${id}`, data);
}

/**
 * 删除用户
 */
export async function deleteUser(id: number): Promise<void> {
  return del(`${SYSTEM_BASE_URL}/users/${id}`);
}

/**
 * 重置用户密码
 */
export async function resetUserPassword(id: number, data: ResetPasswordRequest): Promise<void> {
  return post(`${SYSTEM_BASE_URL}/users/${id}/reset-password`, data);
}

// =============== 角色管理 API ===============

/**
 * 获取角色列表
 */
export async function listRoles(params?: RoleListParams): Promise<PaginatedResponse<SystemRole>> {
  return getList<SystemRole>(`${SYSTEM_BASE_URL}/roles`, params);
}

/**
 * 获取角色详情
 */
export async function getRole(id: number): Promise<SystemRoleDetail> {
  return get<SystemRoleDetail>(`${SYSTEM_BASE_URL}/roles/${id}`);
}

/**
 * 创建角色
 */
export async function createRole(data: CreateRoleRequest): Promise<SystemRoleDetail> {
  return post<SystemRoleDetail>(`${SYSTEM_BASE_URL}/roles`, data);
}

/**
 * 更新角色
 */
export async function updateRole(id: number, data: UpdateRoleRequest): Promise<SystemRoleDetail> {
  return put<SystemRoleDetail>(`${SYSTEM_BASE_URL}/roles/${id}`, data);
}

/**
 * 删除角色
 */
export async function deleteRole(id: number): Promise<void> {
  return del(`${SYSTEM_BASE_URL}/roles/${id}`);
}

// =============== 权限管理 API ===============

/**
 * 获取所有权限列表
 */
export async function listPermissions(): Promise<PermissionInfo[]> {
  return get<PermissionInfo[]>(`${SYSTEM_BASE_URL}/permissions`);
}

/**
 * 获取权限树
 */
export async function getPermissionTree(): Promise<PermissionTreeNode[]> {
  return get<PermissionTreeNode[]>(`${SYSTEM_BASE_URL}/permissions/tree`);
}

// =============== API Key 管理 API ===============

/**
 * 获取 API Key 列表
 */
export async function listApiKeys(params?: ApiKeyListParams): Promise<PaginatedResponse<ApiKeyDto>> {
  return getList<ApiKeyDto>(`${SYSTEM_BASE_URL}/api-keys`, params);
}

/**
 * 创建 API Key
 */
export async function createApiKey(data: CreateApiKeyRequest): Promise<CreateApiKeyResponse> {
  return post<CreateApiKeyResponse>(`${SYSTEM_BASE_URL}/api-keys`, data);
}

/**
 * 删除 API Key
 */
export async function deleteApiKey(id: number): Promise<void> {
  return del(`${SYSTEM_BASE_URL}/api-keys/${id}`);
}

/**
 * 重新生成 API Key
 */
export async function regenerateApiKey(id: number): Promise<CreateApiKeyResponse> {
  return post<CreateApiKeyResponse>(`${SYSTEM_BASE_URL}/api-keys/${id}/regenerate`);
}

/**
 * 切换 API Key 启用状态
 */
export async function toggleApiKeyStatus(id: number, enabled: boolean): Promise<void> {
  return patch(`${SYSTEM_BASE_URL}/api-keys/${id}/status`, { enabled });
}


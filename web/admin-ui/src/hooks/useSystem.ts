/**
 * 系统管理 React Query Hooks
 *
 * 封装用户、角色、权限管理相关的数据查询和变更操作
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  listUsers,
  getUser,
  createUser,
  updateUser,
  deleteUser,
  resetUserPassword,
  listRoles,
  getRole,
  createRole,
  updateRole,
  deleteRole,
  listPermissions,
  getPermissionTree,
  listApiKeys,
  createApiKey,
  deleteApiKey,
  regenerateApiKey,
  toggleApiKeyStatus,
  type UserListParams,
  type CreateUserRequest,
  type UpdateUserRequest,
  type ResetPasswordRequest,
  type RoleListParams,
  type CreateRoleRequest,
  type UpdateRoleRequest,
  type ApiKeyListParams,
  type CreateApiKeyRequest,
} from '@/services/system';

/**
 * 缓存 key 常量
 */
export const SYSTEM_QUERY_KEYS = {
  // 用户
  users: ['system', 'users'] as const,
  userLists: () => [...SYSTEM_QUERY_KEYS.users, 'list'] as const,
  userList: (params: UserListParams) => [...SYSTEM_QUERY_KEYS.userLists(), params] as const,
  userDetails: () => [...SYSTEM_QUERY_KEYS.users, 'detail'] as const,
  userDetail: (id: number) => [...SYSTEM_QUERY_KEYS.userDetails(), id] as const,
  // 角色
  roles: ['system', 'roles'] as const,
  roleLists: () => [...SYSTEM_QUERY_KEYS.roles, 'list'] as const,
  roleList: (params: RoleListParams) => [...SYSTEM_QUERY_KEYS.roleLists(), params] as const,
  roleDetails: () => [...SYSTEM_QUERY_KEYS.roles, 'detail'] as const,
  roleDetail: (id: number) => [...SYSTEM_QUERY_KEYS.roleDetails(), id] as const,
  // 权限
  permissions: ['system', 'permissions'] as const,
  permissionTree: ['system', 'permissions', 'tree'] as const,
  // API Key
  apiKeys: ['system', 'apiKeys'] as const,
  apiKeyLists: () => [...SYSTEM_QUERY_KEYS.apiKeys, 'list'] as const,
  apiKeyList: (params: ApiKeyListParams) => [...SYSTEM_QUERY_KEYS.apiKeyLists(), params] as const,
};

// =============== 用户管理 Hooks ===============

/**
 * 查询用户列表
 */
export function useUserList(params: UserListParams, enabled = true) {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.userList(params),
    queryFn: () => listUsers(params),
    enabled,
  });
}

/**
 * 查询用户详情
 */
export function useUserDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.userDetail(id),
    queryFn: () => getUser(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 创建用户
 */
export function useCreateUser() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateUserRequest) => createUser(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.userLists() });
      message.success('用户创建成功');
    },
    onError: () => {
      message.error('用户创建失败');
    },
  });
}

/**
 * 更新用户
 */
export function useUpdateUser() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateUserRequest }) => updateUser(id, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.userLists() });
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.userDetail(variables.id) });
      message.success('用户更新成功');
    },
    onError: () => {
      message.error('用户更新失败');
    },
  });
}

/**
 * 删除用户
 */
export function useDeleteUser() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteUser(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.userLists() });
      message.success('用户删除成功');
    },
    onError: () => {
      message.error('用户删除失败');
    },
  });
}

/**
 * 重置用户密码
 */
export function useResetPassword() {
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: ResetPasswordRequest }) =>
      resetUserPassword(id, data),
    onSuccess: () => {
      message.success('密码重置成功');
    },
    onError: () => {
      message.error('密码重置失败');
    },
  });
}

// =============== 角色管理 Hooks ===============

/**
 * 查询角色列表
 */
export function useRoleList(params: RoleListParams, enabled = true) {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.roleList(params),
    queryFn: () => listRoles(params),
    enabled,
  });
}

/**
 * 查询所有角色（用于下拉选择）
 */
export function useAllRoles() {
  return useQuery({
    queryKey: [...SYSTEM_QUERY_KEYS.roles, 'all'],
    queryFn: async () => {
      const result = await listRoles({ page: 1, pageSize: 100 });
      return result.items;
    },
  });
}

/**
 * 查询角色详情
 */
export function useRoleDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.roleDetail(id),
    queryFn: () => getRole(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 创建角色
 */
export function useCreateRole() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateRoleRequest) => createRole(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.roleLists() });
      message.success('角色创建成功');
    },
    onError: () => {
      message.error('角色创建失败');
    },
  });
}

/**
 * 更新角色
 */
export function useUpdateRole() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateRoleRequest }) => updateRole(id, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.roleLists() });
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.roleDetail(variables.id) });
      message.success('角色更新成功');
    },
    onError: () => {
      message.error('角色更新失败');
    },
  });
}

/**
 * 删除角色
 */
export function useDeleteRole() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteRole(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.roleLists() });
      message.success('角色删除成功');
    },
    onError: () => {
      message.error('角色删除失败');
    },
  });
}

// =============== 权限管理 Hooks ===============

/**
 * 查询所有权限列表
 */
export function usePermissionList() {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.permissions,
    queryFn: listPermissions,
  });
}

/**
 * 查询权限树
 */
export function usePermissionTree() {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.permissionTree,
    queryFn: getPermissionTree,
  });
}

// =============== API Key 管理 Hooks ===============

/**
 * 查询 API Key 列表
 */
export function useApiKeyList(params: ApiKeyListParams, enabled = true) {
  return useQuery({
    queryKey: SYSTEM_QUERY_KEYS.apiKeyList(params),
    queryFn: () => listApiKeys(params),
    enabled,
  });
}

/**
 * 创建 API Key
 */
export function useCreateApiKey() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateApiKeyRequest) => createApiKey(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.apiKeyLists() });
      message.success('API Key 创建成功');
    },
    onError: () => {
      message.error('API Key 创建失败');
    },
  });
}

/**
 * 删除 API Key
 */
export function useDeleteApiKey() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.apiKeyLists() });
      message.success('API Key 删除成功');
    },
    onError: () => {
      message.error('API Key 删除失败');
    },
  });
}

/**
 * 重新生成 API Key
 */
export function useRegenerateApiKey() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => regenerateApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.apiKeyLists() });
      message.success('API Key 已重新生成');
    },
    onError: () => {
      message.error('API Key 重新生成失败');
    },
  });
}

/**
 * 切换 API Key 启用状态
 */
export function useToggleApiKeyStatus() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) =>
      toggleApiKeyStatus(id, enabled),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: SYSTEM_QUERY_KEYS.apiKeyLists() });
      message.success(variables.enabled ? 'API Key 已启用' : 'API Key 已禁用');
    },
    onError: () => {
      message.error('状态切换失败');
    },
  });
}

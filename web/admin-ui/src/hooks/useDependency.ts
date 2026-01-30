/**
 * 徽章依赖关系 React Query Hooks
 *
 * 封装依赖关系相关的数据查询和变更操作，提供缓存管理
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  getDependencies,
  createDependency,
  deleteDependency,
  refreshDependencyCache,
  type CreateDependencyRequest,
} from '@/services/dependency';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const DEPENDENCY_QUERY_KEYS = {
  all: ['dependencies'] as const,
  lists: () => [...DEPENDENCY_QUERY_KEYS.all, 'list'] as const,
  list: (badgeId: string) => [...DEPENDENCY_QUERY_KEYS.lists(), badgeId] as const,
};

/**
 * 查询徽章依赖关系列表
 *
 * @param badgeId - 徽章 ID
 * @param enabled - 是否启用查询
 */
export function useDependencyList(badgeId: string, enabled = true) {
  return useQuery({
    queryKey: DEPENDENCY_QUERY_KEYS.list(badgeId),
    queryFn: () => getDependencies(badgeId),
    enabled: enabled && !!badgeId,
  });
}

/**
 * 创建依赖关系
 *
 * 成功后自动失效列表缓存
 *
 * @param badgeId - 徽章 ID
 */
export function useCreateDependency(badgeId: string) {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateDependencyRequest) => createDependency(badgeId, data),
    onSuccess: () => {
      message.success('依赖关系创建成功');
      queryClient.invalidateQueries({ queryKey: DEPENDENCY_QUERY_KEYS.list(badgeId) });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 删除依赖关系
 *
 * 成功后自动失效列表缓存
 *
 * @param badgeId - 徽章 ID（用于缓存失效）
 */
export function useDeleteDependency(badgeId: string) {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: string) => deleteDependency(id),
    onSuccess: () => {
      message.success('依赖关系删除成功');
      queryClient.invalidateQueries({ queryKey: DEPENDENCY_QUERY_KEYS.list(badgeId) });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '删除失败');
    },
  });
}

/**
 * 刷新依赖缓存
 *
 * 用于管理员手动刷新服务端缓存
 */
export function useRefreshDependencyCache() {
  const { message } = App.useApp();

  return useMutation({
    mutationFn: () => refreshDependencyCache(),
    onSuccess: () => {
      message.success('依赖缓存刷新成功');
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '缓存刷新失败');
    },
  });
}

/**
 * 徽章定义 React Query Hooks
 *
 * 封装徽章相关的数据查询和变更操作，提供缓存管理
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  getBadges,
  getBadge,
  createBadge,
  updateBadge,
  deleteBadge,
  publishBadge,
  unpublishBadge,
  archiveBadge,
  updateBadgeSortOrder,
  type BadgeListParams,
} from '@/services/badge';
import type {
  Badge,
  CreateBadgeRequest,
  UpdateBadgeRequest,
} from '@/types';
import { SERIES_QUERY_KEYS } from './useSeries';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const BADGE_QUERY_KEYS = {
  all: ['badges'] as const,
  lists: () => [...BADGE_QUERY_KEYS.all, 'list'] as const,
  list: (params: BadgeListParams) =>
    [...BADGE_QUERY_KEYS.lists(), params] as const,
  details: () => [...BADGE_QUERY_KEYS.all, 'detail'] as const,
  detail: (id: number) => [...BADGE_QUERY_KEYS.details(), id] as const,
};

/**
 * 查询徽章列表
 *
 * @param params - 分页和筛选参数
 * @param enabled - 是否启用查询
 */
export function useBadgeList(params: BadgeListParams, enabled = true) {
  return useQuery({
    queryKey: BADGE_QUERY_KEYS.list(params),
    queryFn: () => getBadges(params),
    enabled,
  });
}

/**
 * 查询徽章详情
 *
 * @param id - 徽章 ID
 * @param enabled - 是否启用查询
 */
export function useBadgeDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: BADGE_QUERY_KEYS.detail(id),
    queryFn: () => getBadge(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 创建徽章
 *
 * 成功后自动失效列表缓存
 */
export function useCreateBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateBadgeRequest) => createBadge(data),
    onSuccess: () => {
      message.success('徽章创建成功');
      // 失效徽章列表缓存
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
      // 失效系列徽章缓存（系列下徽章数量变化）
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.all });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 更新徽章
 *
 * 成功后更新缓存中的徽章数据
 */
export function useUpdateBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateBadgeRequest }) =>
      updateBadge(id, data),
    onSuccess: (updatedBadge: Badge) => {
      message.success('徽章更新成功');
      // 更新详情缓存
      queryClient.setQueryData(
        BADGE_QUERY_KEYS.detail(updatedBadge.id),
        updatedBadge
      );
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '更新失败');
    },
  });
}

/**
 * 删除徽章
 *
 * 删除前由调用方负责确认弹窗
 */
export function useDeleteBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteBadge(id),
    onSuccess: (_data, id) => {
      message.success('徽章删除成功');
      // 移除详情缓存
      queryClient.removeQueries({ queryKey: BADGE_QUERY_KEYS.detail(id) });
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
      // 失效系列徽章缓存
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.all });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '删除失败');
    },
  });
}

/**
 * 上架徽章
 *
 * 将徽章状态变更为 ACTIVE
 */
export function usePublishBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => publishBadge(id),
    onSuccess: () => {
      message.success('徽章已上架');
      // 失效列表和详情缓存以刷新状态
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.details() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '上架失败');
    },
  });
}

/**
 * 下架徽章
 *
 * 将徽章状态变更为 INACTIVE
 */
export function useUnpublishBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => unpublishBadge(id),
    onSuccess: () => {
      message.success('徽章已下架');
      // 失效列表和详情缓存以刷新状态
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.details() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '下架失败');
    },
  });
}

/**
 * 归档徽章
 *
 * 将徽章状态变更为 ARCHIVED
 */
export function useArchiveBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => archiveBadge(id),
    onSuccess: () => {
      message.success('徽章已归档');
      // 失效列表和详情缓存
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.details() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '归档失败');
    },
  });
}

/**
 * 更新徽章排序
 *
 * 用于排序值的直接编辑
 */
export function useUpdateBadgeSortOrder() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, sortOrder }: { id: number; sortOrder: number }) =>
      updateBadgeSortOrder(id, sortOrder),
    onSuccess: () => {
      message.success('排序已更新');
      // 失效列表缓存以反映新排序
      queryClient.invalidateQueries({ queryKey: BADGE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '排序更新失败');
    },
  });
}

/**
 * 徽章系列 React Query Hooks
 *
 * 封装系列相关的数据查询和变更操作，提供缓存管理和乐观更新
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  getSeriesList,
  getSeries,
  createSeries,
  updateSeries,
  deleteSeries,
  toggleSeriesStatus,
  updateSeriesSortOrder,
  getSeriesBadges,
  getAllSeries,
  type SeriesListParams,
} from '@/services/series';
import type {
  BadgeSeries,
  CreateSeriesRequest,
  UpdateSeriesRequest,
  CategoryStatus,
} from '@/types';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const SERIES_QUERY_KEYS = {
  all: ['series'] as const,
  lists: () => [...SERIES_QUERY_KEYS.all, 'list'] as const,
  list: (params: SeriesListParams) =>
    [...SERIES_QUERY_KEYS.lists(), params] as const,
  details: () => [...SERIES_QUERY_KEYS.all, 'detail'] as const,
  detail: (id: number) => [...SERIES_QUERY_KEYS.details(), id] as const,
  badges: (seriesId: number) => [...SERIES_QUERY_KEYS.all, 'badges', seriesId] as const,
  allList: (categoryId?: number) => [...SERIES_QUERY_KEYS.all, 'allList', categoryId] as const,
};

/**
 * 查询系列列表
 *
 * @param params - 分页和筛选参数
 * @param enabled - 是否启用查询
 */
export function useSeriesList(params: SeriesListParams, enabled = true) {
  return useQuery({
    queryKey: SERIES_QUERY_KEYS.list(params),
    queryFn: () => getSeriesList(params),
    enabled,
  });
}

/**
 * 查询系列详情
 *
 * @param id - 系列 ID
 * @param enabled - 是否启用查询
 */
export function useSeriesDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: SERIES_QUERY_KEYS.detail(id),
    queryFn: () => getSeries(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 查询系列下的徽章列表
 *
 * @param seriesId - 系列 ID
 * @param enabled - 是否启用查询
 */
export function useSeriesBadges(seriesId: number, enabled = true) {
  return useQuery({
    queryKey: SERIES_QUERY_KEYS.badges(seriesId),
    queryFn: () => getSeriesBadges(seriesId),
    enabled: enabled && seriesId > 0,
  });
}

/**
 * 查询全部系列（下拉选择用）
 *
 * @param categoryId - 可选的分类 ID 过滤
 * @param enabled - 是否启用查询
 */
export function useAllSeries(categoryId?: number, enabled = true) {
  return useQuery({
    queryKey: SERIES_QUERY_KEYS.allList(categoryId),
    queryFn: () => getAllSeries(categoryId),
    enabled,
    staleTime: 10 * 60 * 1000, // 下拉数据可以缓存更久
  });
}

/**
 * 创建系列
 *
 * 成功后自动失效列表缓存
 */
export function useCreateSeries() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateSeriesRequest) => createSeries(data),
    onSuccess: () => {
      message.success('系列创建成功');
      // 失效所有系列列表缓存，触发重新请求
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 更新系列
 *
 * 成功后更新缓存中的系列数据
 */
export function useUpdateSeries() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateSeriesRequest }) =>
      updateSeries(id, data),
    onSuccess: (updatedSeries: BadgeSeries) => {
      message.success('系列更新成功');
      // 更新详情缓存
      queryClient.setQueryData(
        SERIES_QUERY_KEYS.detail(updatedSeries.id),
        updatedSeries
      );
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '更新失败');
    },
  });
}

/**
 * 删除系列
 *
 * 删除前由调用方负责确认弹窗
 */
export function useDeleteSeries() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteSeries(id),
    onSuccess: (_data, id) => {
      message.success('系列删除成功');
      // 移除详情缓存
      queryClient.removeQueries({ queryKey: SERIES_QUERY_KEYS.detail(id) });
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '删除失败，请检查系列下是否有徽章');
    },
  });
}

/**
 * 切换系列状态
 *
 * 用于 Switch 组件的状态切换
 */
export function useToggleSeriesStatus() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: CategoryStatus }) =>
      toggleSeriesStatus(id, status),
    onSuccess: (_data, { status }) => {
      const statusText = status === 'ACTIVE' ? '启用' : '禁用';
      message.success(`系列已${statusText}`);
      // 失效列表缓存以刷新状态
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '状态切换失败');
    },
  });
}

/**
 * 更新系列排序
 *
 * 用于排序值的直接编辑
 */
export function useUpdateSeriesSortOrder() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, sortOrder }: { id: number; sortOrder: number }) =>
      updateSeriesSortOrder(id, sortOrder),
    onSuccess: () => {
      message.success('排序已更新');
      // 失效列表缓存以反映新排序
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: SERIES_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '排序更新失败');
    },
  });
}

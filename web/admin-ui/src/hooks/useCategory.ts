/**
 * 徽章分类 React Query Hooks
 *
 * 封装分类相关的数据查询和变更操作，提供缓存管理和乐观更新
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  getCategories,
  getCategory,
  createCategory,
  updateCategory,
  deleteCategory,
  toggleCategoryStatus,
  updateCategorySortOrder,
  getAllCategories,
  type CategoryListParams,
} from '@/services/category';
import type {
  BadgeCategory,
  CreateCategoryRequest,
  UpdateCategoryRequest,
  CategoryStatus,
} from '@/types';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const CATEGORY_QUERY_KEYS = {
  all: ['categories'] as const,
  lists: () => [...CATEGORY_QUERY_KEYS.all, 'list'] as const,
  list: (params: CategoryListParams) =>
    [...CATEGORY_QUERY_KEYS.lists(), params] as const,
  details: () => [...CATEGORY_QUERY_KEYS.all, 'detail'] as const,
  detail: (id: number) => [...CATEGORY_QUERY_KEYS.details(), id] as const,
  allList: () => [...CATEGORY_QUERY_KEYS.all, 'allList'] as const,
};

/**
 * 查询分类列表
 *
 * @param params - 分页和筛选参数
 * @param enabled - 是否启用查询
 */
export function useCategoryList(params: CategoryListParams, enabled = true) {
  return useQuery({
    queryKey: CATEGORY_QUERY_KEYS.list(params),
    queryFn: () => getCategories(params),
    enabled,
  });
}

/**
 * 查询分类详情
 *
 * @param id - 分类 ID
 * @param enabled - 是否启用查询
 */
export function useCategoryDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: CATEGORY_QUERY_KEYS.detail(id),
    queryFn: () => getCategory(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 查询全部分类（下拉选择用）
 */
export function useAllCategories(enabled = true) {
  return useQuery({
    queryKey: CATEGORY_QUERY_KEYS.allList(),
    queryFn: getAllCategories,
    enabled,
    staleTime: 10 * 60 * 1000, // 下拉数据可以缓存更久
  });
}

/**
 * 创建分类
 *
 * 成功后自动失效列表缓存
 */
export function useCreateCategory() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateCategoryRequest) => createCategory(data),
    onSuccess: () => {
      message.success('分类创建成功');
      // 失效所有分类列表缓存，触发重新请求
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 更新分类
 *
 * 成功后更新缓存中的分类数据
 */
export function useUpdateCategory() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateCategoryRequest }) =>
      updateCategory(id, data),
    onSuccess: (updatedCategory: BadgeCategory) => {
      message.success('分类更新成功');
      // 更新详情缓存
      queryClient.setQueryData(
        CATEGORY_QUERY_KEYS.detail(updatedCategory.id),
        updatedCategory
      );
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '更新失败');
    },
  });
}

/**
 * 删除分类
 *
 * 删除前由调用方负责确认弹窗
 */
export function useDeleteCategory() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: number) => deleteCategory(id),
    onSuccess: (_data, id) => {
      message.success('分类删除成功');
      // 移除详情缓存
      queryClient.removeQueries({ queryKey: CATEGORY_QUERY_KEYS.detail(id) });
      // 失效列表缓存
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '删除失败，请检查分类下是否有系列');
    },
  });
}

/**
 * 切换分类状态
 *
 * 用于 Switch 组件的状态切换
 */
export function useToggleCategoryStatus() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: CategoryStatus }) =>
      toggleCategoryStatus(id, status),
    onSuccess: (_data, { status }) => {
      const statusText = status === 'ACTIVE' ? '启用' : '禁用';
      message.success(`分类已${statusText}`);
      // 失效列表缓存以刷新状态
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '状态切换失败');
    },
  });
}

/**
 * 更新分类排序
 *
 * 用于排序值的直接编辑
 */
export function useUpdateCategorySortOrder() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, sortOrder }: { id: number; sortOrder: number }) =>
      updateCategorySortOrder(id, sortOrder),
    onSuccess: () => {
      message.success('排序已更新');
      // 失效列表缓存以反映新排序
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.lists() });
      queryClient.invalidateQueries({ queryKey: CATEGORY_QUERY_KEYS.allList() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '排序更新失败');
    },
  });
}

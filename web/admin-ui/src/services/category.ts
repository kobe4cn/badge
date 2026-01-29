/**
 * 徽章分类 API 服务
 *
 * 封装分类相关的 CRUD 操作和状态管理接口
 */

import { get, getList, post, put, patch, del } from './api';
import type {
  BadgeCategory,
  CreateCategoryRequest,
  UpdateCategoryRequest,
  CategoryStatus,
  PaginatedResponse,
  ListParams,
} from '@/types';

/**
 * 分类列表查询参数
 */
export interface CategoryListParams extends ListParams {
  /** 名称模糊搜索 */
  name?: string;
  /** 状态筛选 */
  status?: CategoryStatus;
}

/**
 * 分类列表项（包含统计信息）
 */
export interface CategoryListItem extends BadgeCategory {
  /** 该分类下的徽章数量 */
  badgeCount: number;
  /** 该分类下的系列数量 */
  seriesCount: number;
}

/**
 * 获取分类分页列表
 *
 * @param params - 分页和筛选参数
 */
export function getCategories(
  params: CategoryListParams
): Promise<PaginatedResponse<CategoryListItem>> {
  return getList<CategoryListItem>('/api/v1/categories', params as Record<string, unknown>);
}

/**
 * 获取分类详情
 *
 * @param id - 分类 ID
 */
export function getCategory(id: number): Promise<BadgeCategory> {
  return get<BadgeCategory>(`/api/v1/categories/${id}`);
}

/**
 * 创建分类
 *
 * @param data - 创建请求数据
 */
export function createCategory(data: CreateCategoryRequest): Promise<BadgeCategory> {
  return post<BadgeCategory>('/api/v1/categories', data);
}

/**
 * 更新分类
 *
 * @param id - 分类 ID
 * @param data - 更新请求数据
 */
export function updateCategory(
  id: number,
  data: UpdateCategoryRequest
): Promise<BadgeCategory> {
  return put<BadgeCategory>(`/api/v1/categories/${id}`, data);
}

/**
 * 删除分类
 *
 * 删除前需确保分类下无系列，否则会报错
 *
 * @param id - 分类 ID
 */
export function deleteCategory(id: number): Promise<void> {
  return del<void>(`/api/v1/categories/${id}`);
}

/**
 * 切换分类状态
 *
 * 快捷接口，用于启用/禁用状态切换
 *
 * @param id - 分类 ID
 * @param status - 目标状态
 */
export function toggleCategoryStatus(
  id: number,
  status: CategoryStatus
): Promise<void> {
  return patch<void>(`/api/v1/categories/${id}/status`, { status });
}

/**
 * 更新分类排序值
 *
 * @param id - 分类 ID
 * @param sortOrder - 新的排序值（越小越靠前）
 */
export function updateCategorySortOrder(
  id: number,
  sortOrder: number
): Promise<void> {
  return patch<void>(`/api/v1/categories/${id}/sort`, { sortOrder });
}

/**
 * 获取全部分类（不分页）
 *
 * 用于下拉选择等场景
 */
export function getAllCategories(): Promise<CategoryListItem[]> {
  return get<CategoryListItem[]>('/api/v1/categories/all');
}

/**
 * 徽章系列 API 服务
 *
 * 封装系列相关的 CRUD 操作，系列是徽章的二级分类
 */

import { get, getList, post, put, patch, del } from './api';
import type {
  BadgeSeries,
  Badge,
  CreateSeriesRequest,
  UpdateSeriesRequest,
  CategoryStatus,
  PaginatedResponse,
  ListParams,
} from '@/types';

/**
 * 系列列表查询参数
 */
export interface SeriesListParams extends ListParams {
  /** 名称模糊搜索 */
  name?: string;
  /** 所属分类 ID */
  categoryId?: number;
  /** 状态筛选 */
  status?: CategoryStatus;
}

/**
 * 系列列表项（包含统计信息）
 */
export interface SeriesListItem extends BadgeSeries {
  /** 该系列下的徽章数量 */
  badgeCount: number;
  /** 所属分类名称（用于列表展示） */
  categoryName?: string;
}

/**
 * 获取系列分页列表
 *
 * @param params - 分页和筛选参数
 */
export function getSeriesList(
  params: SeriesListParams
): Promise<PaginatedResponse<SeriesListItem>> {
  return getList<SeriesListItem>('/api/v1/series', params as Record<string, unknown>);
}

/**
 * 获取系列详情
 *
 * @param id - 系列 ID
 */
export function getSeries(id: number): Promise<BadgeSeries> {
  return get<BadgeSeries>(`/api/v1/series/${id}`);
}

/**
 * 创建系列
 *
 * @param data - 创建请求数据
 */
export function createSeries(data: CreateSeriesRequest): Promise<BadgeSeries> {
  return post<BadgeSeries>('/api/v1/series', data);
}

/**
 * 更新系列
 *
 * @param id - 系列 ID
 * @param data - 更新请求数据
 */
export function updateSeries(
  id: number,
  data: UpdateSeriesRequest
): Promise<BadgeSeries> {
  return put<BadgeSeries>(`/api/v1/series/${id}`, data);
}

/**
 * 删除系列
 *
 * 删除前需确保系列下无徽章，否则会报错
 *
 * @param id - 系列 ID
 */
export function deleteSeries(id: number): Promise<void> {
  return del<void>(`/api/v1/series/${id}`);
}

/**
 * 切换系列状态
 *
 * 快捷接口，用于启用/禁用状态切换
 *
 * @param id - 系列 ID
 * @param status - 目标状态
 */
export function toggleSeriesStatus(
  id: number,
  status: CategoryStatus
): Promise<void> {
  return patch<void>(`/api/v1/series/${id}/status`, { status });
}

/**
 * 更新系列排序值
 *
 * @param id - 系列 ID
 * @param sortOrder - 新的排序值（越小越靠前）
 */
export function updateSeriesSortOrder(
  id: number,
  sortOrder: number
): Promise<void> {
  return patch<void>(`/api/v1/series/${id}/sort`, { sortOrder });
}

/**
 * 获取系列下的徽章列表
 *
 * @param seriesId - 系列 ID
 */
export function getSeriesBadges(seriesId: number): Promise<Badge[]> {
  return get<Badge[]>(`/api/v1/series/${seriesId}/badges`);
}

/**
 * 获取指定分类下的所有系列（不分页）
 *
 * 用于下拉选择等场景
 *
 * @param categoryId - 分类 ID（可选，不传则返回所有系列）
 */
export function getAllSeries(categoryId?: number): Promise<SeriesListItem[]> {
  const params = categoryId ? { categoryId } : {};
  return get<SeriesListItem[]>('/api/v1/series/all', params);
}

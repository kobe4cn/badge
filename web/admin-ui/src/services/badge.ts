/**
 * 徽章定义 API 服务
 *
 * 封装徽章 CRUD 操作和状态管理接口
 */

import { get, getList, post, put, patch, del } from './api';
import type {
  Badge,
  BadgeType,
  BadgeStatus,
  CreateBadgeRequest,
  UpdateBadgeRequest,
  PaginatedResponse,
  ListParams,
} from '@/types';

/**
 * 徽章列表查询参数
 */
export interface BadgeListParams extends ListParams {
  /** 名称模糊搜索 */
  name?: string;
  /** 徽章类型筛选 */
  badgeType?: BadgeType;
  /** 状态筛选 */
  status?: BadgeStatus;
  /** 所属分类 ID（用于联动筛选） */
  categoryId?: number;
  /** 所属系列 ID */
  seriesId?: number;
}

/**
 * 徽章列表项（包含关联信息）
 */
export interface BadgeListItem extends Badge {
  /** 所属系列名称 */
  seriesName?: string;
  /** 所属分类 ID */
  categoryId?: number;
  /** 所属分类名称 */
  categoryName?: string;
}

/**
 * 徽章详情（包含完整关联信息）
 */
export interface BadgeDetail extends Badge {
  /** 所属系列名称 */
  seriesName?: string;
  /** 所属分类 ID */
  categoryId?: number;
  /** 所属分类名称 */
  categoryName?: string;
  /** 持有该徽章的用户数 */
  holderCount?: number;
}

/**
 * 获取徽章分页列表
 *
 * @param params - 分页和筛选参数
 */
export function getBadges(
  params: BadgeListParams
): Promise<PaginatedResponse<BadgeListItem>> {
  return getList<BadgeListItem>('/admin/badges', params as Record<string, unknown>);
}

/**
 * 获取徽章详情
 *
 * @param id - 徽章 ID
 */
export function getBadge(id: number): Promise<BadgeDetail> {
  return get<BadgeDetail>(`/admin/badges/${id}`);
}

/**
 * 创建徽章
 *
 * @param data - 创建请求数据
 */
export function createBadge(data: CreateBadgeRequest): Promise<Badge> {
  return post<Badge>('/admin/badges', data);
}

/**
 * 更新徽章
 *
 * @param id - 徽章 ID
 * @param data - 更新请求数据
 */
export function updateBadge(
  id: number,
  data: UpdateBadgeRequest
): Promise<Badge> {
  return put<Badge>(`/admin/badges/${id}`, data);
}

/**
 * 删除徽章
 *
 * 仅允许删除草稿状态的徽章
 *
 * @param id - 徽章 ID
 */
export function deleteBadge(id: number): Promise<void> {
  return del<void>(`/admin/badges/${id}`);
}

/**
 * 上架徽章
 *
 * 将徽章状态从 DRAFT/INACTIVE 变更为 ACTIVE
 *
 * @param id - 徽章 ID
 */
export function publishBadge(id: number): Promise<void> {
  return post<void>(`/admin/badges/${id}/publish`);
}

/**
 * 下架徽章
 *
 * 将徽章状态从 ACTIVE 变更为 INACTIVE
 *
 * @param id - 徽章 ID
 */
export function unpublishBadge(id: number): Promise<void> {
  return post<void>(`/admin/badges/${id}/offline`);
}

/**
 * 归档徽章
 *
 * 将徽章状态变更为 ARCHIVED
 *
 * @param id - 徽章 ID
 */
export function archiveBadge(id: number): Promise<void> {
  return post<void>(`/admin/badges/${id}/archive`);
}

/**
 * 更新徽章排序值
 *
 * @param id - 徽章 ID
 * @param sortOrder - 新的排序值（越小越靠前）
 */
export function updateBadgeSortOrder(
  id: number,
  sortOrder: number
): Promise<void> {
  return patch<void>(`/admin/badges/${id}/sort`, { sortOrder });
}

/**
 * 获取所有徽章（不分页）
 *
 * 用于下拉选择等场景，返回所有可用徽章
 */
export function getAllBadges(): Promise<PaginatedResponse<BadgeListItem>> {
  return getList<BadgeListItem>('/admin/badges', { page: 1, pageSize: 1000 });
}

/**
 * 徽章服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const badgeService = {
  getList: getBadges,
  get: getBadge,
  create: createBadge,
  update: updateBadge,
  delete: deleteBadge,
  publish: publishBadge,
  unpublish: unpublishBadge,
  archive: archiveBadge,
  updateSortOrder: updateBadgeSortOrder,
  getAll: getAllBadges,
};

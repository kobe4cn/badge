/**
 * 徽章依赖关系 API 服务
 *
 * 封装依赖关系的 CRUD 操作和缓存管理接口
 */

import { get, post, del } from './api';

/**
 * 依赖关系类型
 *
 * - prerequisite: 前置条件，必须持有依赖徽章才能获取当前徽章
 * - consume: 消耗类型，获取当前徽章时会消耗依赖徽章
 * - exclusive: 互斥关系，与指定互斥组内的徽章不能同时持有
 */
export type DependencyType = 'prerequisite' | 'consume' | 'exclusive';

/**
 * 徽章依赖关系模型
 */
export interface BadgeDependency {
  id: string;
  badgeId: string;
  dependsOnBadgeId: string;
  dependencyType: DependencyType;
  requiredQuantity: number;
  exclusiveGroupId: string | null;
  autoTrigger: boolean;
  priority: number;
  dependencyGroupId: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 创建依赖关系请求参数
 */
export interface CreateDependencyRequest {
  dependsOnBadgeId: string;
  dependencyType: DependencyType;
  requiredQuantity?: number;
  exclusiveGroupId?: string;
  autoTrigger?: boolean;
  priority?: number;
  dependencyGroupId: string;
}

/**
 * 获取徽章的依赖关系列表
 *
 * @param badgeId - 徽章 ID
 */
export function getDependencies(badgeId: string): Promise<BadgeDependency[]> {
  return get<BadgeDependency[]>(`/api/admin/badges/${badgeId}/dependencies`);
}

/**
 * 创建依赖关系
 *
 * @param badgeId - 徽章 ID
 * @param data - 创建请求数据
 */
export function createDependency(
  badgeId: string,
  data: CreateDependencyRequest
): Promise<BadgeDependency> {
  return post<BadgeDependency>(`/api/admin/badges/${badgeId}/dependencies`, data);
}

/**
 * 删除依赖关系
 *
 * @param id - 依赖关系 ID
 */
export function deleteDependency(id: string): Promise<void> {
  return del<void>(`/api/admin/dependencies/${id}`);
}

/**
 * 刷新依赖缓存
 *
 * 用于管理员手动刷新服务端的依赖关系缓存
 */
export function refreshDependencyCache(): Promise<void> {
  return post<void>('/api/admin/cache/dependencies/refresh');
}

/**
 * 依赖关系服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const dependencyService = {
  getList: getDependencies,
  create: createDependency,
  delete: deleteDependency,
  refreshCache: refreshDependencyCache,
};

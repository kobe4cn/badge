/**
 * 徽章依赖关系 API 服务
 *
 * 封装依赖关系的 CRUD 操作和缓存管理接口
 */

import { get, post, put, del } from './api';

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
 * 更新依赖关系请求参数
 */
export interface UpdateDependencyRequest {
  dependencyType?: DependencyType;
  requiredQuantity?: number;
  exclusiveGroupId?: string;
  autoTrigger?: boolean;
  priority?: number;
  dependencyGroupId?: string;
  enabled?: boolean;
}

/**
 * 依赖图节点
 */
export interface DependencyGraphNode {
  id: string;
  badgeId: number;
  label: string;
  nodeType: 'root' | 'prerequisite' | 'dependent' | 'badge';
}

/**
 * 依赖图边
 */
export interface DependencyGraphEdge {
  id: string;
  source: string;
  target: string;
  edgeType: DependencyType;
  label: string;
}

/**
 * 依赖图响应
 */
export interface DependencyGraph {
  nodes: DependencyGraphNode[];
  edges: DependencyGraphEdge[];
}

/**
 * 获取徽章的依赖关系列表
 *
 * @param badgeId - 徽章 ID
 */
export function getDependencies(badgeId: string): Promise<BadgeDependency[]> {
  return get<BadgeDependency[]>(`/admin/badges/${badgeId}/dependencies`);
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
  return post<BadgeDependency>(`/admin/badges/${badgeId}/dependencies`, data);
}

/**
 * 删除依赖关系
 *
 * @param id - 依赖关系 ID
 */
export function deleteDependency(id: string): Promise<void> {
  return del<void>(`/admin/dependencies/${id}`);
}

/**
 * 刷新依赖缓存
 *
 * 用于管理员手动刷新服务端的依赖关系缓存
 */
export function refreshDependencyCache(): Promise<void> {
  return post<void>('/admin/cache/dependencies/refresh');
}

/**
 * 更新依赖关系
 *
 * @param id - 依赖关系 ID
 * @param data - 更新请求数据
 */
export function updateDependency(
  id: string,
  data: UpdateDependencyRequest
): Promise<BadgeDependency> {
  return put<BadgeDependency>(`/admin/dependencies/${id}`, data);
}

/**
 * 获取依赖图数据
 *
 * @param badgeId - 可选的徽章 ID，如果提供则只返回相关的子图
 */
export function getDependencyGraph(badgeId?: string): Promise<DependencyGraph> {
  const params = badgeId ? `?badgeId=${badgeId}` : '';
  return get<DependencyGraph>(`/admin/dependencies/graph${params}`);
}

/**
 * 依赖关系服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const dependencyService = {
  getList: getDependencies,
  create: createDependency,
  update: updateDependency,
  delete: deleteDependency,
  refreshCache: refreshDependencyCache,
  getGraph: getDependencyGraph,
};

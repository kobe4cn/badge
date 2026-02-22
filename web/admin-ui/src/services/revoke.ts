/**
 * 撤销管理 API 服务
 *
 * 封装手动撤销、批量撤销和撤销记录查询接口
 */

import { getList, post, apiClient } from './api';
import type { PaginatedResponse, ListParams, GrantLog, BatchTask } from '@/types';

/**
 * 手动撤销请求
 */
export interface ManualRevokeRequest {
  /** 用户徽章记录 ID */
  userBadgeId: number;
  /** 撤销原因 */
  reason: string;
}

/**
 * 批量撤销请求
 *
 * 支持两种模式：userIds 直接传入或 fileUrl 指向 CSV
 */
export interface BatchRevokeRequest {
  /** 徽章 ID */
  badgeId: number;
  /** 用户 ID 列表（直接传入，少量用户场景） */
  userIds?: string[];
  /** CSV 上传后的 Redis 引用键 */
  csvRefKey?: string;
  /** OSS 文件地址（可选） */
  fileUrl?: string;
  /** 撤销原因 */
  reason: string;
}

/**
 * 手动撤销结果
 */
export interface RevokeResult {
  /** 用户 ID */
  userId: string;
  /** 徽章 ID */
  badgeId: number;
  /** 徽章名称 */
  badgeName: string;
  /** 撤销数量 */
  quantity: number;
  /** 剩余数量 */
  remaining: number;
  /** 来源引用 ID */
  sourceRefId: string;
}

/**
 * 撤销记录查询参数
 */
export interface RevokeLogParams extends ListParams {
  /** 用户 ID */
  userId?: string;
  /** 徽章 ID */
  badgeId?: number;
  /** 来源类型 */
  sourceType?: string;
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

/**
 * 手动撤销徽章
 *
 * @param data - 撤销请求数据
 */
export function manualRevoke(data: ManualRevokeRequest): Promise<RevokeResult> {
  return post<RevokeResult>('/admin/revokes/manual', data);
}

/**
 * 批量撤销徽章
 *
 * 创建异步批量撤销任务
 *
 * @param data - 批量撤销请求
 */
export function batchRevoke(data: BatchRevokeRequest): Promise<BatchTask> {
  return post<BatchTask>('/admin/revokes/batch', data);
}

/**
 * 获取撤销记录列表
 *
 * @param params - 查询参数
 */
export function getRevokeRecords(
  params: RevokeLogParams
): Promise<PaginatedResponse<GrantLog>> {
  return getList<GrantLog>('/admin/revokes', params as Record<string, unknown>);
}

/**
 * 导出撤销记录
 *
 * @param params - 查询参数（不含分页）
 */
export async function exportRevokeRecords(
  params: Omit<RevokeLogParams, 'page' | 'pageSize'>
): Promise<Blob> {
  const response = await apiClient.get('/admin/revokes/export', {
    params,
    responseType: 'blob',
  });
  return response.data;
}

/**
 * 自动取消场景
 */
export type AutoRevokeScenario =
  | 'account_deletion'
  | 'identity_change'
  | 'condition_unmet'
  | 'violation'
  | 'system_triggered';

/**
 * 自动取消请求
 */
export interface AutoRevokeRequest {
  /** 用户 ID */
  userId: string;
  /** 徽章 ID（可选，为空则撤销该用户所有徽章） */
  badgeId?: number;
  /** 自动取消场景 */
  scenario: AutoRevokeScenario;
  /** 关联的业务 ID（如订单号、会员变更单号） */
  refId?: string;
  /** 取消原因说明 */
  reason: string;
}

/**
 * 被撤销的徽章信息
 */
export interface RevokedBadgeInfo {
  badgeId: number;
  badgeName: string;
  quantity: number;
}

/**
 * 自动取消结果
 */
export interface AutoRevokeResult {
  userId: string;
  revokedCount: number;
  revokedBadges: RevokedBadgeInfo[];
  scenario: string;
}

/**
 * 自动取消徽章
 *
 * 用于账号注销、身份变更、条件不满足等自动触发的撤销场景
 *
 * @param data - 自动取消请求数据
 */
export function autoRevoke(data: AutoRevokeRequest): Promise<AutoRevokeResult> {
  return post<AutoRevokeResult>('/admin/revokes/auto', data);
}

/**
 * 撤销服务对象
 */
export const revokeService = {
  manualRevoke,
  batchRevoke,
  autoRevoke,
  getRevokeRecords,
  exportRevokeRecords,
};

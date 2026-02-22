/**
 * 操作日志 API 服务
 *
 * 封装审计日志查询接口，对应后端 GET /api/admin/logs
 */

import { getList } from './api';
import type { PaginatedResponse, ListParams } from '@/types';

/**
 * 操作日志实体
 *
 * 与后端 OperationLogDto 一一对应（camelCase 映射）
 */
export interface OperationLog {
  id: number;
  operatorId: string;
  operatorName?: string;
  /** 操作模块：badge/rule/grant/revoke/category/series/benefit/redemption 等 */
  module: string;
  /** 操作动作：create/update/delete/publish/grant/revoke 等 */
  action: string;
  targetType?: string;
  targetId?: string;
  beforeData?: Record<string, unknown>;
  afterData?: Record<string, unknown>;
  ipAddress?: string;
  createdAt: string;
}

/**
 * 操作日志查询参数
 *
 * 与后端 OperationLogFilter 对应
 */
export interface OperationLogQueryParams extends ListParams {
  operatorId?: string;
  module?: string;
  action?: string;
  targetType?: string;
  targetId?: string;
  startTime?: string;
  endTime?: string;
}

/**
 * 查询操作日志列表
 */
export function listOperationLogs(
  params?: OperationLogQueryParams
): Promise<PaginatedResponse<OperationLog>> {
  return getList<OperationLog>('/admin/logs', params as Record<string, unknown>);
}

/**
 * 操作日志服务对象
 */
export const operationLogService = {
  list: listOperationLogs,
};

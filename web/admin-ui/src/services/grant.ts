/**
 * 发放管理 API 服务
 *
 * 封装手动发放、批量发放和发放日志查询等接口
 */

import { get, getList, post, upload, apiClient } from './api';
import type {
  ManualGrantRequest,
  BatchGrantRequest,
  GrantRecord,
  GrantLog,
  GrantLogQueryParams,
  BatchTask,
  BatchTaskQueryParams,
  BatchTaskFailure,
  CreateBatchTaskRequest,
  CsvParseResult,
  UserFilterCondition,
  UserFilterPreview,
  User,
  PaginatedResponse,
  ListParams,
} from '@/types';

/**
 * 发放结果中的单用户结果
 */
export interface UserGrantResult {
  /** 用户 ID */
  userId: string;
  /** 是否成功 */
  success: boolean;
  /** 错误信息（失败时） */
  message?: string;
  /** 创建的用户徽章 ID（成功时） */
  userBadgeId?: number;
}

/**
 * 手动发放结果
 */
export interface GrantResult {
  /** 总处理数量 */
  totalCount: number;
  /** 成功数量 */
  successCount: number;
  /** 失败数量 */
  failedCount: number;
  /** 各用户发放结果 */
  results: UserGrantResult[];
}

/**
 * 批量发放结果
 */
export interface BatchGrantResult {
  /** 批量任务 ID */
  taskId: number;
  /** 任务状态 */
  status: string;
  /** 提示信息 */
  message: string;
}

/**
 * 发放日志查询参数
 */
export interface GrantLogParams extends ListParams, GrantLogQueryParams {}

/**
 * 发放记录查询参数
 */
export interface GrantRecordParams extends ListParams {
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
 * 手动发放徽章
 *
 * @param data - 发放请求数据，包含用户列表、徽章 ID、数量和原因
 */
export function manualGrant(data: ManualGrantRequest): Promise<GrantResult> {
  return post<GrantResult>('/api/v1/grants/manual', data);
}

/**
 * 批量发放徽章
 *
 * 创建异步批量发放任务
 *
 * @param data - 批量发放请求，包含文件 URL 和徽章 ID
 */
export function batchGrant(data: BatchGrantRequest): Promise<BatchGrantResult> {
  return post<BatchGrantResult>('/api/v1/grants/batch', data);
}

/**
 * 获取发放日志
 *
 * @param params - 查询参数
 */
export function getGrantLogs(
  params: GrantLogParams
): Promise<PaginatedResponse<GrantLog>> {
  return getList<GrantLog>('/api/v1/grants/logs', params as Record<string, unknown>);
}

/**
 * 获取发放记录
 *
 * @param params - 查询参数
 */
export function getGrantRecords(
  params: GrantRecordParams
): Promise<PaginatedResponse<GrantRecord>> {
  return getList<GrantRecord>('/api/v1/grants/records', params as Record<string, unknown>);
}

/**
 * 搜索用户
 *
 * 支持按用户 ID、手机号、昵称模糊搜索
 *
 * @param keyword - 搜索关键词
 */
export function searchUsers(keyword: string): Promise<User[]> {
  return get<User[]>('/api/v1/users/search', { keyword });
}

/**
 * 获取批量任务列表
 *
 * @param params - 查询参数
 */
export function getBatchTasks(
  params: BatchTaskQueryParams & ListParams
): Promise<PaginatedResponse<BatchTask>> {
  return getList<BatchTask>('/api/v1/grants/batch-tasks', params as Record<string, unknown>);
}

/**
 * 获取批量任务详情
 *
 * @param id - 任务 ID
 */
export function getBatchTask(id: number): Promise<BatchTask> {
  return get<BatchTask>(`/api/v1/grants/batch-tasks/${id}`);
}

/**
 * 创建批量任务
 *
 * @param data - 创建请求数据
 */
export function createBatchTask(data: CreateBatchTaskRequest): Promise<BatchTask> {
  return post<BatchTask>('/api/v1/grants/batch-tasks', data);
}

/**
 * 取消批量任务
 *
 * @param id - 任务 ID
 */
export function cancelBatchTask(id: number): Promise<void> {
  return post<void>(`/api/v1/grants/batch-tasks/${id}/cancel`);
}

/**
 * 获取批量任务失败明细
 *
 * @param id - 任务 ID
 */
export function getBatchTaskFailures(
  id: number,
  params?: ListParams
): Promise<PaginatedResponse<BatchTaskFailure>> {
  return getList<BatchTaskFailure>(
    `/api/v1/grants/batch-tasks/${id}/failures`,
    params as Record<string, unknown>
  );
}

/**
 * 下载批量任务结果
 *
 * @param id - 任务 ID
 */
export async function downloadBatchResult(id: number): Promise<Blob> {
  const response = await apiClient.get(`/api/v1/grants/batch-tasks/${id}/result`, {
    responseType: 'blob',
  });
  return response.data;
}

/**
 * 上传用户 CSV 文件
 *
 * @param file - CSV 文件
 */
export function uploadUserCsv(file: File): Promise<CsvParseResult> {
  return upload<CsvParseResult>('/api/v1/grants/upload-csv', file);
}

/**
 * 预览用户筛选结果
 *
 * @param filter - 筛选条件
 */
export function previewUserFilter(filter: UserFilterCondition): Promise<UserFilterPreview> {
  return post<UserFilterPreview>('/api/v1/grants/preview-filter', filter);
}

/**
 * 发放服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const grantService = {
  manualGrant,
  batchGrant,
  getGrantLogs,
  getGrantRecords,
  searchUsers,
  getBatchTasks,
  getBatchTask,
  createBatchTask,
  cancelBatchTask,
  getBatchTaskFailures,
  downloadBatchResult,
  uploadUserCsv,
  previewUserFilter,
};

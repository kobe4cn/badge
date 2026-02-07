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
  GrantLogDetail,
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
  RetryResult,
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
  return post<GrantResult>('/admin/grants/manual', data);
}

/**
 * 批量发放徽章
 *
 * 创建异步批量发放任务
 *
 * @param data - 批量发放请求，包含文件 URL 和徽章 ID
 */
export function batchGrant(data: BatchGrantRequest): Promise<BatchGrantResult> {
  return post<BatchGrantResult>('/admin/grants/batch', data);
}

/**
 * 获取发放日志
 *
 * @param params - 查询参数
 */
export function getGrantLogs(
  params: GrantLogParams
): Promise<PaginatedResponse<GrantLog>> {
  return getList<GrantLog>('/admin/grants/logs', params as Record<string, unknown>);
}

/**
 * 获取发放日志详情
 *
 * @param id - 日志 ID
 */
export function getGrantLogDetail(id: number): Promise<GrantLogDetail> {
  return get<GrantLogDetail>(`/admin/grants/logs/${id}`);
}

/**
 * 导出发放日志
 *
 * 根据筛选条件导出日志为 CSV 文件
 *
 * @param params - 查询参数（不含分页）
 */
export async function exportGrantLogs(
  params: Omit<GrantLogParams, 'page' | 'pageSize'>
): Promise<Blob> {
  const response = await apiClient.get('/admin/grants/logs/export', {
    params,
    responseType: 'blob',
  });
  return response.data;
}

/**
 * 获取发放记录
 *
 * @param params - 查询参数
 */
export function getGrantRecords(
  params: GrantRecordParams
): Promise<PaginatedResponse<GrantRecord>> {
  return getList<GrantRecord>('/admin/grants/records', params as Record<string, unknown>);
}

/**
 * 搜索用户
 *
 * 支持按用户 ID、手机号、昵称模糊搜索
 *
 * @param keyword - 搜索关键词
 */
export function searchUsers(keyword: string): Promise<User[]> {
  return get<User[]>('/admin/users/search', { keyword });
}

/**
 * 获取批量任务列表
 *
 * @param params - 查询参数
 */
export function getBatchTasks(
  params: BatchTaskQueryParams & ListParams
): Promise<PaginatedResponse<BatchTask>> {
  return getList<BatchTask>('/admin/tasks', params as Record<string, unknown>);
}

/**
 * 获取批量任务详情
 *
 * @param id - 任务 ID
 */
export function getBatchTask(id: number): Promise<BatchTask> {
  return get<BatchTask>(`/admin/tasks/${id}`);
}

/**
 * 创建批量任务
 *
 * 前端表单数据需转换为后端 batch_tasks 表的 { task_type, params } 格式，
 * worker 从 params JSONB 中读取 badge_id、user_ids 等信息执行任务
 */
export function createBatchTask(data: CreateBatchTaskRequest): Promise<BatchTask> {
  const payload = {
    task_type: 'batch_grant',
    params: {
      badge_id: data.badgeId,
      quantity: data.quantity,
      reason: data.reason,
      user_ids: data.userIds,
      user_filter: data.userFilter,
      name: data.name,
    },
  };
  return post<BatchTask>('/admin/tasks', payload);
}

/**
 * 取消批量任务
 *
 * @param id - 任务 ID
 */
export function cancelBatchTask(id: number): Promise<void> {
  return post<void>(`/admin/tasks/${id}/cancel`);
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
    `/admin/tasks/${id}/failures`,
    params as Record<string, unknown>
  );
}

/**
 * 下载批量任务结果
 *
 * 后端返回 { taskId, resultFileUrl }，前端取得 URL 后触发浏览器下载。
 *
 * @param id - 任务 ID
 */
export async function downloadBatchResult(id: number): Promise<void> {
  const result = await get<{ taskId: number; resultFileUrl: string | null }>(
    `/admin/tasks/${id}/result`
  );

  if (!result.resultFileUrl) {
    throw new Error('任务结果文件不存在');
  }

  // 通过创建临时 <a> 标签触发浏览器下载
  const link = document.createElement('a');
  link.href = result.resultFileUrl;
  link.download = `task-${id}-result.csv`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/**
 * 上传用户 CSV 文件
 *
 * @param file - CSV 文件
 */
export function uploadUserCsv(file: File): Promise<CsvParseResult> {
  return upload<CsvParseResult>('/admin/grants/upload-csv', file);
}

/**
 * 预览用户筛选结果
 *
 * @param filter - 筛选条件
 */
export function previewUserFilter(filter: UserFilterCondition): Promise<UserFilterPreview> {
  return post<UserFilterPreview>('/admin/grants/preview-filter', filter);
}

/**
 * 下载批量任务失败清单
 *
 * 以 CSV 格式导出任务的所有失败记录，包含重试状态信息
 *
 * @param id - 任务 ID
 */
export async function downloadTaskFailures(id: number): Promise<void> {
  const response = await apiClient.get(`/admin/tasks/${id}/failures/download`, {
    responseType: 'blob',
  });

  // 从 Content-Disposition 中获取文件名，或使用默认名
  const contentDisposition = response.headers['content-disposition'];
  let filename = `task-${id}-failures.csv`;
  if (contentDisposition) {
    const match = contentDisposition.match(/filename="?([^";\n]+)"?/);
    if (match) {
      filename = match[1];
    }
  }

  // 通过创建临时 <a> 标签触发浏览器下载
  const blob = new Blob([response.data], { type: 'text/csv;charset=utf-8' });
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  window.URL.revokeObjectURL(url);
}

/**
 * 触发批量任务失败记录重试
 *
 * 将任务中所有 EXHAUSTED 状态的失败记录重置为 PENDING，
 * 后台 Worker 会自动捡起并重试
 *
 * @param id - 任务 ID
 */
export function retryTaskFailures(id: number): Promise<RetryResult> {
  return post<RetryResult>(`/admin/tasks/${id}/retry`);
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
  getGrantLogDetail,
  exportGrantLogs,
  getGrantRecords,
  searchUsers,
  getBatchTasks,
  getBatchTask,
  createBatchTask,
  cancelBatchTask,
  getBatchTaskFailures,
  downloadBatchResult,
  downloadTaskFailures,
  retryTaskFailures,
  uploadUserCsv,
  previewUserFilter,
};

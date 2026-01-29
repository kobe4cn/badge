/**
 * 发放管理 React Query Hooks
 *
 * 封装发放相关的数据查询和变更操作，提供缓存管理
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  manualGrant,
  batchGrant,
  getGrantLogs,
  getGrantRecords,
  searchUsers,
  getBatchTasks,
  getBatchTask,
  type GrantLogParams,
  type GrantRecordParams,
  type GrantResult,
  type BatchGrantResult,
} from '@/services/grant';
import type {
  ManualGrantRequest,
  BatchGrantRequest,
  BatchTaskQueryParams,
  ListParams,
} from '@/types';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const GRANT_QUERY_KEYS = {
  all: ['grants'] as const,
  logs: () => [...GRANT_QUERY_KEYS.all, 'logs'] as const,
  logList: (params: GrantLogParams) =>
    [...GRANT_QUERY_KEYS.logs(), params] as const,
  records: () => [...GRANT_QUERY_KEYS.all, 'records'] as const,
  recordList: (params: GrantRecordParams) =>
    [...GRANT_QUERY_KEYS.records(), params] as const,
  users: () => [...GRANT_QUERY_KEYS.all, 'users'] as const,
  userSearch: (keyword: string) =>
    [...GRANT_QUERY_KEYS.users(), keyword] as const,
  batchTasks: () => [...GRANT_QUERY_KEYS.all, 'batchTasks'] as const,
  batchTaskList: (params: BatchTaskQueryParams & ListParams) =>
    [...GRANT_QUERY_KEYS.batchTasks(), params] as const,
  batchTask: (id: number) =>
    [...GRANT_QUERY_KEYS.batchTasks(), id] as const,
};

/**
 * 手动发放徽章
 *
 * 返回 mutation 用于触发发放操作
 */
export function useManualGrant() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: ManualGrantRequest) => manualGrant(data),
    onSuccess: (result: GrantResult) => {
      if (result.failedCount === 0) {
        message.success(`发放成功，共 ${result.successCount} 人`);
      } else {
        message.warning(
          `发放完成，成功 ${result.successCount} 人，失败 ${result.failedCount} 人`
        );
      }
      // 失效发放记录缓存
      queryClient.invalidateQueries({ queryKey: GRANT_QUERY_KEYS.records() });
      queryClient.invalidateQueries({ queryKey: GRANT_QUERY_KEYS.logs() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '发放失败');
    },
  });
}

/**
 * 批量发放徽章
 *
 * 创建异步批量发放任务
 */
export function useBatchGrant() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: BatchGrantRequest) => batchGrant(data),
    onSuccess: (result: BatchGrantResult) => {
      message.success(`批量任务已创建，任务 ID: ${result.taskId}`);
      // 失效批量任务缓存
      queryClient.invalidateQueries({ queryKey: GRANT_QUERY_KEYS.batchTasks() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建批量任务失败');
    },
  });
}

/**
 * 查询发放日志
 *
 * @param params - 查询参数
 * @param enabled - 是否启用查询
 */
export function useGrantLogs(params: GrantLogParams, enabled = true) {
  return useQuery({
    queryKey: GRANT_QUERY_KEYS.logList(params),
    queryFn: () => getGrantLogs(params),
    enabled,
  });
}

/**
 * 查询发放记录
 *
 * @param params - 查询参数
 * @param enabled - 是否启用查询
 */
export function useGrantRecords(params: GrantRecordParams, enabled = true) {
  return useQuery({
    queryKey: GRANT_QUERY_KEYS.recordList(params),
    queryFn: () => getGrantRecords(params),
    enabled,
  });
}

/**
 * 搜索用户
 *
 * 支持按用户 ID、手机号、昵称模糊搜索
 *
 * @param keyword - 搜索关键词
 * @param enabled - 是否启用查询（关键词为空时禁用）
 */
export function useSearchUsers(keyword: string, enabled = true) {
  return useQuery({
    queryKey: GRANT_QUERY_KEYS.userSearch(keyword),
    queryFn: () => searchUsers(keyword),
    // 关键词至少 2 个字符才触发搜索
    enabled: enabled && keyword.length >= 2,
    // 搜索结果缓存时间短一些
    staleTime: 30 * 1000,
  });
}

/**
 * 查询批量任务列表
 *
 * @param params - 查询参数
 * @param enabled - 是否启用查询
 */
export function useBatchTasks(
  params: BatchTaskQueryParams & ListParams,
  enabled = true
) {
  return useQuery({
    queryKey: GRANT_QUERY_KEYS.batchTaskList(params),
    queryFn: () => getBatchTasks(params),
    enabled,
  });
}

/**
 * 查询批量任务详情
 *
 * @param id - 任务 ID
 * @param enabled - 是否启用查询
 */
export function useBatchTaskDetail(id: number, enabled = true) {
  return useQuery({
    queryKey: GRANT_QUERY_KEYS.batchTask(id),
    queryFn: () => getBatchTask(id),
    enabled: enabled && id > 0,
    // 任务进行中时轮询刷新
    refetchInterval: (query) => {
      const task = query.state.data;
      if (task && (task.status === 'pending' || task.status === 'processing')) {
        return 3000;
      }
      return false;
    },
  });
}

/**
 * 通知管理 React Query Hooks
 *
 * 提供通知配置和任务的数据查询和变更操作
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { message } from 'antd';
import {
  getNotificationConfigs,
  getNotificationConfig,
  createNotificationConfig,
  updateNotificationConfig,
  deleteNotificationConfig,
  testNotification,
  getNotificationTasks,
  type NotificationConfigParams,
  type NotificationTaskParams,
  type CreateNotificationConfigRequest,
  type UpdateNotificationConfigRequest,
  type TestNotificationRequest,
} from '@/services/notification';

// ============ Query Keys ============

export const notificationKeys = {
  all: ['notification'] as const,
  configs: () => [...notificationKeys.all, 'configs'] as const,
  configList: (params: NotificationConfigParams) =>
    [...notificationKeys.configs(), 'list', params] as const,
  configDetail: (id: number) => [...notificationKeys.configs(), 'detail', id] as const,
  tasks: () => [...notificationKeys.all, 'tasks'] as const,
  taskList: (params: NotificationTaskParams) =>
    [...notificationKeys.tasks(), 'list', params] as const,
};

// ============ Config Hooks ============

/**
 * 查询通知配置列表
 */
export function useNotificationConfigs(params: NotificationConfigParams) {
  return useQuery({
    queryKey: notificationKeys.configList(params),
    queryFn: () => getNotificationConfigs(params),
  });
}

/**
 * 查询单个通知配置
 */
export function useNotificationConfig(id: number, enabled = true) {
  return useQuery({
    queryKey: notificationKeys.configDetail(id),
    queryFn: () => getNotificationConfig(id),
    enabled: enabled && id > 0,
  });
}

/**
 * 创建通知配置
 */
export function useCreateNotificationConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateNotificationConfigRequest) => createNotificationConfig(data),
    onSuccess: () => {
      message.success('通知配置创建成功');
      queryClient.invalidateQueries({ queryKey: notificationKeys.configs() });
    },
    onError: (error: Error) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 更新通知配置
 */
export function useUpdateNotificationConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateNotificationConfigRequest }) =>
      updateNotificationConfig(id, data),
    onSuccess: (_, { id }) => {
      message.success('通知配置更新成功');
      queryClient.invalidateQueries({ queryKey: notificationKeys.configs() });
      queryClient.invalidateQueries({ queryKey: notificationKeys.configDetail(id) });
    },
    onError: (error: Error) => {
      message.error(error.message || '更新失败');
    },
  });
}

/**
 * 删除通知配置
 */
export function useDeleteNotificationConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => deleteNotificationConfig(id),
    onSuccess: () => {
      message.success('通知配置已删除');
      queryClient.invalidateQueries({ queryKey: notificationKeys.configs() });
    },
    onError: (error: Error) => {
      message.error(error.message || '删除失败');
    },
  });
}

/**
 * 测试通知发送
 */
export function useTestNotification() {
  return useMutation({
    mutationFn: (data: TestNotificationRequest) => testNotification(data),
    onSuccess: (result) => {
      if (result.success) {
        message.success(result.message || '测试通知已发送');
      } else {
        message.warning(result.message || '测试发送失败');
      }
    },
    onError: (error: Error) => {
      message.error(error.message || '测试发送失败');
    },
  });
}

// ============ Task Hooks ============

/**
 * 查询通知任务列表
 */
export function useNotificationTasks(params: NotificationTaskParams) {
  return useQuery({
    queryKey: notificationKeys.taskList(params),
    queryFn: () => getNotificationTasks(params),
  });
}

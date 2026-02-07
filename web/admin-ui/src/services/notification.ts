/**
 * 通知配置管理服务
 *
 * 提供通知配置的 CRUD 操作、测试发送和任务查询
 */

import { get, post, put, del } from './api';
import type { PaginatedResponse, PaginationParams } from '@/types';

// ============ 类型定义 ============

/**
 * 通知配置
 */
export interface NotificationConfig {
  id: number;
  /** 关联徽章 ID（与 benefitId 二选一） */
  badgeId?: number;
  /** 关联权益 ID（与 badgeId 二选一） */
  benefitId?: number;
  /** 关联徽章名称 */
  badgeName?: string;
  /** 关联权益名称 */
  benefitName?: string;
  /** 触发类型: badge_grant, badge_expire_remind, badge_expired, benefit_grant, benefit_expire_remind */
  triggerType: string;
  /** 通知渠道列表: app_push, sms, wechat, email, in_app */
  channels: string[];
  /** 消息模板 ID */
  templateId?: string;
  /** 提前提醒天数（过期提醒场景） */
  advanceDays?: number;
  /** 失败重试次数 */
  retryCount: number;
  /** 重试间隔（秒） */
  retryIntervalSeconds: number;
  /** 是否启用 */
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 通知任务记录
 */
export interface NotificationTask {
  id: number;
  configId: number;
  userId: string;
  badgeId?: number;
  benefitId?: number;
  /** 触发类型 */
  triggerType: string;
  /** 通知渠道 */
  channel: string;
  /** 任务状态: pending, sending, success, failed */
  status: string;
  /** 错误信息 */
  errorMessage?: string;
  /** 重试次数 */
  retryCount: number;
  /** 发送时间 */
  sentAt?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 创建通知配置请求
 */
export interface CreateNotificationConfigRequest {
  badgeId?: number;
  benefitId?: number;
  triggerType: string;
  channels: string[];
  templateId?: string;
  advanceDays?: number;
  retryCount?: number;
  retryIntervalSeconds?: number;
}

/**
 * 更新通知配置请求
 */
export interface UpdateNotificationConfigRequest {
  triggerType?: string;
  channels?: string[];
  templateId?: string;
  advanceDays?: number;
  retryCount?: number;
  retryIntervalSeconds?: number;
  enabled?: boolean;
}

/**
 * 测试通知请求
 */
export interface TestNotificationRequest {
  configId: number;
  testUserId: string;
}

/**
 * 测试通知结果
 */
export interface TestNotificationResult {
  success: boolean;
  message: string;
  taskId?: number;
}

/**
 * 通知配置查询参数
 */
export interface NotificationConfigParams extends PaginationParams {
  triggerType?: string;
  badgeId?: number;
  benefitId?: number;
  enabled?: boolean;
}

/**
 * 通知任务查询参数
 */
export interface NotificationTaskParams extends PaginationParams {
  configId?: number;
  userId?: string;
  status?: string;
  triggerType?: string;
}

// ============ 触发类型选项 ============

/**
 * 触发类型选项
 */
export const TRIGGER_TYPE_OPTIONS = [
  { value: 'badge_grant', label: '徽章发放' },
  { value: 'badge_expire_remind', label: '徽章即将过期' },
  { value: 'badge_expired', label: '徽章已过期' },
  { value: 'benefit_grant', label: '权益发放' },
  { value: 'benefit_expire_remind', label: '权益即将过期' },
];

/**
 * 通知渠道选项
 */
export const CHANNEL_OPTIONS = [
  { value: 'app_push', label: 'App 推送' },
  { value: 'sms', label: '短信' },
  { value: 'wechat', label: '微信' },
  { value: 'email', label: '邮件' },
  { value: 'in_app', label: '站内信' },
];

/**
 * 任务状态选项
 */
export const TASK_STATUS_OPTIONS = [
  { value: 'pending', label: '待发送', color: 'default' },
  { value: 'sending', label: '发送中', color: 'processing' },
  { value: 'success', label: '成功', color: 'success' },
  { value: 'failed', label: '失败', color: 'error' },
];

// ============ API 函数 ============

/**
 * 获取通知配置列表
 */
export async function getNotificationConfigs(
  params: NotificationConfigParams
): Promise<PaginatedResponse<NotificationConfig>> {
  return get<PaginatedResponse<NotificationConfig>>('/notification-configs', params as Record<string, unknown>);
}

/**
 * 获取单个通知配置
 */
export async function getNotificationConfig(id: number): Promise<NotificationConfig> {
  return get<NotificationConfig>(`/notification-configs/${id}`);
}

/**
 * 创建通知配置
 */
export async function createNotificationConfig(
  data: CreateNotificationConfigRequest
): Promise<NotificationConfig> {
  return post<NotificationConfig>('/notification-configs', data);
}

/**
 * 更新通知配置
 */
export async function updateNotificationConfig(
  id: number,
  data: UpdateNotificationConfigRequest
): Promise<NotificationConfig> {
  return put<NotificationConfig>(`/notification-configs/${id}`, data);
}

/**
 * 删除通知配置
 */
export async function deleteNotificationConfig(id: number): Promise<void> {
  return del(`/notification-configs/${id}`);
}

/**
 * 测试通知发送
 */
export async function testNotification(
  data: TestNotificationRequest
): Promise<TestNotificationResult> {
  return post<TestNotificationResult>('/notification-configs/test', data);
}

/**
 * 获取通知任务列表
 */
export async function getNotificationTasks(
  params: NotificationTaskParams
): Promise<PaginatedResponse<NotificationTask>> {
  return get<PaginatedResponse<NotificationTask>>('/notification-tasks', params as Record<string, unknown>);
}

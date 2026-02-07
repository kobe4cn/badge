/**
 * 兑换管理 API 服务
 *
 * 封装兑换规则的 CRUD 操作和兑换记录查询接口
 */

import { get, post, put, del, getList } from './api';
import type { PaginatedResponse } from '@/types';

/**
 * 频率配置
 */
export interface FrequencyConfig {
  maxPerUser?: number;
  maxPerDay?: number;
  maxPerWeek?: number;
  maxPerMonth?: number;
  maxPerYear?: number;
}

/**
 * 所需徽章
 */
export interface RequiredBadge {
  badgeId: number;
  badgeName: string;
  quantity: number;
}

/**
 * 有效期类型
 */
export type ValidityType = 'FIXED' | 'RELATIVE';

/**
 * 兑换规则
 */
export interface RedemptionRule {
  id: number;
  name: string;
  description?: string;
  benefitId: number;
  benefitName: string;
  requiredBadges: RequiredBadge[];
  frequencyConfig: FrequencyConfig;
  /** 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间 */
  validityType: ValidityType;
  /** 相对有效天数（validityType=RELATIVE 时使用） */
  relativeDays?: number;
  startTime?: string;
  endTime?: string;
  enabled: boolean;
  autoRedeem: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 创建兑换规则请求
 */
export interface CreateRedemptionRuleRequest {
  name: string;
  description?: string;
  benefitId: number;
  requiredBadges: Array<{ badgeId: number; quantity: number }>;
  frequencyConfig?: FrequencyConfig;
  /** 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间 */
  validityType?: ValidityType;
  /** 相对有效天数（validityType=RELATIVE 时使用） */
  relativeDays?: number;
  startTime?: string;
  endTime?: string;
  autoRedeem?: boolean;
}

/**
 * 更新兑换规则请求
 */
export interface UpdateRedemptionRuleRequest {
  name?: string;
  description?: string;
  requiredBadges?: Array<{ badgeId: number; quantity: number }>;
  frequencyConfig?: FrequencyConfig;
  /** 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间 */
  validityType?: ValidityType;
  /** 相对有效天数（validityType=RELATIVE 时使用） */
  relativeDays?: number;
  startTime?: string;
  endTime?: string;
  enabled?: boolean;
  autoRedeem?: boolean;
}

/**
 * 兑换规则查询参数
 */
export interface RedemptionRuleQueryParams {
  page?: number;
  pageSize?: number;
  enabled?: boolean;
  keyword?: string;
  [key: string]: unknown;
}

/**
 * 兑换订单状态
 */
export type RedemptionOrderStatus =
  | 'PENDING'
  | 'PROCESSING'
  | 'COMPLETED'
  | 'FAILED'
  | 'CANCELLED';

/**
 * 兑换订单
 */
export interface RedemptionOrder {
  id: number;
  orderNo: string;
  userId: string;
  ruleId: number;
  ruleName: string;
  benefitId: number;
  benefitName: string;
  status: RedemptionOrderStatus;
  failureReason?: string;
  createdAt: string;
  completedAt?: string;
}

/**
 * 兑换记录查询参数
 */
export interface RedemptionOrderQueryParams {
  page?: number;
  pageSize?: number;
  userId?: string;
  ruleId?: number;
  status?: RedemptionOrderStatus;
  startDate?: string;
  endDate?: string;
  [key: string]: unknown;
}

// ==================== API 方法 ====================

/**
 * 获取兑换规则列表
 */
export function listRedemptionRules(
  params?: RedemptionRuleQueryParams
): Promise<PaginatedResponse<RedemptionRule>> {
  return getList<RedemptionRule>('/admin/redemption/rules', params);
}

/**
 * 获取兑换规则详情
 */
export function getRedemptionRule(id: number): Promise<RedemptionRule> {
  return get<RedemptionRule>(`/admin/redemption/rules/${id}`);
}

/**
 * 创建兑换规则
 */
export function createRedemptionRule(data: CreateRedemptionRuleRequest): Promise<RedemptionRule> {
  return post<RedemptionRule>('/admin/redemption/rules', data);
}

/**
 * 更新兑换规则
 */
export function updateRedemptionRule(
  id: number,
  data: UpdateRedemptionRuleRequest
): Promise<RedemptionRule> {
  return put<RedemptionRule>(`/admin/redemption/rules/${id}`, data);
}

/**
 * 删除兑换规则
 */
export function deleteRedemptionRule(id: number): Promise<void> {
  return del(`/admin/redemption/rules/${id}`);
}

/**
 * 启用/禁用兑换规则
 */
export function toggleRedemptionRule(id: number, enabled: boolean): Promise<RedemptionRule> {
  return put<RedemptionRule>(`/admin/redemption/rules/${id}`, { enabled });
}

/**
 * 获取兑换记录列表
 */
export function listRedemptionOrders(
  params?: RedemptionOrderQueryParams
): Promise<PaginatedResponse<RedemptionOrder>> {
  return getList<RedemptionOrder>('/admin/redemption/orders', params);
}

/**
 * 获取兑换记录详情
 */
export function getRedemptionOrder(orderNo: string): Promise<RedemptionOrder> {
  return get<RedemptionOrder>(`/admin/redemption/orders/${orderNo}`);
}

/**
 * 执行兑换请求
 */
export interface ExecuteRedemptionRequest {
  userId: string;
  ruleId: number;
  idempotencyKey?: string;
}

/**
 * 兑换响应
 */
export interface ExecuteRedemptionResponse {
  success: boolean;
  orderId: number;
  orderNo: string;
  benefitName: string;
  message: string;
}

/**
 * 执行手动兑换
 */
export function executeRedemption(
  data: ExecuteRedemptionRequest
): Promise<ExecuteRedemptionResponse> {
  return post<ExecuteRedemptionResponse>('/admin/redemption/redeem', data);
}

/**
 * 兑换服务对象
 */
export const redemptionService = {
  listRules: listRedemptionRules,
  getRule: getRedemptionRule,
  createRule: createRedemptionRule,
  updateRule: updateRedemptionRule,
  deleteRule: deleteRedemptionRule,
  toggleRule: toggleRedemptionRule,
  listOrders: listRedemptionOrders,
  getOrder: getRedemptionOrder,
  execute: executeRedemption,
};

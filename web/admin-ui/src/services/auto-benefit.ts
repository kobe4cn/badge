/**
 * 自动权益管理 API 服务
 *
 * 封装自动权益发放记录和评估日志的查询接口
 */

import { getList, post } from './api';
import type { PaginatedResponse, ListParams } from '@/types';

/**
 * 自动权益发放记录
 */
export interface AutoBenefitGrant {
  /** 记录 ID */
  id: number;
  /** 用户 ID */
  userId: string;
  /** 规则 ID */
  ruleId: number;
  /** 规则名称 */
  ruleName?: string;
  /** 触发徽章 ID */
  triggerBadgeId: number;
  /** 触发徽章名称 */
  triggerBadgeName?: string;
  /** 关联的权益发放记录 ID */
  benefitGrantId?: number;
  /** 状态: PENDING, PROCESSING, SUCCESS, FAILED, SKIPPED */
  status: 'PENDING' | 'PROCESSING' | 'SUCCESS' | 'FAILED' | 'SKIPPED';
  /** 错误信息 */
  errorMessage?: string;
  /** 创建时间 */
  createdAt: string;
  /** 完成时间 */
  completedAt?: string;
}

/**
 * 评估日志
 */
export interface EvaluationLog {
  /** 日志 ID */
  id: number;
  /** 用户 ID */
  userId: string;
  /** 触发徽章 ID */
  triggerBadgeId: number;
  /** 触发徽章名称 */
  triggerBadgeName?: string;
  /** 评估上下文 */
  evaluationContext: Record<string, unknown>;
  /** 评估的规则数 */
  rulesEvaluated: number;
  /** 匹配的规则数 */
  rulesMatched: number;
  /** 创建的发放记录数 */
  grantsCreated: number;
  /** 评估耗时(毫秒) */
  durationMs: number;
  /** 创建时间 */
  createdAt: string;
}

/**
 * 自动权益发放记录查询参数
 */
export interface AutoBenefitGrantParams extends ListParams {
  /** 用户 ID */
  userId?: string;
  /** 规则 ID */
  ruleId?: number;
  /** 状态 */
  status?: string;
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

/**
 * 评估日志查询参数
 */
export interface EvaluationLogParams extends ListParams {
  /** 用户 ID */
  userId?: string;
  /** 触发徽章 ID */
  triggerBadgeId?: number;
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

/**
 * 获取自动权益发放记录列表
 */
export function listAutoBenefitGrants(
  params: AutoBenefitGrantParams
): Promise<PaginatedResponse<AutoBenefitGrant>> {
  return getList<AutoBenefitGrant>('/admin/auto-benefits/grants', params as Record<string, unknown>);
}

/**
 * 获取评估日志列表
 */
export function listEvaluationLogs(
  params: EvaluationLogParams
): Promise<PaginatedResponse<EvaluationLog>> {
  return getList<EvaluationLog>('/admin/auto-benefits/logs', params as Record<string, unknown>);
}

/**
 * 重试失败的自动权益发放
 */
export function retryAutoGrant(id: number): Promise<AutoBenefitGrant> {
  return post<AutoBenefitGrant>(`/admin/auto-benefits/grants/${id}/retry`);
}

/**
 * 自动权益服务对象
 */
export const autoBenefitService = {
  listGrants: listAutoBenefitGrants,
  listLogs: listEvaluationLogs,
  retryGrant: retryAutoGrant,
};

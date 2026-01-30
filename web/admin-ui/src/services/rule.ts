/**
 * 规则 API 服务
 *
 * 封装规则 CRUD 操作、测试和发布接口
 */

import { get, getList, post, put, patch, del } from './api';
import type { PaginatedResponse, ListParams } from '@/types';

/**
 * 规则状态
 */
export type RuleStatus = 'DRAFT' | 'PUBLISHED' | 'DISABLED' | 'ARCHIVED';

/**
 * 规则实体
 */
export interface Rule {
  id: string;
  /** 规则名称 */
  name: string;
  /** 规则描述 */
  description?: string;
  /** 规则定义 JSON（包含条件和动作配置） */
  ruleJson: Record<string, unknown>;
  /** 规则状态 */
  status: RuleStatus;
  /** 优先级（数值越小优先级越高） */
  priority: number;
  /** 生效开始时间 */
  startTime?: string;
  /** 生效结束时间 */
  endTime?: string;
  /** 版本号 */
  version: number;
  createdAt: string;
  updatedAt: string;
}

/**
 * 规则列表查询参数
 */
export interface RuleListParams extends ListParams {
  /** 名称模糊搜索 */
  name?: string;
  /** 状态筛选 */
  status?: RuleStatus;
}

/**
 * 创建规则请求
 */
export interface CreateRuleRequest {
  name: string;
  description?: string;
  ruleJson: Record<string, unknown>;
  priority?: number;
  startTime?: string;
  endTime?: string;
}

/**
 * 更新规则请求
 */
export interface UpdateRuleRequest {
  name?: string;
  description?: string;
  ruleJson?: Record<string, unknown>;
  priority?: number;
  startTime?: string;
  endTime?: string;
}

/**
 * 规则测试上下文
 *
 * 用于模拟规则评估的运行时环境
 */
export interface TestContext {
  /** 事件类型 */
  eventType: string;
  /** 事件数据 */
  eventData: Record<string, unknown>;
  /** 用户 ID */
  userId: string;
  /** 会员等级 */
  membershipLevel?: string;
  /** 时间戳（ISO 格式） */
  timestamp: string;
  /** 用户属性 */
  user?: Record<string, unknown>;
  /** 订单属性 */
  order?: Record<string, unknown>;
}

/**
 * 条件评估结果
 */
export interface ConditionEvaluation {
  /** 条件节点 ID */
  nodeId: string;
  /** 条件字段 */
  field: string;
  /** 操作符 */
  operator: string;
  /** 期望值 */
  expectedValue: unknown;
  /** 实际值 */
  actualValue: unknown;
  /** 是否匹配 */
  matched: boolean;
}

/**
 * 规则测试结果
 */
export interface RuleTestResult {
  /** 规则是否匹配 */
  matched: boolean;
  /** 各条件的评估详情 */
  conditionResults: ConditionEvaluation[];
  /** 匹配的条件节点 ID 列表（用于画布高亮） */
  matchedNodeIds: string[];
  /** 触发的动作描述 */
  triggeredActions: Array<{
    type: string;
    badgeId?: string;
    badgeName?: string;
    quantity?: number;
  }>;
  /** 评估耗时（毫秒） */
  evaluationTimeMs: number;
  /** 错误信息 */
  error?: string;
}

/**
 * 获取规则分页列表
 */
export function getRules(params: RuleListParams): Promise<PaginatedResponse<Rule>> {
  return getList<Rule>('/admin/rules', params as Record<string, unknown>);
}

/**
 * 获取规则详情
 */
export function getRule(id: string): Promise<Rule> {
  return get<Rule>(`/admin/rules/${id}`);
}

/**
 * 创建规则
 */
export function createRule(data: CreateRuleRequest): Promise<Rule> {
  return post<Rule>('/admin/rules', data);
}

/**
 * 更新规则
 */
export function updateRule(id: string, data: UpdateRuleRequest): Promise<Rule> {
  return put<Rule>(`/admin/rules/${id}`, data);
}

/**
 * 删除规则
 *
 * 仅允许删除草稿状态的规则
 */
export function deleteRule(id: string): Promise<void> {
  return del<void>(`/admin/rules/${id}`);
}

/**
 * 测试规则
 *
 * 使用提供的上下文数据评估规则，返回详细的评估结果
 */
export function testRule(ruleId: string, context: TestContext): Promise<RuleTestResult> {
  return post<RuleTestResult>(`/admin/rules/${ruleId}/test`, context);
}

/**
 * 测试规则定义（不保存）
 *
 * 直接用规则 JSON 进行测试，用于规则编辑时的即时测试
 */
export function testRuleDefinition(
  ruleJson: Record<string, unknown>,
  context: TestContext
): Promise<RuleTestResult> {
  return post<RuleTestResult>('/admin/rules/test', { ruleJson, context });
}

/**
 * 发布规则
 *
 * 将规则状态从 DRAFT 变更为 PUBLISHED
 */
export function publishRule(id: string): Promise<void> {
  return patch<void>(`/admin/rules/${id}/publish`);
}

/**
 * 禁用规则
 *
 * 将规则状态变更为 DISABLED
 */
export function disableRule(id: string): Promise<void> {
  return patch<void>(`/admin/rules/${id}/disable`);
}

/**
 * 规则服务对象
 */
export const ruleService = {
  getList: getRules,
  get: getRule,
  create: createRule,
  update: updateRule,
  delete: deleteRule,
  test: testRule,
  testDefinition: testRuleDefinition,
  publish: publishRule,
  disable: disableRule,
};

/**
 * 规则 API 服务
 *
 * 封装规则 CRUD 操作、测试和发布接口
 */

import { get, getList, post, put, del } from './api';
import type { PaginatedResponse, ListParams } from '@/types';

/**
 * 规则实体
 */
export interface Rule {
  id: number;
  /** 关联的徽章 ID */
  badgeId: number;
  /** 关联的徽章名称（后端返回） */
  badgeName: string;
  /** 事件类型（如 purchase, login, sign_up） */
  eventType: string;
  /** 规则编码（唯一标识） */
  ruleCode: string;
  /** 规则名称（显示用） */
  name?: string;
  /** 规则描述 */
  description?: string;
  /** 规则定义 JSON（包含条件和动作配置） */
  ruleJson: Record<string, unknown>;
  /** 生效开始时间 */
  startTime?: string;
  /** 生效结束时间 */
  endTime?: string;
  /** 每用户最大获取次数 */
  maxCountPerUser?: number;
  /** 全局配额限制 */
  globalQuota?: number;
  /** 已发放数量 */
  globalGranted: number;
  /** 是否启用 */
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/**
 * 规则列表查询参数
 */
export interface RuleListParams extends ListParams {
  /** 徽章 ID 筛选 */
  badgeId?: number;
  /** 事件类型筛选 */
  eventType?: string;
  /** 启用状态筛选 */
  enabled?: boolean;
}

/**
 * 创建规则请求
 */
export interface CreateRuleRequest {
  /** 关联的徽章 ID（必填） */
  badgeId: number;
  /** 规则编码（必填，唯一标识） */
  ruleCode: string;
  /** 事件类型（必填，如 purchase, login, sign_up） */
  eventType: string;
  /** 规则名称（必填，显示用） */
  name: string;
  /** 规则描述 */
  description?: string;
  /** 规则定义 JSON（包含条件和动作配置） */
  ruleJson: Record<string, unknown>;
  /** 生效开始时间 */
  startTime?: string;
  /** 生效结束时间 */
  endTime?: string;
  /** 每用户最大获取次数 */
  maxCountPerUser?: number;
  /** 全局配额限制 */
  globalQuota?: number;
}

/**
 * 更新规则请求
 */
export interface UpdateRuleRequest {
  /** 事件类型 */
  eventType?: string;
  /** 规则编码 */
  ruleCode?: string;
  /** 规则名称（显示用） */
  name?: string;
  /** 规则描述 */
  description?: string;
  /** 规则定义 JSON */
  ruleJson?: Record<string, unknown>;
  /** 生效开始时间 */
  startTime?: string;
  /** 生效结束时间 */
  endTime?: string;
  /** 每用户最大获取次数 */
  maxCountPerUser?: number;
  /** 全局配额限制 */
  globalQuota?: number;
  /** 是否启用 */
  enabled?: boolean;
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
export function getRule(id: number): Promise<Rule> {
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
export function updateRule(id: number, data: UpdateRuleRequest): Promise<Rule> {
  return put<Rule>(`/admin/rules/${id}`, data);
}

/**
 * 删除规则
 *
 * 仅允许删除草稿状态的规则
 */
export function deleteRule(id: number): Promise<void> {
  return del<void>(`/admin/rules/${id}`);
}

/**
 * 测试规则
 *
 * 使用提供的上下文数据评估规则，返回详细的评估结果
 */
export function testRule(ruleId: number, context: TestContext): Promise<RuleTestResult> {
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
export function publishRule(id: number): Promise<void> {
  return post<void>(`/admin/rules/${id}/publish`);
}

/**
 * 禁用规则
 *
 * 将规则状态变更为 DISABLED
 */
export function disableRule(id: number): Promise<void> {
  return post<void>(`/admin/rules/${id}/disable`);
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

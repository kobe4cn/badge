/**
 * 规则 React Query Hooks
 *
 * 封装规则相关的数据查询和变更操作，提供缓存管理
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  getRules,
  getRule,
  createRule,
  updateRule,
  deleteRule,
  testRule,
  testRuleDefinition,
  publishRule,
  disableRule,
  type RuleListParams,
  type CreateRuleRequest,
  type UpdateRuleRequest,
  type TestContext,
} from '@/services/rule';

/**
 * 缓存 key 常量
 */
export const RULE_QUERY_KEYS = {
  all: ['rules'] as const,
  lists: () => [...RULE_QUERY_KEYS.all, 'list'] as const,
  list: (params: RuleListParams) => [...RULE_QUERY_KEYS.lists(), params] as const,
  details: () => [...RULE_QUERY_KEYS.all, 'detail'] as const,
  detail: (id: string) => [...RULE_QUERY_KEYS.details(), id] as const,
};

/**
 * 查询规则列表
 */
export function useRuleList(params: RuleListParams, enabled = true) {
  return useQuery({
    queryKey: RULE_QUERY_KEYS.list(params),
    queryFn: () => getRules(params),
    enabled,
  });
}

/**
 * 查询规则详情
 */
export function useRuleDetail(id: string, enabled = true) {
  return useQuery({
    queryKey: RULE_QUERY_KEYS.detail(id),
    queryFn: () => getRule(id),
    enabled: enabled && !!id,
  });
}

/**
 * 创建规则
 */
export function useCreateRule() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (data: CreateRuleRequest) => createRule(data),
    onSuccess: () => {
      message.success('规则创建成功');
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '创建失败');
    },
  });
}

/**
 * 更新规则
 */
export function useUpdateRule() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateRuleRequest }) =>
      updateRule(id, data),
    onSuccess: (_, variables) => {
      message.success('规则更新成功');
      queryClient.invalidateQueries({
        queryKey: RULE_QUERY_KEYS.detail(variables.id),
      });
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '更新失败');
    },
  });
}

/**
 * 删除规则
 */
export function useDeleteRule() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: string) => deleteRule(id),
    onSuccess: (_data, id) => {
      message.success('规则删除成功');
      queryClient.removeQueries({ queryKey: RULE_QUERY_KEYS.detail(id) });
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '删除失败');
    },
  });
}

/**
 * 测试规则
 *
 * 用于测试已保存的规则
 */
export function useTestRule() {
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({ ruleId, context }: { ruleId: string; context: TestContext }) =>
      testRule(ruleId, context),
    onError: (error: { message?: string }) => {
      message.error(error.message || '测试失败');
    },
  });
}

/**
 * 测试规则定义
 *
 * 用于编辑器中的即时测试，不需要先保存规则
 */
export function useTestRuleDefinition() {
  const { message } = App.useApp();

  return useMutation({
    mutationFn: ({
      ruleJson,
      context,
    }: {
      ruleJson: Record<string, unknown>;
      context: TestContext;
    }) => testRuleDefinition(ruleJson, context),
    onError: (error: { message?: string }) => {
      message.error(error.message || '测试失败');
    },
  });
}

/**
 * 发布规则
 */
export function usePublishRule() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: string) => publishRule(id),
    onSuccess: (_data, id) => {
      message.success('规则已发布');
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.detail(id) });
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '发布失败');
    },
  });
}

/**
 * 禁用规则
 */
export function useDisableRule() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (id: string) => disableRule(id),
    onSuccess: (_data, id) => {
      message.success('规则已禁用');
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.detail(id) });
      queryClient.invalidateQueries({ queryKey: RULE_QUERY_KEYS.lists() });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '禁用失败');
    },
  });
}

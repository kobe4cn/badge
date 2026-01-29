/**
 * 会员管理 React Query Hooks
 *
 * 封装会员相关的数据查询和变更操作，提供缓存管理
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { App } from 'antd';
import {
  searchMembers,
  getMemberDetail,
  getMemberBadges,
  getMemberBadgeStats,
  revokeBadge,
} from '@/services/member';
import type { RevokeBadgeRequest } from '@/types';

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const MEMBER_QUERY_KEYS = {
  all: ['members'] as const,
  search: (keyword: string) => [...MEMBER_QUERY_KEYS.all, 'search', keyword] as const,
  detail: (userId: string) => [...MEMBER_QUERY_KEYS.all, 'detail', userId] as const,
  badges: (userId: string) => [...MEMBER_QUERY_KEYS.all, 'badges', userId] as const,
  badgeStats: (userId: string) => [...MEMBER_QUERY_KEYS.all, 'badgeStats', userId] as const,
};

/**
 * 搜索会员
 *
 * 支持按用户 ID、手机号、昵称模糊搜索
 *
 * @param keyword - 搜索关键词
 * @param enabled - 是否启用查询（关键词为空或长度不足时禁用）
 */
export function useSearchMembers(keyword: string, enabled = true) {
  return useQuery({
    queryKey: MEMBER_QUERY_KEYS.search(keyword),
    queryFn: () => searchMembers(keyword),
    enabled: enabled && keyword.length >= 2,
    staleTime: 30 * 1000,
  });
}

/**
 * 获取会员详情
 *
 * @param userId - 用户 ID
 * @param enabled - 是否启用查询
 */
export function useMemberDetail(userId: string, enabled = true) {
  return useQuery({
    queryKey: MEMBER_QUERY_KEYS.detail(userId),
    queryFn: () => getMemberDetail(userId),
    enabled: enabled && !!userId,
  });
}

/**
 * 获取会员徽章列表
 *
 * @param userId - 用户 ID
 * @param enabled - 是否启用查询
 */
export function useMemberBadges(userId: string, enabled = true) {
  return useQuery({
    queryKey: MEMBER_QUERY_KEYS.badges(userId),
    queryFn: () => getMemberBadges(userId),
    enabled: enabled && !!userId,
  });
}

/**
 * 获取会员徽章统计
 *
 * @param userId - 用户 ID
 * @param enabled - 是否启用查询
 */
export function useMemberBadgeStats(userId: string, enabled = true) {
  return useQuery({
    queryKey: MEMBER_QUERY_KEYS.badgeStats(userId),
    queryFn: () => getMemberBadgeStats(userId),
    enabled: enabled && !!userId,
  });
}

/**
 * 撤销用户徽章
 *
 * 返回 mutation 用于触发撤销操作
 */
export function useRevokeBadge() {
  const queryClient = useQueryClient();
  const { message } = App.useApp();

  return useMutation({
    mutationFn: (request: RevokeBadgeRequest) => revokeBadge(request),
    onSuccess: () => {
      message.success('徽章已撤销');
      // 刷新会员徽章相关缓存
      queryClient.invalidateQueries({
        predicate: (query) => {
          const key = query.queryKey;
          return (
            Array.isArray(key) &&
            key[0] === 'members' &&
            (key[1] === 'badges' || key[1] === 'badgeStats')
          );
        },
      });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '撤销失败');
    },
  });
}

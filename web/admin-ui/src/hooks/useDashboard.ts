/**
 * 数据看板 React Query Hooks
 *
 * 封装数据看板的数据查询操作，支持自动刷新和缓存管理
 */

import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getDashboardStats,
  getTodayStats,
  getBadgeRanking,
  getGrantTrend,
  getBadgeTypeDistribution,
  getUserActivityTrend,
  getTopBadges,
} from '@/services/dashboard';
import type { RankingParams, TrendParams } from '@/types/dashboard';

/**
 * 默认自动刷新间隔（5 分钟）
 */
const DEFAULT_REFETCH_INTERVAL = 5 * 60 * 1000;

/**
 * 缓存 key 常量
 *
 * 集中管理避免拼写错误，便于缓存失效处理
 */
export const DASHBOARD_QUERY_KEYS = {
  all: ['dashboard'] as const,
  stats: () => [...DASHBOARD_QUERY_KEYS.all, 'stats'] as const,
  today: () => [...DASHBOARD_QUERY_KEYS.all, 'today'] as const,
  ranking: (params?: RankingParams) =>
    [...DASHBOARD_QUERY_KEYS.all, 'ranking', params] as const,
  grantTrend: (params: TrendParams) =>
    [...DASHBOARD_QUERY_KEYS.all, 'grantTrend', params] as const,
  typeDistribution: () =>
    [...DASHBOARD_QUERY_KEYS.all, 'typeDistribution'] as const,
  activityTrend: (params: TrendParams) =>
    [...DASHBOARD_QUERY_KEYS.all, 'activityTrend', params] as const,
  topBadges: (limit: number) =>
    [...DASHBOARD_QUERY_KEYS.all, 'topBadges', limit] as const,
};

/**
 * 查询看板概览统计数据
 *
 * @param options - 查询选项
 * @param options.enabled - 是否启用查询
 * @param options.refetchInterval - 自动刷新间隔（ms），默认 5 分钟
 */
export function useDashboardStats(options?: {
  enabled?: boolean;
  refetchInterval?: number | false;
}) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.stats(),
    queryFn: getDashboardStats,
    enabled,
    refetchInterval,
    // 统计数据可以在切换标签页后刷新以保持最新
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询今日统计数据
 *
 * @param options - 查询选项
 * @param options.enabled - 是否启用查询
 * @param options.refetchInterval - 自动刷新间隔（ms），默认 5 分钟
 */
export function useTodayStats(options?: {
  enabled?: boolean;
  refetchInterval?: number | false;
}) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.today(),
    queryFn: getTodayStats,
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询徽章排行榜
 *
 * @param params - 排行榜查询参数
 * @param options - 查询选项
 */
export function useBadgeRanking(
  params?: RankingParams,
  options?: {
    enabled?: boolean;
    refetchInterval?: number | false;
  }
) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.ranking(params),
    queryFn: () => getBadgeRanking(params),
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询发放趋势数据
 *
 * @param params - 趋势查询参数
 * @param options - 查询选项
 */
export function useGrantTrend(
  params: TrendParams,
  options?: {
    enabled?: boolean;
    refetchInterval?: number | false;
  }
) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.grantTrend(params),
    queryFn: () => getGrantTrend(params),
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询徽章类型分布数据
 *
 * @param options - 查询选项
 */
export function useBadgeTypeDistribution(options?: {
  enabled?: boolean;
  refetchInterval?: number | false;
}) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.typeDistribution(),
    queryFn: getBadgeTypeDistribution,
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询用户活跃度趋势
 *
 * @param params - 趋势查询参数
 * @param options - 查询选项
 */
export function useUserActivityTrend(
  params: TrendParams,
  options?: {
    enabled?: boolean;
    refetchInterval?: number | false;
  }
) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.activityTrend(params),
    queryFn: () => getUserActivityTrend(params),
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 查询热门徽章 Top N
 *
 * @param limit - 返回数量，默认 10
 * @param options - 查询选项
 */
export function useTopBadges(
  limit: number = 10,
  options?: {
    enabled?: boolean;
    refetchInterval?: number | false;
  }
) {
  const { enabled = true, refetchInterval = DEFAULT_REFETCH_INTERVAL } = options || {};

  return useQuery({
    queryKey: DASHBOARD_QUERY_KEYS.topBadges(limit),
    queryFn: () => getTopBadges(limit),
    enabled,
    refetchInterval,
    refetchOnWindowFocus: true,
  });
}

/**
 * 手动刷新所有看板数据
 *
 * 用于提供手动刷新按钮功能
 */
export function useRefreshDashboard() {
  const queryClient = useQueryClient();

  const refresh = () => {
    // 使看板相关的所有缓存失效并重新获取
    queryClient.invalidateQueries({ queryKey: DASHBOARD_QUERY_KEYS.all });
  };

  return { refresh };
}

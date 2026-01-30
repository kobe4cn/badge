/**
 * 数据看板 API 服务
 *
 * 封装数据看板统计相关的接口调用
 */

import { get } from './api';
import type {
  DashboardStats,
  TodayStats,
  BadgeRanking,
  RankingParams,
  TrendParams,
  TrendData,
  TypeDistribution,
} from '@/types/dashboard';

/**
 * 获取看板概览统计数据
 *
 * 返回徽章系统的整体指标
 */
export function getDashboardStats(): Promise<DashboardStats> {
  return get<DashboardStats>('/admin/stats/overview');
}

/**
 * 获取今日统计数据
 *
 * 返回当日运营数据及环比变化
 */
export function getTodayStats(): Promise<TodayStats> {
  return get<TodayStats>('/admin/stats/today');
}

/**
 * 获取徽章排行榜
 *
 * @param params - 排行榜查询参数
 */
export function getBadgeRanking(params?: RankingParams): Promise<BadgeRanking[]> {
  return get<BadgeRanking[]>('/admin/stats/ranking', params as Record<string, unknown>);
}

/**
 * 获取发放趋势数据
 *
 * @param params - 趋势查询参数
 * @returns 按时间排序的趋势数据数组
 */
export function getGrantTrend(params: TrendParams): Promise<TrendData[]> {
  return get<TrendData[]>('/admin/stats/trends', params as unknown as Record<string, unknown>);
}

/**
 * 获取徽章类型分布数据
 *
 * 用于饼图展示各类型徽章的发放占比
 */
export function getBadgeTypeDistribution(): Promise<TypeDistribution[]> {
  return get<TypeDistribution[]>('/admin/stats/distribution/types');
}

/**
 * 获取用户活跃度趋势数据
 *
 * @param params - 趋势查询参数
 * @returns 按时间排序的用户活跃度趋势
 */
export function getUserActivityTrend(params: TrendParams): Promise<TrendData[]> {
  return get<TrendData[]>('/admin/stats/trend/activity', params as unknown as Record<string, unknown>);
}

/**
 * 获取热门徽章排行榜
 *
 * @param limit - 返回数量，默认 10
 * @returns 按发放量排序的热门徽章列表
 */
export function getTopBadges(limit: number = 10): Promise<BadgeRanking[]> {
  return get<BadgeRanking[]>('/admin/stats/ranking', { type: 'grant', limit });
}

/**
 * 数据看板服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const dashboardService = {
  getStats: getDashboardStats,
  getTodayStats: getTodayStats,
  getRanking: getBadgeRanking,
  getGrantTrend,
  getBadgeTypeDistribution,
  getUserActivityTrend,
  getTopBadges,
};

/**
 * 数据看板 API 服务
 *
 * 封装数据看板统计相关的接口调用
 */

import { get } from './api';
import type { DashboardStats, TodayStats, BadgeRanking, RankingParams } from '@/types/dashboard';

/**
 * 获取看板概览统计数据
 *
 * 返回徽章系统的整体指标
 */
export function getDashboardStats(): Promise<DashboardStats> {
  return get<DashboardStats>('/api/v1/dashboard/stats');
}

/**
 * 获取今日统计数据
 *
 * 返回当日运营数据及环比变化
 */
export function getTodayStats(): Promise<TodayStats> {
  return get<TodayStats>('/api/v1/dashboard/today');
}

/**
 * 获取徽章排行榜
 *
 * @param params - 排行榜查询参数
 */
export function getBadgeRanking(params?: RankingParams): Promise<BadgeRanking[]> {
  return get<BadgeRanking[]>('/api/v1/dashboard/ranking', params as Record<string, unknown>);
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
};

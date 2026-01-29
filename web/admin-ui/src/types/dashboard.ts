/**
 * 数据看板类型定义
 *
 * 定义数据看板页面所需的统计数据结构
 */

/**
 * 总体统计数据
 *
 * 展示徽章系统的核心指标概览
 */
export interface DashboardStats {
  /** 总发放次数 */
  totalGrants: number;
  /** 系统总用户数 */
  totalUsers: number;
  /** 持有徽章的用户数 */
  badgeHolders: number;
  /** 活跃状态的徽章数（已上线的徽章类型） */
  activeBadges: number;
  /** 徽章类型总数（所有状态） */
  totalBadgeTypes: number;
  /** 用户覆盖率（badgeHolders / totalUsers） */
  userCoverageRate: number;
  /** 累计兑换次数 */
  redemptionCount: number;
  /** 兑换率（已兑换 / 总发放） */
  redemptionRate: number;
}

/**
 * 今日统计数据
 *
 * 展示当日运营指标及环比变化
 */
export interface TodayStats {
  /** 今日发放次数 */
  grants: number;
  /** 较昨日变化百分比（正数表示上涨） */
  grantsChange: number;
  /** 今日新增徽章持有者数 */
  newHolders: number;
  /** 较昨日变化百分比 */
  holdersChange: number;
  /** 今日兑换次数 */
  redemptions: number;
  /** 较昨日变化百分比 */
  redemptionsChange: number;
}

/**
 * 徽章排行查询参数
 */
export interface RankingParams {
  /** 排行类型 */
  type?: 'grant' | 'holder';
  /** 返回数量限制 */
  limit?: number;
  /** 时间范围：today/week/month/all */
  period?: 'today' | 'week' | 'month' | 'all';
}

/**
 * 徽章排行项
 *
 * 用于展示发放量/持有量排行榜
 */
export interface BadgeRanking {
  /** 徽章 ID */
  badgeId: number;
  /** 徽章名称 */
  badgeName: string;
  /** 徽章图标 URL */
  badgeIcon: string;
  /** 发放次数 */
  grantCount: number;
  /** 持有人数 */
  holderCount: number;
  /** 排名 */
  rank: number;
}

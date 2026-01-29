/**
 * 格式化工具函数
 *
 * 提供日期、金额、状态等常用格式化方法
 */

import dayjs from 'dayjs';
import type { BadgeStatus, BadgeType, MembershipLevel, CategoryStatus } from '@/types';

/**
 * 日期格式化
 *
 * @param date - ISO 8601 日期字符串或 Date 对象
 * @param format - 输出格式，默认 'YYYY-MM-DD HH:mm:ss'
 */
export function formatDate(
  date: string | Date | undefined | null,
  format = 'YYYY-MM-DD HH:mm:ss'
): string {
  if (!date) return '-';
  return dayjs(date).format(format);
}

/**
 * 日期时间格式化（formatDate 的别名）
 *
 * @param date - ISO 8601 日期字符串或 Date 对象
 */
export function formatDateTime(date: string | Date | undefined | null): string {
  return formatDate(date, 'YYYY-MM-DD HH:mm:ss');
}

/**
 * 相对时间格式化
 *
 * 显示如"3分钟前"、"2小时前"等相对时间
 */
export function formatRelativeTime(date: string | Date | undefined | null): string {
  if (!date) return '-';
  const now = dayjs();
  const target = dayjs(date);
  const diffMinutes = now.diff(target, 'minute');

  if (diffMinutes < 1) return '刚刚';
  if (diffMinutes < 60) return `${diffMinutes}分钟前`;
  if (diffMinutes < 1440) return `${Math.floor(diffMinutes / 60)}小时前`;
  if (diffMinutes < 43200) return `${Math.floor(diffMinutes / 1440)}天前`;
  return formatDate(date, 'YYYY-MM-DD');
}

/**
 * 金额格式化
 *
 * @param amount - 金额数值
 * @param decimals - 小数位数，默认 2
 */
export function formatAmount(amount: number | undefined | null, decimals = 2): string {
  if (amount === undefined || amount === null) return '-';
  return amount.toLocaleString('zh-CN', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/**
 * 数量格式化
 *
 * 大数值使用 K/M/B 后缀简化显示
 */
export function formatCount(count: number | undefined | null): string {
  if (count === undefined || count === null) return '-';
  if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`;
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
  return count.toString();
}

/**
 * 百分比格式化
 */
export function formatPercent(value: number | undefined | null, decimals = 1): string {
  if (value === undefined || value === null) return '-';
  return `${(value * 100).toFixed(decimals)}%`;
}

// ============ 状态文本映射 ============

/**
 * 徽章状态文本
 */
export const BADGE_STATUS_TEXT: Record<BadgeStatus, string> = {
  DRAFT: '草稿',
  ACTIVE: '已上线',
  INACTIVE: '已下线',
  ARCHIVED: '已归档',
};

/**
 * 徽章类型文本
 */
export const BADGE_TYPE_TEXT: Record<BadgeType, string> = {
  NORMAL: '普通徽章',
  LIMITED: '限定徽章',
  ACHIEVEMENT: '成就徽章',
  EVENT: '活动徽章',
};

/**
 * 分类状态文本
 */
export const CATEGORY_STATUS_TEXT: Record<CategoryStatus, string> = {
  ACTIVE: '启用',
  INACTIVE: '禁用',
};

/**
 * 会员等级文本
 */
export const MEMBERSHIP_LEVEL_TEXT: Record<MembershipLevel, string> = {
  Bronze: '青铜会员',
  Silver: '白银会员',
  Gold: '黄金会员',
  Platinum: '铂金会员',
  Diamond: '钻石会员',
};

/**
 * 获取徽章状态文本
 */
export function getBadgeStatusText(status: BadgeStatus): string {
  return BADGE_STATUS_TEXT[status] || status;
}

/**
 * 获取徽章类型文本
 */
export function getBadgeTypeText(type: BadgeType): string {
  return BADGE_TYPE_TEXT[type] || type;
}

/**
 * 获取分类状态文本
 */
export function getCategoryStatusText(status: CategoryStatus): string {
  return CATEGORY_STATUS_TEXT[status] || status;
}

/**
 * 获取会员等级文本
 */
export function getMembershipLevelText(level: MembershipLevel): string {
  return MEMBERSHIP_LEVEL_TEXT[level] || level;
}

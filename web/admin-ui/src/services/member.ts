/**
 * 会员管理 API 服务
 *
 * 封装会员查询、徽章查询和撤销等接口
 */

import { get, post, getList } from './api';
import type {
  User,
  MemberDetail,
  MemberBadgeStats,
  UserBadgeDetail,
  RevokeBadgeRequest,
  UserLedgerEntry,
  UserBenefit,
  UserRedemptionHistory,
  PaginatedResponse,
  PaginationParams,
} from '@/types';

/**
 * 搜索会员
 *
 * 支持按用户 ID、手机号、昵称模糊搜索
 *
 * @param keyword - 搜索关键词
 */
export function searchMembers(keyword: string): Promise<User[]> {
  return get<User[]>('/admin/users/search', { keyword });
}

/**
 * 获取会员详情
 *
 * @param userId - 用户 ID
 */
export function getMemberDetail(userId: string): Promise<MemberDetail> {
  return get<MemberDetail>(`/admin/users/${userId}`);
}

/**
 * 获取会员徽章列表
 *
 * @param userId - 用户 ID
 */
export function getMemberBadges(userId: string): Promise<UserBadgeDetail[]> {
  return get<UserBadgeDetail[]>(`/admin/users/${userId}/badges`);
}

/**
 * 获取会员徽章统计
 *
 * @param userId - 用户 ID
 */
export function getMemberBadgeStats(userId: string): Promise<MemberBadgeStats> {
  return get<MemberBadgeStats>(`/admin/users/${userId}/stats`);
}

/**
 * 撤销用户徽章
 *
 * @param request - 撤销请求，包含用户徽章 ID 和撤销原因
 */
export function revokeBadge(request: RevokeBadgeRequest): Promise<void> {
  return post<void>('/admin/revokes/manual', {
    userBadgeId: request.userBadgeId,
    reason: request.reason,
  });
}

/**
 * 获取用户账本流水
 *
 * 返回用户在 badge_ledger 中的所有变动记录
 *
 * @param userId - 用户 ID
 * @param params - 分页参数
 */
export function getMemberLedger(
  userId: string,
  params?: PaginationParams
): Promise<PaginatedResponse<UserLedgerEntry>> {
  return getList<UserLedgerEntry>(`/admin/users/${userId}/ledger`, params as Record<string, unknown>);
}

/**
 * 获取用户权益列表
 *
 * @param userId - 用户 ID
 * @param params - 分页参数
 */
export function getMemberBenefits(
  userId: string,
  params?: PaginationParams
): Promise<PaginatedResponse<UserBenefit>> {
  return getList<UserBenefit>(`/admin/users/${userId}/benefits`, params as Record<string, unknown>);
}

/**
 * 获取用户兑换历史
 *
 * @param userId - 用户 ID
 * @param params - 分页参数
 */
export function getMemberRedemptionHistory(
  userId: string,
  params?: PaginationParams
): Promise<PaginatedResponse<UserRedemptionHistory>> {
  return getList<UserRedemptionHistory>(`/admin/users/${userId}/redemption-history`, params as Record<string, unknown>);
}

/**
 * 会员服务对象
 *
 * 提供面向对象风格的 API 调用方式
 */
export const memberService = {
  searchMembers,
  getMemberDetail,
  getMemberBadges,
  getMemberBadgeStats,
  revokeBadge,
  getMemberLedger,
  getMemberBenefits,
  getMemberRedemptionHistory,
};

/**
 * 会员相关类型定义
 *
 * 用于会员徽章查询页面的数据结构
 */

import type { UserBadgeStatus, BadgeType } from './badge';
import type { SourceType } from './grant';

/**
 * 会员详情
 *
 * 包含用户的完整信息，用于会员详情展示
 */
export interface MemberDetail {
  /** 用户 ID */
  userId: string;
  /** 昵称 */
  nickname: string;
  /** 头像 URL */
  avatar: string;
  /** 手机号 */
  phone?: string;
  /** 会员等级 */
  membershipLevel: string;
  /** 注册时间 */
  registeredAt: string;
  /** 最后活跃时间 */
  lastActiveAt: string;
}

/**
 * 会员徽章统计
 *
 * 用户持有的徽章分布情况
 */
export interface MemberBadgeStats {
  /** 总徽章数 */
  totalBadges: number;
  /** 有效徽章数 */
  activeBadges: number;
  /** 已过期徽章数 */
  expiredBadges: number;
  /** 已撤销徽章数 */
  revokedBadges: number;
  /** 徽章种类数 */
  totalTypes: number;
}

/**
 * 用户徽章详情
 *
 * 包含徽章定义信息的用户徽章记录
 */
export interface UserBadgeDetail {
  /** 用户徽章 ID */
  id: number;
  /** 用户 ID */
  userId: string;
  /** 徽章定义 ID */
  badgeId: number;
  /** 徽章名称 */
  badgeName: string;
  /** 徽章图标 URL */
  badgeIcon: string;
  /** 徽章类型 */
  badgeType: BadgeType;
  /** 徽章描述 */
  badgeDescription?: string;
  /** 用户徽章状态 */
  status: UserBadgeStatus;
  /** 持有数量 */
  quantity: number;
  /** 来源类型 */
  sourceType: SourceType;
  /** 来源引用 ID */
  sourceRefId?: string;
  /** 发放原因 */
  grantReason?: string;
  /** 获取时间 */
  grantedAt: string;
  /** 过期时间 */
  expiresAt?: string;
  /** 撤销时间 */
  revokedAt?: string;
  /** 撤销原因 */
  revokedReason?: string;
  /** 操作人 */
  operatorName?: string;
}

/**
 * 撤销徽章请求
 */
export interface RevokeBadgeRequest {
  /** 用户徽章 ID */
  userBadgeId: number;
  /** 撤销原因 */
  reason: string;
}

/**
 * 用户账本流水记录
 *
 * 对应后端 UserLedgerDto，记录用户徽章的所有变动（获取/撤销/兑换）
 */
export interface UserLedgerEntry {
  id: number;
  badgeId: number;
  badgeName: string;
  /** 变动类型：GRANT / REVOKE / REDEEM / EXPIRE 等 */
  changeType: string;
  /** 来源类型：EVENT / MANUAL / SYSTEM 等 */
  sourceType: string;
  /** 变动数量（正为获得，负为扣减） */
  quantity: number;
  remark?: string;
  createdAt: string;
}

/**
 * 用户权益记录
 *
 * 对应后端 UserBenefitDto，展示用户获得的权益发放记录
 */
export interface UserBenefit {
  grantId: number;
  grantNo: string;
  benefitId: number;
  benefitName: string;
  benefitType: string;
  status: string;
  grantedAt?: string;
  expiresAt?: string;
  createdAt: string;
}

/**
 * 用户兑换历史记录
 *
 * 对应后端 RedemptionOrderDto
 */
export interface UserRedemptionHistory {
  id: number;
  orderNo: string;
  userId: string;
  ruleId: number;
  ruleName: string;
  benefitId: number;
  benefitName: string;
  status: string;
  failureReason?: string;
  consumedBadges: Array<{
    badgeId: number;
    badgeName: string;
    quantity: number;
  }>;
  createdAt: string;
}

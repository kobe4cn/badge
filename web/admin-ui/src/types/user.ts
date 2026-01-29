/**
 * 用户相关类型定义
 *
 * 对应后端 mock-services 的用户模型，
 * 用于会员管理和徽章发放场景
 */

/**
 * 会员等级
 *
 * 五级会员体系，与消费金额关联
 */
export type MembershipLevel = 'Bronze' | 'Silver' | 'Gold' | 'Platinum' | 'Diamond';

/**
 * 用户信息
 *
 * 从外部用户系统获取的基本信息
 */
export interface User {
  /** 用户 ID */
  userId: string;
  /** 用户名 */
  username: string;
  /** 邮箱 */
  email: string;
  /** 手机号 */
  phone?: string;
  /** 会员等级 */
  membershipLevel: MembershipLevel;
  /** 注册时间 */
  registrationDate: string;
  /** 累计消费金额 */
  totalSpent: number;
  /** 订单数量 */
  orderCount: number;
}

/**
 * 用户档案
 *
 * 包含用户的完整信息，用于详情页展示
 */
export interface UserProfile extends User {
  /** 头像 URL */
  avatarUrl?: string;
  /** 昵称 */
  nickname?: string;
  /** 最后登录时间 */
  lastLoginAt?: string;
  /** 账号状态 */
  status: UserStatus;
}

/**
 * 用户状态
 */
export type UserStatus = 'ACTIVE' | 'INACTIVE' | 'BANNED';

/**
 * 用户搜索参数
 *
 * 用于用户列表的筛选条件
 */
export interface UserSearchParams {
  /** 关键词（用户ID/用户名/邮箱） */
  keyword?: string;
  /** 会员等级 */
  membershipLevel?: MembershipLevel;
  /** 注册开始日期 */
  registeredFrom?: string;
  /** 注册结束日期 */
  registeredTo?: string;
}

/**
 * 会员等级配置
 *
 * 定义各等级的权益和门槛
 */
export interface MembershipLevelConfig {
  /** 等级标识 */
  level: MembershipLevel;
  /** 等级名称 */
  name: string;
  /** 达到该等级所需的消费金额 */
  threshold: number;
  /** 等级颜色 */
  color: string;
  /** 等级图标 */
  icon?: string;
}

/**
 * 会员等级配置常量
 *
 * 与后端 MembershipLevel::from_spent 阈值保持一致
 */
export const MEMBERSHIP_LEVELS: MembershipLevelConfig[] = [
  { level: 'Bronze', name: '青铜会员', threshold: 0, color: '#CD7F32' },
  { level: 'Silver', name: '白银会员', threshold: 1000, color: '#C0C0C0' },
  { level: 'Gold', name: '黄金会员', threshold: 5000, color: '#FFD700' },
  { level: 'Platinum', name: '铂金会员', threshold: 20000, color: '#E5E4E2' },
  { level: 'Diamond', name: '钻石会员', threshold: 50000, color: '#B9F2FF' },
];

/**
 * 根据会员等级获取配置
 */
export function getMembershipConfig(level: MembershipLevel): MembershipLevelConfig {
  return MEMBERSHIP_LEVELS.find((config) => config.level === level) || MEMBERSHIP_LEVELS[0];
}

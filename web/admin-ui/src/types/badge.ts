/**
 * 徽章相关类型定义
 *
 * 对应后端 badge-management-service 的实体定义，
 * 包含三层结构：Category -> Series -> Badge
 */

/**
 * 徽章类型
 *
 * 区分不同性质的徽章，影响获取方式和展示逻辑
 */
export type BadgeType = 'NORMAL' | 'LIMITED' | 'ACHIEVEMENT' | 'EVENT';

/**
 * 徽章状态（运营侧）
 *
 * 控制徽章对用户的可见性和可获取性
 */
export type BadgeStatus = 'DRAFT' | 'ACTIVE' | 'INACTIVE' | 'ARCHIVED';

/**
 * 分类/系列状态
 */
export type CategoryStatus = 'ACTIVE' | 'INACTIVE';

/**
 * 有效期类型
 *
 * 决定徽章过期时间的计算方式
 */
export type ValidityType = 'PERMANENT' | 'FIXED_DATE' | 'RELATIVE_DAYS';

/**
 * 徽章大类（一级分类）
 *
 * 用于统计和顶层分类，如"交易徽章"、"互动徽章"等
 */
export interface BadgeCategory {
  id: number;
  /** 分类名称 */
  name: string;
  /** 分类图标 URL */
  iconUrl?: string;
  /** 排序权重，数值越小越靠前 */
  sortOrder: number;
  /** 分类状态 */
  status: CategoryStatus;
  createdAt: string;
  updatedAt: string;
}

/**
 * 徽章系列（二级分类）
 *
 * 用于分组展示，如"2024春节系列"、"周年庆系列"
 */
export interface BadgeSeries {
  id: number;
  /** 所属大类 ID */
  categoryId: number;
  /** 系列名称 */
  name: string;
  /** 系列描述 */
  description?: string;
  /** 系列封面图 URL */
  coverUrl?: string;
  /** 排序权重 */
  sortOrder: number;
  /** 系列状态 */
  status: CategoryStatus;
  /** 系列开始时间（用于限时系列） */
  startTime?: string;
  /** 系列结束时间 */
  endTime?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 有效期配置
 *
 * 嵌入在 Badge 的 validityConfig 字段中
 */
export interface ValidityConfig {
  /** 有效期类型 */
  validityType: ValidityType;
  /** 固定过期日期（validityType = FIXED_DATE 时使用） */
  fixedDate?: string;
  /** 相对有效天数（validityType = RELATIVE_DAYS 时使用） */
  relativeDays?: number;
}

/**
 * 徽章资源配置
 *
 * 存储徽章的各种展示资源
 */
export interface BadgeAssets {
  /** 徽章图标（小图） */
  iconUrl: string;
  /** 徽章大图 */
  imageUrl?: string;
  /** 动效资源（Lottie 或视频） */
  animationUrl?: string;
  /** 灰态图标（未获取时展示） */
  disabledIconUrl?: string;
}

/**
 * 徽章定义
 *
 * 实际发放给用户的徽章实体
 */
export interface Badge {
  id: number;
  /** 所属系列 ID */
  seriesId: number;
  /** 徽章类型 */
  badgeType: BadgeType;
  /** 徽章名称 */
  name: string;
  /** 业务唯一编码（用于外部系统对接） */
  code?: string;
  /** 徽章描述 */
  description?: string;
  /** 获取条件描述（展示给用户） */
  obtainDescription?: string;
  /** 排序权重 */
  sortOrder: number;
  /** 徽章状态 */
  status: BadgeStatus;
  /** 资源配置 */
  assets: BadgeAssets;
  /** 有效期配置 */
  validityConfig: ValidityConfig;
  /** 最大发放总量（null 表示不限量） */
  maxSupply?: number;
  /** 已发放数量 */
  issuedCount: number;
  createdAt: string;
  updatedAt: string;
}

/**
 * 用户徽章状态
 */
export type UserBadgeStatus = 'ACTIVE' | 'EXPIRED' | 'REVOKED' | 'REDEEMED';

/**
 * 用户徽章
 *
 * 记录用户持有的徽章实例
 */
export interface UserBadge {
  id: number;
  /** 用户 ID */
  userId: string;
  /** 徽章定义 ID */
  badgeId: number;
  /** 徽章状态 */
  status: UserBadgeStatus;
  /** 持有数量 */
  quantity: number;
  /** 获取时间 */
  acquiredAt: string;
  /** 过期时间（null 表示永久有效） */
  expiresAt?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 用户徽章汇总
 *
 * 用于展示用户的徽章统计信息
 */
export interface UserBadgeSummary {
  /** 用户 ID */
  userId: string;
  /** 总徽章种类数 */
  totalBadgeTypes: number;
  /** 总徽章数量 */
  totalQuantity: number;
  /** 有效徽章数量 */
  activeQuantity: number;
  /** 已过期数量 */
  expiredQuantity: number;
  /** 已兑换数量 */
  redeemedQuantity: number;
}

// ============ 表单数据类型 ============

/**
 * 创建徽章分类请求
 */
export interface CreateCategoryRequest {
  name: string;
  iconUrl?: string;
  sortOrder?: number;
}

/**
 * 更新徽章分类请求
 */
export interface UpdateCategoryRequest {
  name?: string;
  iconUrl?: string;
  sortOrder?: number;
}

/**
 * 创建徽章系列请求
 */
export interface CreateSeriesRequest {
  categoryId: number;
  name: string;
  description?: string;
  coverUrl?: string;
  /** 排序权重（后端通过独立 /sort 端点处理，此处仅保留表单兼容） */
  sortOrder?: number;
  startTime?: string;
  endTime?: string;
}

/**
 * 更新徽章系列请求
 */
export interface UpdateSeriesRequest {
  name?: string;
  description?: string;
  coverUrl?: string;
  /** 排序权重（后端通过独立 /sort 端点处理） */
  sortOrder?: number;
  startTime?: string;
  endTime?: string;
}

/**
 * 创建徽章请求
 */
export interface CreateBadgeRequest {
  seriesId: number;
  badgeType: BadgeType;
  name: string;
  /** 业务唯一编码（用于外部系统对接） */
  code?: string;
  description?: string;
  obtainDescription?: string;
  /** 排序权重（后端通过独立 /sort 端点处理） */
  sortOrder?: number;
  assets: BadgeAssets;
  validityConfig: ValidityConfig;
  maxSupply?: number;
}

/**
 * 更新徽章请求
 */
export interface UpdateBadgeRequest {
  name?: string;
  /** 业务唯一编码 */
  code?: string;
  description?: string;
  obtainDescription?: string;
  status?: BadgeStatus;
  assets?: BadgeAssets;
  validityConfig?: ValidityConfig;
  maxSupply?: number;
}

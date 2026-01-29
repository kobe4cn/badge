/**
 * 发放管理相关类型定义
 *
 * 对应后端 badge-admin-service 的发放和审计实体
 */

/**
 * 来源类型
 *
 * 标识徽章变动的触发来源
 */
export type SourceType = 'EVENT' | 'SCHEDULED' | 'MANUAL' | 'REDEMPTION' | 'SYSTEM';

/**
 * 日志动作类型
 */
export type LogAction = 'GRANT' | 'REVOKE' | 'REDEEM' | 'EXPIRE';

/**
 * 发放记录
 *
 * 记录单次徽章发放操作
 */
export interface GrantRecord {
  id: number;
  /** 用户 ID */
  userId: string;
  /** 用户名（冗余存储） */
  username?: string;
  /** 徽章 ID */
  badgeId: number;
  /** 徽章名称（冗余存储） */
  badgeName: string;
  /** 发放数量 */
  quantity: number;
  /** 发放原因 */
  reason?: string;
  /** 来源类型 */
  sourceType: SourceType;
  /** 来源引用 ID（如批量任务 ID） */
  sourceRefId?: string;
  /** 操作人 */
  operator: string;
  /** 发放时间 */
  createdAt: string;
}

/**
 * 发放日志
 *
 * 用于审计追踪的操作日志
 */
export interface GrantLog {
  id: number;
  /** 关联的用户徽章 ID */
  userBadgeId: number;
  /** 用户 ID */
  userId: string;
  /** 徽章 ID */
  badgeId: number;
  /** 操作动作 */
  action: LogAction;
  /** 操作原因 */
  reason?: string;
  /** 操作人 */
  operator?: string;
  /** 操作数量 */
  quantity: number;
  /** 来源类型 */
  sourceType: SourceType;
  /** 来源引用 ID */
  sourceRefId?: string;
  /** 操作时间 */
  createdAt: string;
}

/**
 * 批量任务类型
 */
export type BatchTaskType = 'batch_grant' | 'batch_revoke' | 'data_export';

/**
 * 批量任务状态
 */
export type BatchTaskStatus = 'pending' | 'processing' | 'completed' | 'failed';

/**
 * 批量任务
 *
 * 记录批量发放/取消等异步任务
 */
export interface BatchTask {
  id: number;
  /** 任务类型 */
  taskType: BatchTaskType;
  /** 输入文件地址 */
  fileUrl?: string;
  /** 总处理条数 */
  totalCount: number;
  /** 成功条数 */
  successCount: number;
  /** 失败条数 */
  failureCount: number;
  /** 任务状态 */
  status: BatchTaskStatus;
  /** 处理进度（0-100） */
  progress: number;
  /** 结果文件地址 */
  resultFileUrl?: string;
  /** 错误消息 */
  errorMessage?: string;
  /** 创建人 ID */
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 操作日志
 *
 * 记录 B 端所有运营操作
 */
export interface OperationLog {
  id: number;
  /** 操作人 ID */
  operatorId: string;
  /** 操作人名称 */
  operatorName?: string;
  /** 操作模块 */
  module: OperationModule;
  /** 操作动作 */
  action: string;
  /** 操作目标类型 */
  targetType?: string;
  /** 操作目标 ID */
  targetId?: string;
  /** 变更前数据 */
  beforeData?: Record<string, unknown>;
  /** 变更后数据 */
  afterData?: Record<string, unknown>;
  /** 操作者 IP */
  ipAddress?: string;
  /** User-Agent */
  userAgent?: string;
  createdAt: string;
}

/**
 * 操作模块
 */
export type OperationModule = 'category' | 'series' | 'badge' | 'rule' | 'grant' | 'revoke';

// ============ 表单数据类型 ============

/**
 * 手动发放请求
 */
export interface ManualGrantRequest {
  /** 用户 ID 列表 */
  userIds: string[];
  /** 徽章 ID */
  badgeId: number;
  /** 发放数量 */
  quantity?: number;
  /** 发放原因 */
  reason?: string;
}

/**
 * 批量发放请求
 */
export interface BatchGrantRequest {
  /** 文件 URL */
  fileUrl: string;
  /** 徽章 ID */
  badgeId: number;
  /** 发放原因 */
  reason?: string;
}

/**
 * 撤回徽章请求
 */
export interface RevokeRequest {
  /** 用户 ID */
  userId: string;
  /** 徽章 ID */
  badgeId: number;
  /** 撤回数量 */
  quantity?: number;
  /** 撤回原因 */
  reason: string;
}

/**
 * 发放日志查询参数
 */
export interface GrantLogQueryParams {
  /** 用户 ID */
  userId?: string;
  /** 徽章 ID */
  badgeId?: number;
  /** 操作动作 */
  action?: LogAction;
  /** 来源类型 */
  sourceType?: SourceType;
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

/**
 * 批量任务查询参数
 */
export interface BatchTaskQueryParams {
  /** 任务类型 */
  taskType?: BatchTaskType;
  /** 任务状态 */
  status?: BatchTaskStatus;
  /** 创建人 */
  createdBy?: string;
}

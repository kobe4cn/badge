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
 * 用于审计追踪的操作日志，包含完整的用户和徽章信息
 */
export interface GrantLog {
  id: number;
  /** 关联的用户徽章 ID */
  userBadgeId: number;
  /** 用户 ID */
  userId: string;
  /** 用户名（冗余存储，便于展示） */
  userName?: string;
  /** 用户头像 */
  userAvatar?: string;
  /** 徽章 ID */
  badgeId: number;
  /** 徽章名称（冗余存储，便于展示） */
  badgeName?: string;
  /** 徽章图标 */
  badgeIcon?: string;
  /** 操作动作 */
  action: LogAction;
  /** 操作原因 */
  reason?: string;
  /** 操作人 ID */
  operatorId?: string;
  /** 操作人名称 */
  operatorName?: string;
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
 * 发放日志详情
 *
 * 包含关联实体的完整信息
 */
export interface GrantLogDetail extends GrantLog {
  /** 用户会员等级 */
  userMembershipLevel?: string;
  /** 徽章类型 */
  badgeType?: string;
  /** 关联的批量任务名称 */
  batchTaskName?: string;
  /** 关联的规则名称 */
  ruleName?: string;
}

/**
 * 批量任务类型
 */
export type BatchTaskType = 'batch_grant' | 'batch_revoke' | 'data_export';

/**
 * 批量任务状态
 */
export type BatchTaskStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';

/**
 * 批量任务
 *
 * 记录批量发放/取消等异步任务
 */
export interface BatchTask {
  id: number;
  /** 任务名称 */
  name?: string;
  /** 任务类型 */
  taskType: BatchTaskType;
  /** 关联的徽章 ID */
  badgeId?: number;
  /** 关联的徽章名称 */
  badgeName?: string;
  /** 每人发放数量 */
  quantity?: number;
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
  /** 完成时间 */
  completedAt?: string;
  /** 调度类型 */
  scheduleType?: ScheduleType;
  /** 计划执行时间 */
  scheduledAt?: string;
  /** Cron 表达式 */
  cronExpression?: string;
  /** 下次执行时间（周期任务） */
  nextRunAt?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 批量任务失败明细
 */
export interface BatchTaskFailure {
  /** 记录 ID */
  id: number;
  /** 任务 ID */
  taskId: number;
  /** 行号 */
  rowNumber: number;
  /** 用户 ID */
  userId?: string;
  /** 错误码 */
  errorCode: string;
  /** 错误信息 */
  errorMessage: string;
  /** 重试次数 */
  retryCount: number;
  /** 重试状态: PENDING, RETRYING, SUCCESS, EXHAUSTED */
  retryStatus: string;
  /** 上次重试时间 */
  lastRetryAt?: string;
  /** 创建时间 */
  createdAt: string;
}

/**
 * 重试结果
 */
export interface RetryResult {
  /** 任务 ID */
  taskId: number;
  /** 待重试数量 */
  pendingCount: number;
  /** 提示信息 */
  message: string;
}

/**
 * 任务调度类型
 *
 * immediate: 立即执行（默认）
 * once: 定时单次执行
 * recurring: 周期执行
 */
export type ScheduleType = 'immediate' | 'once' | 'recurring';

/**
 * 创建批量任务请求
 */
export interface CreateBatchTaskRequest {
  /** 任务名称 */
  name: string;
  /** 徽章 ID */
  badgeId: number;
  /** 每人发放数量 */
  quantity: number;
  /** 发放原因 */
  reason?: string;
  /** 用户 ID 列表（CSV 上传模式） */
  userIds?: string[];
  /** 用户筛选条件（条件筛选模式） */
  userFilter?: UserFilterCondition;
  /** 调度类型：立即执行/定时/周期 */
  scheduleType?: ScheduleType;
  /** 计划执行时间（scheduleType = 'once' 时使用） */
  scheduledAt?: string;
  /** Cron 表达式（scheduleType = 'recurring' 时使用） */
  cronExpression?: string;
}

/**
 * 用户筛选条件
 */
export interface UserFilterCondition {
  /** 会员等级列表 */
  membershipLevels?: string[];
  /** 注册时间起始 */
  registeredAfter?: string;
  /** 注册时间结束 */
  registeredBefore?: string;
  /** 最低消费金额 */
  minTotalSpent?: number;
  /** 最低订单数 */
  minOrderCount?: number;
}

/**
 * CSV 解析结果
 */
export interface CsvParseResult {
  /** 有效用户 ID 列表 */
  userIds: string[];
  /** 无效行号列表 */
  invalidRows: number[];
  /** 总行数 */
  totalRows: number;
}

/**
 * 用户筛选预览结果
 */
export interface UserFilterPreview {
  /** 符合条件的用户数量 */
  count: number;
  /** 预览用户列表（前 10 条） */
  users: Array<{
    userId: string;
    username: string;
  }>;
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
 * 发放对象类型
 *
 * OWNER: 账号注册人（默认）
 * USER: 实际使用人
 */
export type RecipientType = 'OWNER' | 'USER';

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
  /** 发放对象类型：OWNER-账号注册人（默认），USER-实际使用人 */
  recipientType?: RecipientType;
  /** 实际使用人 ID（当 recipientType = USER 时必填） */
  actualUserId?: string;
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
  /** 用户名（模糊搜索） */
  userName?: string;
  /** 徽章 ID */
  badgeId?: number;
  /** 徽章名称（模糊搜索） */
  badgeName?: string;
  /** 操作动作 */
  action?: LogAction;
  /** 来源类型 */
  sourceType?: SourceType;
  /** 操作人 ID */
  operatorId?: string;
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

/**
 * 监控告警相关类型定义
 *
 * 对应需求 §11（系统日志和告警）中的监控、告警管理功能
 */

/**
 * 服务健康状态
 */
export type ServiceHealthStatus = 'healthy' | 'degraded' | 'down' | 'unknown';

/**
 * 服务健康信息
 *
 * 展示各微服务的运行状态
 */
export interface ServiceHealth {
  /** 服务名称 */
  name: string;
  /** 服务标识 */
  serviceId: string;
  /** 健康状态 */
  status: ServiceHealthStatus;
  /** 延迟（ms） */
  latencyMs?: number;
  /** 最后检查时间 */
  lastCheckAt: string;
  /** 正常运行时间（秒） */
  uptimeSeconds?: number;
  /** 附加信息 */
  metadata?: Record<string, unknown>;
}

/**
 * 告警严重程度
 */
export type AlertSeverity = 'critical' | 'warning' | 'info';

/**
 * 告警状态
 */
export type AlertStatus = 'firing' | 'resolved' | 'silenced' | 'acknowledged';

/**
 * 告警规则
 *
 * 对应 Prometheus AlertManager 的告警规则配置
 */
export interface AlertRule {
  id: number;
  /** 规则名称 */
  name: string;
  /** 规则描述 */
  description?: string;
  /** 严重程度 */
  severity: AlertSeverity;
  /** 触发条件表达式（PromQL 或自定义表达式） */
  expression: string;
  /** 持续时间（秒），超过此时间才触发告警 */
  durationSeconds: number;
  /** 是否启用 */
  enabled: boolean;
  /** 通知渠道 */
  notifyChannels: string[];
  /** 创建时间 */
  createdAt: string;
  /** 更新时间 */
  updatedAt: string;
}

/**
 * 告警事件
 *
 * 记录一次告警的触发和处理历史
 */
export interface AlertEvent {
  id: number;
  /** 关联的告警规则 ID */
  ruleId: number;
  /** 规则名称（冗余） */
  ruleName: string;
  /** 严重程度 */
  severity: AlertSeverity;
  /** 告警状态 */
  status: AlertStatus;
  /** 告警详情 */
  message: string;
  /** 触发时的指标值 */
  value?: string;
  /** 触发时间 */
  firedAt: string;
  /** 恢复时间 */
  resolvedAt?: string;
  /** 确认人 */
  acknowledgedBy?: string;
  /** 确认时间 */
  acknowledgedAt?: string;
  /** 静默截止时间 */
  silencedUntil?: string;
}

/**
 * 告警事件查询参数
 */
export interface AlertEventQueryParams {
  /** 严重程度 */
  severity?: AlertSeverity;
  /** 状态 */
  status?: AlertStatus;
  /** 规则 ID */
  ruleId?: number;
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

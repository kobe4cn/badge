/**
 * 监控告警 API 服务
 *
 * 封装系统监控和告警管理的接口调用。
 * 当前部分接口为前端 Mock 实现（后端 §11 告警 API 尚未完成），
 * 后续接入后端 API 后只需替换实现即可。
 */

import { get, post, put } from './api';
import type {
  ServiceHealth,
  AlertRule,
  AlertEvent,
  AlertEventQueryParams,
} from '@/types/monitoring';
import type { PaginatedResponse } from '@/types';

/**
 * 获取各服务健康状态
 *
 * 后端未提供统一健康检查接口前，通过多个 endpoint 拼装
 */
export async function getServiceHealth(): Promise<ServiceHealth[]> {
  // 尝试调用后端健康检查端点，失败则标记为 down
  const services: ServiceHealth[] = [
    { name: 'Badge Admin API', serviceId: 'badge-admin', status: 'unknown', lastCheckAt: new Date().toISOString() },
    { name: 'Rule Engine (gRPC)', serviceId: 'rule-engine', status: 'unknown', lastCheckAt: new Date().toISOString() },
    { name: 'PostgreSQL', serviceId: 'postgres', status: 'unknown', lastCheckAt: new Date().toISOString() },
    { name: 'Redis', serviceId: 'redis', status: 'unknown', lastCheckAt: new Date().toISOString() },
    { name: 'Kafka', serviceId: 'kafka', status: 'unknown', lastCheckAt: new Date().toISOString() },
  ];

  try {
    // 通过 stats/overview 接口检测 admin API 和 DB 是否可用
    await get('/admin/stats/overview');
    services[0].status = 'healthy';
    services[2].status = 'healthy'; // DB 可达说明 Postgres 正常
  } catch {
    services[0].status = 'down';
    services[2].status = 'unknown';
  }

  return services;
}

/**
 * 获取告警规则列表
 *
 * TODO: 后端实现告警规则 CRUD 后替换此 Mock
 */
export async function listAlertRules(): Promise<AlertRule[]> {
  try {
    return await get<AlertRule[]>('/admin/alerts/rules');
  } catch {
    // 后端未实现，返回预置规则
    return getDefaultAlertRules();
  }
}

/**
 * 更新告警规则启用状态
 */
export async function toggleAlertRule(ruleId: number, enabled: boolean): Promise<void> {
  try {
    await put(`/admin/alerts/rules/${ruleId}`, { enabled });
  } catch {
    // 后端未实现，前端静默处理
    console.warn('告警规则 API 未就绪，变更仅在前端生效');
  }
}

/**
 * 获取告警事件列表
 *
 * TODO: 后端实现告警事件查询后替换此 Mock
 */
export async function listAlertEvents(
  params?: AlertEventQueryParams
): Promise<PaginatedResponse<AlertEvent>> {
  try {
    return await get<PaginatedResponse<AlertEvent>>('/admin/alerts/events', params as Record<string, unknown>);
  } catch {
    // 后端未实现，返回空列表
    return { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
  }
}

/**
 * 确认告警事件
 */
export async function acknowledgeAlert(eventId: number): Promise<void> {
  try {
    await post(`/admin/alerts/events/${eventId}/acknowledge`);
  } catch {
    console.warn('告警确认 API 未就绪');
  }
}

/**
 * 静默告警事件
 *
 * @param eventId - 告警事件 ID
 * @param durationMinutes - 静默持续时间（分钟）
 */
export async function silenceAlert(eventId: number, durationMinutes: number): Promise<void> {
  try {
    await post(`/admin/alerts/events/${eventId}/silence`, { durationMinutes });
  } catch {
    console.warn('告警静默 API 未就绪');
  }
}

/**
 * 预置告警规则
 *
 * 在后端告警系统就绪前，提供默认的监控规则参考
 */
function getDefaultAlertRules(): AlertRule[] {
  const now = new Date().toISOString();
  return [
    {
      id: 1,
      name: '发放失败率过高',
      description: '5 分钟内发放失败率超过 5%',
      severity: 'critical',
      expression: 'rate(badge_grant_failures_total[5m]) / rate(badge_grant_total[5m]) > 0.05',
      durationSeconds: 300,
      enabled: true,
      notifyChannels: ['email', 'wechat'],
      createdAt: now,
      updatedAt: now,
    },
    {
      id: 2,
      name: 'API 响应延迟过高',
      description: 'P99 延迟超过 2 秒持续 3 分钟',
      severity: 'warning',
      expression: 'histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 2',
      durationSeconds: 180,
      enabled: true,
      notifyChannels: ['email'],
      createdAt: now,
      updatedAt: now,
    },
    {
      id: 3,
      name: '数据库连接池耗尽',
      description: '可用连接数低于 5',
      severity: 'critical',
      expression: 'pg_pool_available_connections < 5',
      durationSeconds: 60,
      enabled: true,
      notifyChannels: ['email', 'wechat'],
      createdAt: now,
      updatedAt: now,
    },
    {
      id: 4,
      name: 'Kafka 消费延迟',
      description: '消费者 lag 超过 1000 条持续 5 分钟',
      severity: 'warning',
      expression: 'kafka_consumer_group_lag > 1000',
      durationSeconds: 300,
      enabled: true,
      notifyChannels: ['email'],
      createdAt: now,
      updatedAt: now,
    },
    {
      id: 5,
      name: '批量任务积压',
      description: 'pending 状态任务超过 10 个',
      severity: 'info',
      expression: 'badge_batch_tasks_pending > 10',
      durationSeconds: 600,
      enabled: false,
      notifyChannels: ['email'],
      createdAt: now,
      updatedAt: now,
    },
  ];
}

/**
 * 事件类型 API 服务
 *
 * 获取系统支持的事件类型列表，用于规则配置
 */

import { get } from './api';

/**
 * 事件类型实体
 */
export interface EventType {
  /** 事件类型编码 */
  code: string;
  /** 事件类型名称 */
  name: string;
  /** 事件描述 */
  description?: string;
  /** 是否启用 */
  enabled: boolean;
}

/**
 * 获取事件类型列表
 *
 * 返回所有启用的事件类型
 */
export function getEventTypes(): Promise<EventType[]> {
  return get<EventType[]>('/admin/event-types');
}

/**
 * 事件类型服务对象
 */
export const eventTypeService = {
  getList: getEventTypes,
};

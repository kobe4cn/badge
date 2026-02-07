/**
 * 事件类型 React Query Hooks
 *
 * 获取系统支持的事件类型列表
 */

import { useQuery } from '@tanstack/react-query';
import { getEventTypes } from '@/services/eventType';

/**
 * 缓存 key 常量
 */
export const EVENT_TYPE_QUERY_KEYS = {
  all: ['eventTypes'] as const,
  list: () => [...EVENT_TYPE_QUERY_KEYS.all, 'list'] as const,
};

/**
 * 查询事件类型列表
 *
 * @param enabled - 是否启用查询
 */
export function useEventTypes(enabled = true) {
  return useQuery({
    queryKey: EVENT_TYPE_QUERY_KEYS.list(),
    queryFn: getEventTypes,
    enabled,
    staleTime: 5 * 60 * 1000, // 事件类型变化较少，缓存 5 分钟
  });
}

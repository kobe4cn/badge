/**
 * 规则画布节点组件导出
 *
 * 统一导出所有节点类型和相关组件，
 * 供 React Flow 画布注册使用
 */

import ConditionNode from './ConditionNode';
import LogicNode from './LogicNode';
import BadgeNode from './BadgeNode';

export { default as NodeBase } from './NodeBase';
export { default as ConditionNode } from './ConditionNode';
export { default as LogicNode } from './LogicNode';
export { default as BadgeNode } from './BadgeNode';
export { default as ConditionNodeConfig } from './ConditionNodeConfig';
export { default as BadgeNodeConfig } from './BadgeNodeConfig';

/**
 * 节点类型映射
 *
 * 用于 React Flow 的 nodeTypes 属性
 */
export const nodeTypes = {
  condition: ConditionNode,
  logic: LogicNode,
  badge: BadgeNode,
} as const;

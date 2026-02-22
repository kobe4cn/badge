/**
 * 规则画布节点类型定义
 *
 * 定义 React Flow 画布中各类节点的数据结构，
 * 与 rule.ts 中的规则引擎类型相互配合使用
 */

import type { Node, Edge } from '@xyflow/react';

// ============ 节点类型枚举 ============

/**
 * 画布节点类型
 *
 * 与 rule.ts 中的 RuleNodeType 不同，这里只定义画布相关的节点
 */
export type CanvasNodeType = 'condition' | 'logic' | 'badge';

/**
 * 条件操作符
 *
 * 定义条件节点支持的比较操作，涵盖常见的比较、字符串匹配和集合操作
 */
export type ConditionOperator =
  | 'eq'           // 等于
  | 'neq'          // 不等于
  | 'gt'           // 大于
  | 'gte'          // 大于等于
  | 'lt'           // 小于
  | 'lte'          // 小于等于
  | 'contains'     // 包含
  | 'starts_with'  // 以...开头
  | 'ends_with'    // 以...结尾
  | 'in'           // 属于列表
  | 'not_in'       // 不属于列表
  | 'between'      // 区间范围
  | 'is_empty'     // 为空
  | 'is_not_empty'; // 不为空

/**
 * 操作符配置信息
 */
export interface OperatorConfig {
  label: string;
  /** 操作符对应的值输入类型 */
  valueType: 'single' | 'range' | 'list' | 'none';
}

/**
 * 操作符配置映射
 *
 * 用于 UI 展示和表单验证
 */
export const OPERATOR_CONFIG: Record<ConditionOperator, OperatorConfig> = {
  eq: { label: '等于', valueType: 'single' },
  neq: { label: '不等于', valueType: 'single' },
  gt: { label: '大于', valueType: 'single' },
  gte: { label: '大于等于', valueType: 'single' },
  lt: { label: '小于', valueType: 'single' },
  lte: { label: '小于等于', valueType: 'single' },
  contains: { label: '包含', valueType: 'single' },
  starts_with: { label: '以...开头', valueType: 'single' },
  ends_with: { label: '以...结尾', valueType: 'single' },
  in: { label: '属于', valueType: 'list' },
  not_in: { label: '不属于', valueType: 'list' },
  between: { label: '介于', valueType: 'range' },
  is_empty: { label: '为空', valueType: 'none' },
  is_not_empty: { label: '不为空', valueType: 'none' },
};

/**
 * 逻辑类型
 */
export type LogicType = 'AND' | 'OR';

// ============ 节点数据类型 ============

/**
 * 条件节点数据
 *
 * 存储单个条件的判断配置
 */
export interface ConditionNodeData extends Record<string, unknown> {
  /** 字段路径，如 event.type, user.level */
  field: string;
  /** 比较操作符 */
  operator: ConditionOperator;
  /** 比较值 */
  value: string | number | boolean | string[];
  /** 字段的显示名称，用于 UI 友好展示 */
  fieldLabel?: string;
}

/**
 * 逻辑组节点数据
 *
 * 定义多个条件之间的逻辑关系
 */
export interface LogicNodeData extends Record<string, unknown> {
  logicType: LogicType;
}

/**
 * 徽章节点数据
 *
 * 作为规则的终点节点，配置徽章发放信息
 */
export interface BadgeNodeData extends Record<string, unknown> {
  /** 关联的徽章 ID */
  badgeId: string;
  /** 徽章名称，用于展示 */
  badgeName: string;
  /** 发放数量 */
  quantity: number;
}

// ============ React Flow 节点类型 ============

/**
 * 条件节点
 */
export type ConditionNode = Node<ConditionNodeData, 'condition'>;

/**
 * 逻辑节点
 */
export type LogicNode = Node<LogicNodeData, 'logic'>;

/**
 * 徽章节点
 */
export type BadgeNode = Node<BadgeNodeData, 'badge'>;

/**
 * 规则画布节点联合类型
 */
export type RuleCanvasNode = ConditionNode | LogicNode | BadgeNode;

/**
 * 规则画布连线
 */
export type RuleCanvasEdge = Edge;

// ============ 字段定义 ============

/**
 * 字段类型
 */
export type FieldType = 'string' | 'number' | 'boolean' | 'date' | 'array';

/**
 * 可选字段配置
 *
 * 用于条件节点的字段选择下拉框
 */
export interface FieldConfig {
  /** 字段路径 */
  field: string;
  /** 显示名称 */
  label: string;
  /** 字段类型 */
  type: FieldType;
  /** 所属分类 */
  category: 'event' | 'user' | 'order' | 'time';
}

/**
 * 预置字段列表
 *
 * 提供常用的条件字段配置
 */
export const PRESET_FIELDS: FieldConfig[] = [
  // 事件属性
  { field: 'event.type', label: '事件类型', type: 'string', category: 'event' },
  { field: 'event.name', label: '事件名称', type: 'string', category: 'event' },
  { field: 'event.timestamp', label: '事件时间', type: 'date', category: 'event' },

  // 用户属性
  { field: 'user.level', label: '用户等级', type: 'number', category: 'user' },
  { field: 'user.points', label: '用户积分', type: 'number', category: 'user' },
  { field: 'user.registerDays', label: '注册天数', type: 'number', category: 'user' },
  { field: 'user.tags', label: '用户标签', type: 'array', category: 'user' },

  // 订单属性
  { field: 'order.amount', label: '订单金额', type: 'number', category: 'order' },
  { field: 'order.count', label: '订单数量', type: 'number', category: 'order' },
  { field: 'order.status', label: '订单状态', type: 'string', category: 'order' },

  // 时间条件
  { field: 'time.hour', label: '当前小时', type: 'number', category: 'time' },
  { field: 'time.dayOfWeek', label: '星期几', type: 'number', category: 'time' },
  { field: 'time.dayOfMonth', label: '月份日期', type: 'number', category: 'time' },
];

/**
 * 规则序列化工具
 *
 * 负责画布状态（节点和边）与规则 JSON 之间的相互转换，
 * 使规则可以保存到后端并从后端加载恢复
 */

import type { Node, Edge } from '@xyflow/react';
import type {
  ConditionNodeData,
  LogicNodeData,
  BadgeNodeData,
  ConditionOperator,
  LogicType,
} from '../../../types/rule-canvas';

// ============ 规则定义类型 ============

/**
 * 规则条件
 *
 * 递归结构，支持嵌套的逻辑组合
 */
export interface RuleCondition {
  type: 'condition' | 'logic';
  // 条件类型字段
  field?: string;
  operator?: ConditionOperator;
  value?: string | number | boolean | string[];
  // 逻辑类型字段
  logicType?: LogicType;
  children?: RuleCondition[];
}

/**
 * 规则动作
 *
 * 当规则条件满足时执行的操作
 */
export interface RuleAction {
  type: 'award_badge';
  badgeId: string;
  badgeName?: string;
  quantity: number;
}

/**
 * 规则定义
 *
 * 完整的规则配置，包含条件树和动作列表
 */
export interface RuleDefinition {
  id: string;
  name: string;
  description?: string;
  conditions: RuleCondition[];
  actions: RuleAction[];
  /** 画布布局信息，用于恢复节点位置 */
  layout?: {
    nodes: Array<{ id: string; position: { x: number; y: number } }>;
  };
}

// ============ 画布到规则转换 ============

/**
 * 构建节点的入边映射
 *
 * 用于快速查找连接到某个节点的所有源节点
 */
const buildIncomingEdgesMap = (edges: Edge[]): Map<string, Edge[]> => {
  const map = new Map<string, Edge[]>();
  for (const edge of edges) {
    const existing = map.get(edge.target) || [];
    existing.push(edge);
    map.set(edge.target, existing);
  }
  return map;
};

/**
 * 构建节点映射
 */
const buildNodeMap = (nodes: Node[]): Map<string, Node> => {
  return new Map(nodes.map((n) => [n.id, n]));
};

/**
 * 从节点构建条件树
 *
 * 递归遍历节点的输入连接，构建完整的条件表达式
 */
const buildConditionTree = (
  nodeId: string,
  nodeMap: Map<string, Node>,
  incomingMap: Map<string, Edge[]>,
  visited: Set<string>
): RuleCondition | null => {
  // 防止循环
  if (visited.has(nodeId)) return null;
  visited.add(nodeId);

  const node = nodeMap.get(nodeId);
  if (!node) return null;

  if (node.type === 'condition') {
    const data = node.data as ConditionNodeData;
    return {
      type: 'condition',
      field: data.field,
      operator: data.operator,
      value: data.value,
    };
  }

  if (node.type === 'logic') {
    const data = node.data as LogicNodeData;
    const incomingEdges = incomingMap.get(nodeId) || [];
    const children: RuleCondition[] = [];

    for (const edge of incomingEdges) {
      const childCondition = buildConditionTree(
        edge.source,
        nodeMap,
        incomingMap,
        new Set(visited)
      );
      if (childCondition) {
        children.push(childCondition);
      }
    }

    return {
      type: 'logic',
      logicType: data.logicType,
      children,
    };
  }

  return null;
};

/**
 * 从画布状态构建规则定义
 *
 * 分析节点和边的拓扑结构，生成可序列化的规则 JSON
 */
export function canvasToRule(
  nodes: Node[],
  edges: Edge[],
  ruleId: string = 'new-rule',
  ruleName: string = '新规则'
): RuleDefinition {
  const nodeMap = buildNodeMap(nodes);
  const incomingMap = buildIncomingEdgesMap(edges);

  // 找到所有徽章节点作为规则的终点
  const badgeNodes = nodes.filter((n) => n.type === 'badge');

  const conditions: RuleCondition[] = [];
  const actions: RuleAction[] = [];

  for (const badgeNode of badgeNodes) {
    const badgeData = badgeNode.data as BadgeNodeData;

    // 构建发放动作
    actions.push({
      type: 'award_badge',
      badgeId: badgeData.badgeId,
      badgeName: badgeData.badgeName,
      quantity: badgeData.quantity,
    });

    // 从徽章节点向上遍历构建条件树
    const incomingEdges = incomingMap.get(badgeNode.id) || [];
    for (const edge of incomingEdges) {
      const condition = buildConditionTree(
        edge.source,
        nodeMap,
        incomingMap,
        new Set()
      );
      if (condition) {
        conditions.push(condition);
      }
    }
  }

  // 保存布局信息以便恢复
  const layout = {
    nodes: nodes.map((n) => ({
      id: n.id,
      position: n.position,
    })),
  };

  return {
    id: ruleId,
    name: ruleName,
    conditions,
    actions,
    layout,
  };
}

// ============ 规则到画布转换 ============

/**
 * 节点 ID 生成器
 */
let idCounter = 0;
const generateId = (prefix: string): string => `${prefix}-${++idCounter}`;
const resetIdCounter = (): void => {
  idCounter = 0;
};

/**
 * 从条件树构建节点和边
 *
 * 递归展开条件树，生成画布节点和连接
 */
const conditionToNodes = (
  condition: RuleCondition,
  nodes: Node[],
  edges: Edge[],
  parentId?: string,
  handleId?: string,
  depth: number = 0
): string | null => {
  // 基于深度计算节点位置
  const baseX = 100 + depth * 250;
  const baseY = 100 + nodes.length * 80;

  if (condition.type === 'condition') {
    const nodeId = generateId('condition');
    nodes.push({
      id: nodeId,
      type: 'condition',
      position: { x: baseX, y: baseY },
      data: {
        field: condition.field || '',
        operator: condition.operator || 'eq',
        value: condition.value ?? '',
      } as ConditionNodeData,
    });

    // 连接到父节点
    if (parentId) {
      edges.push({
        id: `e-${nodeId}-${parentId}`,
        source: nodeId,
        target: parentId,
        targetHandle: handleId,
      });
    }

    return nodeId;
  }

  if (condition.type === 'logic' && condition.children) {
    const nodeId = generateId('logic');
    nodes.push({
      id: nodeId,
      type: 'logic',
      position: { x: baseX, y: baseY },
      data: {
        logicType: condition.logicType || 'AND',
      } as LogicNodeData,
    });

    // 连接到父节点
    if (parentId) {
      edges.push({
        id: `e-${nodeId}-${parentId}`,
        source: nodeId,
        target: parentId,
        targetHandle: handleId,
      });
    }

    // 递归处理子条件
    condition.children.forEach((child, index) => {
      const inputHandle = `input-${index + 1}`;
      conditionToNodes(child, nodes, edges, nodeId, inputHandle, depth + 1);
    });

    return nodeId;
  }

  return null;
};

/**
 * 从规则定义构建画布状态
 *
 * 将 JSON 规则转换回节点和边，恢复画布编辑状态
 */
export function ruleToCanvas(rule: RuleDefinition): { nodes: Node[]; edges: Edge[] } {
  resetIdCounter();
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // 首先创建徽章节点
  rule.actions.forEach((action, actionIndex) => {
    if (action.type === 'award_badge') {
      const badgeId = generateId('badge');
      const badgeX = 550;
      const badgeY = 100 + actionIndex * 150;

      nodes.push({
        id: badgeId,
        type: 'badge',
        position: { x: badgeX, y: badgeY },
        data: {
          badgeId: action.badgeId,
          badgeName: action.badgeName || '',
          quantity: action.quantity,
        } as BadgeNodeData,
      });

      // 为每个条件创建节点树并连接到徽章
      if (rule.conditions[actionIndex]) {
        conditionToNodes(rule.conditions[actionIndex], nodes, edges, badgeId, undefined, 0);
      }
    }
  });

  // 如果有保存的布局信息，恢复节点位置
  if (rule.layout?.nodes) {
    const layoutMap = new Map(rule.layout.nodes.map((l) => [l.id, l.position]));
    // 优先使用原始 ID 匹配，否则按顺序匹配
    nodes.forEach((node, index) => {
      const savedPosition = layoutMap.get(node.id) || rule.layout?.nodes[index]?.position;
      if (savedPosition) {
        node.position = savedPosition;
      }
    });
  }

  return { nodes, edges };
}

/**
 * 验证规则定义是否有效
 */
export function validateRule(rule: RuleDefinition): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (!rule.id) {
    errors.push('规则 ID 不能为空');
  }

  if (!rule.name?.trim()) {
    errors.push('规则名称不能为空');
  }

  if (!rule.actions || rule.actions.length === 0) {
    errors.push('规则必须至少有一个动作');
  }

  for (const action of rule.actions || []) {
    if (!action.badgeId) {
      errors.push('徽章动作必须指定徽章 ID');
    }
    if (action.quantity < 1) {
      errors.push('发放数量必须大于 0');
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * 序列化规则为 JSON 字符串
 */
export function serializeRule(rule: RuleDefinition): string {
  return JSON.stringify(rule, null, 2);
}

/**
 * 从 JSON 字符串反序列化规则
 */
export function deserializeRule(json: string): RuleDefinition | null {
  try {
    return JSON.parse(json) as RuleDefinition;
  } catch {
    return null;
  }
}

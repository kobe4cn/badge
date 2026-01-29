/**
 * 连接验证工具
 *
 * 负责验证规则画布中节点连接的有效性，
 * 确保连线符合规则引擎的业务逻辑约束
 */

import type { Connection, Node, Edge } from '@xyflow/react';

/**
 * 节点类型定义
 */
type NodeType = 'condition' | 'logic' | 'badge';

/**
 * 获取节点类型
 */
const getNodeType = (nodeId: string, nodes: Node[]): NodeType | null => {
  const node = nodes.find((n) => n.id === nodeId);
  return node?.type as NodeType | null;
};

/**
 * 检查是否存在循环连接
 *
 * 使用 DFS 遍历从目标节点出发能否回到源节点，
 * 若能回到则说明新连接会形成循环
 */
const hasCycle = (
  sourceId: string,
  targetId: string,
  edges: Edge[],
  visited: Set<string> = new Set()
): boolean => {
  // 如果目标节点能到达源节点，则存在循环
  if (targetId === sourceId) return true;
  if (visited.has(targetId)) return false;

  visited.add(targetId);

  // 查找所有从 targetId 出发的边
  const outgoingEdges = edges.filter((e) => e.source === targetId);
  for (const edge of outgoingEdges) {
    if (hasCycle(sourceId, edge.target, edges, visited)) {
      return true;
    }
  }

  return false;
};

/**
 * 连接验证结果
 */
export interface ValidationResult {
  valid: boolean;
  reason?: string;
}

/**
 * 验证连接是否有效
 *
 * 验证规则：
 * 1. 不允许自连接
 * 2. 条件节点输出 -> 逻辑节点或徽章节点
 * 3. 逻辑节点输出 -> 逻辑节点或徽章节点
 * 4. 徽章节点是终点，没有输出
 * 5. 不允许形成循环
 */
export function isValidConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[]
): boolean {
  return validateConnection(connection, nodes, edges).valid;
}

/**
 * 验证连接并返回详细结果
 *
 * 提供验证失败的具体原因，便于用户理解
 */
export function validateConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[]
): ValidationResult {
  const { source, target } = connection;

  // 必须有源和目标
  if (!source || !target) {
    return { valid: false, reason: '连接不完整' };
  }

  // 规则1: 不能自连接
  if (source === target) {
    return { valid: false, reason: '不能连接自身' };
  }

  const sourceType = getNodeType(source, nodes);
  const targetType = getNodeType(target, nodes);

  if (!sourceType || !targetType) {
    return { valid: false, reason: '节点类型无效' };
  }

  // 规则4: 徽章节点不能作为源节点（终点节点）
  if (sourceType === 'badge') {
    return { valid: false, reason: '徽章节点是终点，不能有输出连接' };
  }

  // 规则2 & 3: 条件节点和逻辑节点只能连接到逻辑节点或徽章节点
  // 实际上这意味着：条件节点不能直接连接到另一个条件节点
  if (sourceType === 'condition' && targetType === 'condition') {
    return { valid: false, reason: '条件节点不能直接连接条件节点，请通过逻辑节点组合' };
  }

  // 规则5: 检测循环
  if (hasCycle(source, target, edges)) {
    return { valid: false, reason: '不能形成循环连接' };
  }

  return { valid: true };
}

/**
 * 获取节点可接受的连接来源类型
 */
export function getAcceptableSourceTypes(targetType: NodeType): NodeType[] {
  switch (targetType) {
    case 'logic':
      // 逻辑节点可以接受条件节点和其他逻辑节点的输入
      return ['condition', 'logic'];
    case 'badge':
      // 徽章节点可以接受条件节点和逻辑节点的输入
      return ['condition', 'logic'];
    case 'condition':
      // 条件节点一般不接受输入（除非作为子条件，但当前设计中条件节点是叶子节点）
      return [];
    default:
      return [];
  }
}

/**
 * 获取节点可输出的目标类型
 */
export function getAcceptableTargetTypes(sourceType: NodeType): NodeType[] {
  switch (sourceType) {
    case 'condition':
      // 条件节点可以连接到逻辑节点或徽章节点
      return ['logic', 'badge'];
    case 'logic':
      // 逻辑节点可以连接到其他逻辑节点或徽章节点
      return ['logic', 'badge'];
    case 'badge':
      // 徽章节点是终点，不能有输出
      return [];
    default:
      return [];
  }
}

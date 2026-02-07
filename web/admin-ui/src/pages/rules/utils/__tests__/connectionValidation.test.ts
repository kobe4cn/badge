import { describe, it, expect } from 'vitest';
import type { Node, Edge, Connection } from '@xyflow/react';
import {
  validateConnection,
  isValidConnection,
  getAcceptableSourceTypes,
  getAcceptableTargetTypes,
} from '../connectionValidation';

const makeNode = (id: string, type: string): Node => ({
  id,
  type,
  position: { x: 0, y: 0 },
  data: {},
});

const makeEdge = (source: string, target: string): Edge => ({
  id: `${source}-${target}`,
  source,
  target,
});

const nodes: Node[] = [
  makeNode('c1', 'condition'),
  makeNode('c2', 'condition'),
  makeNode('l1', 'logic'),
  makeNode('l2', 'logic'),
  makeNode('b1', 'badge'),
];

describe('validateConnection', () => {
  it('不完整的连接无效', () => {
    const conn: Connection = { source: null as unknown as string, target: 'c1', sourceHandle: null, targetHandle: null };
    const result = validateConnection(conn, nodes, []);
    expect(result.valid).toBe(false);
    expect(result.reason).toContain('不完整');
  });

  it('自连接无效', () => {
    const conn: Connection = { source: 'c1', target: 'c1', sourceHandle: null, targetHandle: null };
    const result = validateConnection(conn, nodes, []);
    expect(result.valid).toBe(false);
    expect(result.reason).toContain('自身');
  });

  it('徽章节点不能作为源', () => {
    const conn: Connection = { source: 'b1', target: 'l1', sourceHandle: null, targetHandle: null };
    const result = validateConnection(conn, nodes, []);
    expect(result.valid).toBe(false);
    expect(result.reason).toContain('终点');
  });

  it('条件节点不能直接连接条件节点', () => {
    const conn: Connection = { source: 'c1', target: 'c2', sourceHandle: null, targetHandle: null };
    const result = validateConnection(conn, nodes, []);
    expect(result.valid).toBe(false);
  });

  it('条件 -> 逻辑有效', () => {
    const conn: Connection = { source: 'c1', target: 'l1', sourceHandle: null, targetHandle: null };
    expect(isValidConnection(conn, nodes, [])).toBe(true);
  });

  it('条件 -> 徽章有效', () => {
    const conn: Connection = { source: 'c1', target: 'b1', sourceHandle: null, targetHandle: null };
    expect(isValidConnection(conn, nodes, [])).toBe(true);
  });

  it('逻辑 -> 逻辑有效', () => {
    const conn: Connection = { source: 'l1', target: 'l2', sourceHandle: null, targetHandle: null };
    expect(isValidConnection(conn, nodes, [])).toBe(true);
  });

  it('逻辑 -> 徽章有效', () => {
    const conn: Connection = { source: 'l1', target: 'b1', sourceHandle: null, targetHandle: null };
    expect(isValidConnection(conn, nodes, [])).toBe(true);
  });

  it('检测循环', () => {
    const edges = [makeEdge('l1', 'l2'), makeEdge('l2', 'l1')];
    const conn: Connection = { source: 'c1', target: 'l1', sourceHandle: null, targetHandle: null };
    // c1 -> l1 本身不形成循环
    expect(isValidConnection(conn, nodes, edges)).toBe(true);

    // l2 -> l1 已存在，l1 -> l2 也存在，尝试 c1 -> l2 -> l1 -> l2 循环
    const circularConn: Connection = { source: 'l2', target: 'l1', sourceHandle: null, targetHandle: null };
    const result = validateConnection(circularConn, nodes, [makeEdge('l1', 'l2')]);
    expect(result.valid).toBe(false);
    expect(result.reason).toContain('循环');
  });
});

describe('getAcceptableSourceTypes', () => {
  it('逻辑节点接受条件和逻辑输入', () => {
    expect(getAcceptableSourceTypes('logic')).toEqual(['condition', 'logic']);
  });

  it('徽章节点接受条件和逻辑输入', () => {
    expect(getAcceptableSourceTypes('badge')).toEqual(['condition', 'logic']);
  });

  it('条件节点不接受输入', () => {
    expect(getAcceptableSourceTypes('condition')).toEqual([]);
  });
});

describe('getAcceptableTargetTypes', () => {
  it('条件节点可输出到逻辑和徽章', () => {
    expect(getAcceptableTargetTypes('condition')).toEqual(['logic', 'badge']);
  });

  it('逻辑节点可输出到逻辑和徽章', () => {
    expect(getAcceptableTargetTypes('logic')).toEqual(['logic', 'badge']);
  });

  it('徽章节点不能输出', () => {
    expect(getAcceptableTargetTypes('badge')).toEqual([]);
  });
});

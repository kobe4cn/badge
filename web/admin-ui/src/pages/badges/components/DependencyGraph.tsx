/**
 * 依赖图可视化组件
 *
 * 使用 @xyflow/react 展示徽章之间的依赖关系，支持交互式查看
 */

import React, { useMemo, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  Node,
  Edge,
  MarkerType,
  Position,
  ConnectionLineType,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { Spin, Empty, Card } from 'antd';
import { useDependencyGraph } from '@/hooks/useDependency';
import type { DependencyGraphNode, DependencyGraphEdge } from '@/services/dependency';

interface DependencyGraphProps {
  /** 徽章 ID，如果提供则只显示相关子图 */
  badgeId?: string;
  /** 容器高度，默认 500px */
  height?: number;
}

/**
 * 节点类型对应的颜色
 */
const nodeColors: Record<string, string> = {
  root: '#1890ff',
  prerequisite: '#52c41a',
  dependent: '#faad14',
  badge: '#722ed1',
};

/**
 * 边类型对应的颜色
 */
const edgeColors: Record<string, string> = {
  prerequisite: '#1890ff',
  consume: '#fa541c',
  exclusive: '#eb2f96',
};

/**
 * 将后端数据转换为 ReactFlow 节点
 */
function convertToFlowNodes(nodes: DependencyGraphNode[]): Node[] {
  // 使用分层布局算法
  const nodeMap = new Map<string, DependencyGraphNode>();
  nodes.forEach((n) => nodeMap.set(n.id, n));

  // 简单的网格布局
  return nodes.map((node, index) => {
    // 根据节点类型确定列位置
    let xOffset = 0;
    if (node.nodeType === 'prerequisite') {
      xOffset = -300;
    } else if (node.nodeType === 'dependent') {
      xOffset = 300;
    }

    return {
      id: node.id,
      type: 'default',
      className: 'node',
      position: {
        x: 400 + xOffset + (index % 3) * 50,
        y: 100 + Math.floor(index / 3) * 120,
      },
      data: {
        label: (
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 'bold' }}>{node.label}</div>
            <div style={{ fontSize: 10, color: '#666' }}>
              {node.nodeType === 'root'
                ? '当前徽章'
                : node.nodeType === 'prerequisite'
                  ? '前置条件'
                  : node.nodeType === 'dependent'
                    ? '依赖此徽章'
                    : ''}
            </div>
          </div>
        ),
        nodeType: node.nodeType,
      },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      style: {
        background: nodeColors[node.nodeType] || '#1890ff',
        color: '#fff',
        border: '2px solid #fff',
        borderRadius: 8,
        padding: '10px 15px',
        minWidth: 120,
        boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
      },
    };
  });
}

/**
 * 将后端数据转换为 ReactFlow 边
 */
function convertToFlowEdges(edges: DependencyGraphEdge[]): Edge[] {
  return edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    type: 'smoothstep',
    className: 'edge',
    animated: edge.edgeType === 'consume',
    label: edge.label,
    labelStyle: { fontSize: 11, fill: '#666' },
    labelBgStyle: { fill: '#fff', fillOpacity: 0.9 },
    style: {
      stroke: edgeColors[edge.edgeType] || '#1890ff',
      strokeWidth: 2,
    },
    markerEnd: {
      type: MarkerType.ArrowClosed,
      color: edgeColors[edge.edgeType] || '#1890ff',
    },
  }));
}

const DependencyGraph: React.FC<DependencyGraphProps> = ({ badgeId, height = 500 }) => {
  const { data: graph, isLoading, error } = useDependencyGraph(badgeId, true);

  const nodes = useMemo(() => {
    if (!graph?.nodes) return [];
    return convertToFlowNodes(graph.nodes);
  }, [graph?.nodes]);

  const edges = useMemo(() => {
    if (!graph?.edges) return [];
    return convertToFlowEdges(graph.edges);
  }, [graph?.edges]);

  const onInit = useCallback(() => {
    // 初始化完成后的回调
  }, []);

  if (isLoading) {
    return (
      <div
        style={{
          height,
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <Spin tip="加载依赖图中..." />
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ height, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Empty description="加载失败，请重试" />
      </div>
    );
  }

  if (!nodes.length) {
    return (
      <div style={{ height, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Empty description="暂无依赖关系" />
      </div>
    );
  }

  return (
    <Card
      size="small"
      title="依赖关系图"
      extra={
        <div style={{ fontSize: 12, color: '#666' }}>
          <span style={{ marginRight: 16 }}>
            <span
              style={{
                display: 'inline-block',
                width: 12,
                height: 12,
                background: nodeColors.root,
                borderRadius: 2,
                marginRight: 4,
                verticalAlign: 'middle',
              }}
            />
            当前徽章
          </span>
          <span style={{ marginRight: 16 }}>
            <span
              style={{
                display: 'inline-block',
                width: 12,
                height: 12,
                background: nodeColors.prerequisite,
                borderRadius: 2,
                marginRight: 4,
                verticalAlign: 'middle',
              }}
            />
            前置条件
          </span>
          <span>
            <span
              style={{
                display: 'inline-block',
                width: 12,
                height: 12,
                background: nodeColors.dependent,
                borderRadius: 2,
                marginRight: 4,
                verticalAlign: 'middle',
              }}
            />
            依赖此徽章
          </span>
        </div>
      }
      className="dependency-graph"
    >
      <div style={{ height }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onInit={onInit}
          connectionLineType={ConnectionLineType.SmoothStep}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={2}
          defaultEdgeOptions={{
            type: 'smoothstep',
          }}
        >
          <Background color="#aaa" gap={16} />
          <Controls />
          <MiniMap
            nodeColor={(node) => {
              const nodeType =
                (node.data?.nodeType as string) ||
                (node.id.includes('root') ? 'root' : 'badge');
              return nodeColors[nodeType] || '#1890ff';
            }}
            style={{ height: 100 }}
          />
        </ReactFlow>
      </div>
    </Card>
  );
};

export default DependencyGraph;

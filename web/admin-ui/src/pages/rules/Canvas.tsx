/**
 * 规则画布页面
 *
 * 可视化规则编辑器，使用节点连线方式定义徽章发放规则。
 * 基于 @xyflow/react 实现拖拽式规则配置。
 */

import React, { useCallback } from 'react';
import { Card, Button, Space, Tooltip } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  PlusOutlined,
  PlayCircleOutlined,
  FilterOutlined,
  BranchesOutlined,
  TrophyOutlined,
} from '@ant-design/icons';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  ReactFlowProvider,
} from '@xyflow/react';
import type { Connection, Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { nodeTypes } from './components/nodes';
import type {
  ConditionNodeData,
  LogicNodeData,
  BadgeNodeData,
} from '../../types/rule-canvas';

/**
 * 初始节点配置
 *
 * 提供演示用的默认节点布局
 */
const initialNodes: Node[] = [
  {
    id: 'condition-1',
    type: 'condition',
    position: { x: 50, y: 50 },
    data: {
      field: 'user.level',
      operator: 'gte',
      value: 5,
      fieldLabel: '用户等级',
    } as ConditionNodeData,
  },
  {
    id: 'condition-2',
    type: 'condition',
    position: { x: 50, y: 180 },
    data: {
      field: 'order.count',
      operator: 'gte',
      value: 10,
      fieldLabel: '订单数量',
    } as ConditionNodeData,
  },
  {
    id: 'logic-1',
    type: 'logic',
    position: { x: 300, y: 100 },
    data: {
      logicType: 'AND',
    } as LogicNodeData,
  },
  {
    id: 'badge-1',
    type: 'badge',
    position: { x: 550, y: 100 },
    data: {
      badgeId: '1',
      badgeName: '忠实用户徽章',
      quantity: 1,
    } as BadgeNodeData,
  },
];

/**
 * 初始连接配置
 */
const initialEdges: Edge[] = [
  { id: 'e1-3', source: 'condition-1', target: 'logic-1', targetHandle: 'input-1' },
  { id: 'e2-3', source: 'condition-2', target: 'logic-1', targetHandle: 'input-2' },
  { id: 'e3-4', source: 'logic-1', target: 'badge-1' },
];

/**
 * 节点 ID 计数器
 */
let nodeIdCounter = 5;

/**
 * 生成唯一节点 ID
 */
const generateNodeId = (type: string): string => {
  return `${type}-${nodeIdCounter++}`;
};

/**
 * 规则画布内部组件
 *
 * 需要包裹在 ReactFlowProvider 中使用
 */
const CanvasInner: React.FC = () => {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  /**
   * 处理连接创建
   */
  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => addEdge(connection, eds));
    },
    [setEdges]
  );

  /**
   * 添加条件节点
   */
  const addConditionNode = useCallback(() => {
    const newNode: Node = {
      id: generateNodeId('condition'),
      type: 'condition',
      position: { x: 100 + Math.random() * 100, y: 100 + Math.random() * 100 },
      data: {
        field: '',
        operator: 'eq',
        value: '',
      } as ConditionNodeData,
    };
    setNodes((nds) => [...nds, newNode]);
  }, [setNodes]);

  /**
   * 添加逻辑节点
   */
  const addLogicNode = useCallback(() => {
    const newNode: Node = {
      id: generateNodeId('logic'),
      type: 'logic',
      position: { x: 250 + Math.random() * 100, y: 100 + Math.random() * 100 },
      data: {
        logicType: 'AND',
      } as LogicNodeData,
    };
    setNodes((nds) => [...nds, newNode]);
  }, [setNodes]);

  /**
   * 添加徽章节点
   */
  const addBadgeNode = useCallback(() => {
    const newNode: Node = {
      id: generateNodeId('badge'),
      type: 'badge',
      position: { x: 450 + Math.random() * 100, y: 100 + Math.random() * 100 },
      data: {
        badgeId: '',
        badgeName: '',
        quantity: 1,
      } as BadgeNodeData,
    };
    setNodes((nds) => [...nds, newNode]);
  }, [setNodes]);

  /**
   * MiniMap 节点颜色
   */
  const nodeColor = useCallback((node: Node) => {
    switch (node.type) {
      case 'condition':
        return '#1890ff';
      case 'logic':
        return '#722ed1';
      case 'badge':
        return '#faad14';
      default:
        return '#999';
    }
  }, []);

  return (
    <PageContainer
      title="规则画布"
      extra={
        <Space>
          <Tooltip title="添加条件节点">
            <Button icon={<FilterOutlined />} onClick={addConditionNode}>
              条件
            </Button>
          </Tooltip>
          <Tooltip title="添加逻辑组节点">
            <Button icon={<BranchesOutlined />} onClick={addLogicNode}>
              逻辑组
            </Button>
          </Tooltip>
          <Tooltip title="添加徽章节点">
            <Button icon={<TrophyOutlined />} onClick={addBadgeNode}>
              徽章
            </Button>
          </Tooltip>
          <Button icon={<PlayCircleOutlined />} disabled>
            测试规则
          </Button>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            保存规则
          </Button>
        </Space>
      }
    >
      <Card bodyStyle={{ padding: 0, height: 'calc(100vh - 260px)', minHeight: 500 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          nodeTypes={nodeTypes}
          fitView
          attributionPosition="bottom-left"
        >
          <Background color="#f0f0f0" gap={16} />
          <Controls />
          <MiniMap nodeColor={nodeColor} zoomable pannable />
        </ReactFlow>
      </Card>
    </PageContainer>
  );
};

/**
 * 规则画布页面
 *
 * 使用 ReactFlowProvider 包裹以支持 useReactFlow 钩子
 */
const CanvasPage: React.FC = () => {
  return (
    <ReactFlowProvider>
      <CanvasInner />
    </ReactFlowProvider>
  );
};

export default CanvasPage;

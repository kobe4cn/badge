/**
 * 规则画布页面
 *
 * 可视化规则编辑器，使用节点连线方式定义徽章发放规则。
 * 支持节点拖拽、连接验证、撤销重做、快捷键操作等交互功能。
 */

import React, { useCallback, useRef, useEffect, useMemo } from 'react';
import { Card, message } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  useReactFlow,
  ReactFlowProvider,
} from '@xyflow/react';
import type { Connection, Node, Edge, NodeChange, EdgeChange, IsValidConnection } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { nodeTypes, CanvasToolbar, CustomEdge } from './components';
import { useCanvasHistory, useCanvasHotkeys } from './hooks';
import { isValidConnection, validateConnection, canvasToRule, serializeRule } from './utils';
import type {
  ConditionNodeData,
  LogicNodeData,
  BadgeNodeData,
} from '../../types/rule-canvas';

/**
 * 自定义边类型
 */
const edgeTypes = {
  custom: CustomEdge,
};

/**
 * 默认边配置
 */
const defaultEdgeOptions = {
  type: 'custom',
  animated: true,
};

/**
 * 初始节点配置
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
  {
    id: 'e1-3',
    source: 'condition-1',
    target: 'logic-1',
    targetHandle: 'input-1',
    type: 'custom',
  },
  {
    id: 'e2-3',
    source: 'condition-2',
    target: 'logic-1',
    targetHandle: 'input-2',
    type: 'custom',
  },
  {
    id: 'e3-4',
    source: 'logic-1',
    target: 'badge-1',
    type: 'custom',
  },
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
 */
const CanvasInner: React.FC = () => {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const { zoomIn, zoomOut, fitView, getNodes, getEdges } = useReactFlow();

  // 历史记录管理
  const { canUndo, canRedo, undo, redo, pushHistory } = useCanvasHistory();

  // 记录初始状态
  const isInitialized = useRef(false);
  useEffect(() => {
    if (!isInitialized.current) {
      pushHistory(nodes, edges);
      isInitialized.current = true;
    }
  }, [nodes, edges, pushHistory]);

  /**
   * 获取选中的节点
   */
  const selectedNodes = useMemo(() => nodes.filter((n) => n.selected), [nodes]);
  const hasSelection = selectedNodes.length > 0;

  /**
   * 处理节点变更
   *
   * 在节点变更后记录历史，支持撤销
   */
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      onNodesChange(changes);

      // 仅在有实质性变更时记录历史（排除选中状态变化）
      const hasSubstantialChange = changes.some(
        (change) =>
          change.type === 'remove' ||
          change.type === 'add' ||
          (change.type === 'position' && change.dragging === false)
      );

      if (hasSubstantialChange) {
        // 延迟记录以获取最新状态
        setTimeout(() => {
          pushHistory(getNodes(), getEdges());
        }, 0);
      }
    },
    [onNodesChange, pushHistory, getNodes, getEdges]
  );

  /**
   * 处理边变更
   */
  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      onEdgesChange(changes);

      const hasSubstantialChange = changes.some(
        (change) => change.type === 'remove' || change.type === 'add'
      );

      if (hasSubstantialChange) {
        setTimeout(() => {
          pushHistory(getNodes(), getEdges());
        }, 0);
      }
    },
    [onEdgesChange, pushHistory, getNodes, getEdges]
  );

  /**
   * 处理连接创建
   *
   * 验证连接有效性后添加边
   */
  const onConnect = useCallback(
    (connection: Connection) => {
      const validation = validateConnection(connection, nodes, edges);

      if (!validation.valid) {
        message.warning(validation.reason || '连接无效');
        return;
      }

      setEdges((eds) => addEdge({ ...connection, type: 'custom' }, eds));

      // 记录历史
      setTimeout(() => {
        pushHistory(getNodes(), getEdges());
      }, 0);
    },
    [nodes, edges, setEdges, pushHistory, getNodes, getEdges]
  );

  /**
   * 连接有效性验证（用于拖拽时的视觉反馈）
   */
  const isValidConnectionCallback: IsValidConnection = useCallback(
    (connection) => {
      // 将 Edge | Connection 转换为标准 Connection 格式
      const conn: Connection = {
        source: connection.source,
        target: connection.target,
        sourceHandle: connection.sourceHandle ?? null,
        targetHandle: connection.targetHandle ?? null,
      };
      return isValidConnection(conn, nodes, edges);
    },
    [nodes, edges]
  );

  // ============ 节点操作 ============

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
    setTimeout(() => pushHistory(getNodes(), getEdges()), 0);
  }, [setNodes, pushHistory, getNodes, getEdges]);

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
    setTimeout(() => pushHistory(getNodes(), getEdges()), 0);
  }, [setNodes, pushHistory, getNodes, getEdges]);

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
    setTimeout(() => pushHistory(getNodes(), getEdges()), 0);
  }, [setNodes, pushHistory, getNodes, getEdges]);

  /**
   * 删除选中的节点和边
   */
  const deleteSelected = useCallback(() => {
    const selectedNodeIds = new Set(nodes.filter((n) => n.selected).map((n) => n.id));
    const selectedEdgeIds = new Set(edges.filter((e) => e.selected).map((e) => e.id));

    if (selectedNodeIds.size === 0 && selectedEdgeIds.size === 0) {
      message.info('请先选中要删除的节点或连线');
      return;
    }

    // 删除选中的节点及其关联的边
    setNodes((nds) => nds.filter((n) => !selectedNodeIds.has(n.id)));
    setEdges((eds) =>
      eds.filter(
        (e) =>
          !selectedEdgeIds.has(e.id) &&
          !selectedNodeIds.has(e.source) &&
          !selectedNodeIds.has(e.target)
      )
    );

    message.success('已删除选中的元素');
    setTimeout(() => pushHistory(getNodes(), getEdges()), 0);
  }, [nodes, edges, setNodes, setEdges, pushHistory, getNodes, getEdges]);

  // ============ 历史操作 ============

  /**
   * 撤销操作
   */
  const handleUndo = useCallback(() => {
    const state = undo();
    if (state) {
      setNodes(state.nodes);
      setEdges(state.edges);
      message.info('已撤销');
    }
  }, [undo, setNodes, setEdges]);

  /**
   * 重做操作
   */
  const handleRedo = useCallback(() => {
    const state = redo();
    if (state) {
      setNodes(state.nodes);
      setEdges(state.edges);
      message.info('已重做');
    }
  }, [redo, setNodes, setEdges]);

  // ============ 保存操作 ============

  /**
   * 保存规则
   */
  const handleSave = useCallback(() => {
    const currentNodes = getNodes();
    const currentEdges = getEdges();

    // 检查是否有徽章节点
    const badgeNodes = currentNodes.filter((n) => n.type === 'badge');
    if (badgeNodes.length === 0) {
      message.warning('规则必须至少包含一个徽章节点');
      return;
    }

    // 检查徽章节点是否有输入连接
    const hasInput = badgeNodes.some((badge) =>
      currentEdges.some((e) => e.target === badge.id)
    );
    if (!hasInput) {
      message.warning('徽章节点必须有条件连接');
      return;
    }

    // 转换为规则 JSON
    const rule = canvasToRule(currentNodes, currentEdges, 'rule-1', '自定义规则');
    const json = serializeRule(rule);

    // 这里可以调用 API 保存到后端
    console.log('规则 JSON:', json);
    message.success('规则保存成功');
  }, [getNodes, getEdges]);

  // ============ 键盘快捷键 ============

  useCanvasHotkeys({
    onDelete: deleteSelected,
    onUndo: handleUndo,
    onRedo: handleRedo,
    onSave: handleSave,
  });

  // ============ MiniMap 配置 ============

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
    <PageContainer title="规则画布">
      <Card bodyStyle={{ padding: 0, height: 'calc(100vh - 220px)', minHeight: 500 }}>
        <div style={{ position: 'relative', width: '100%', height: '100%' }}>
          {/* 工具栏 */}
          <CanvasToolbar
            canUndo={canUndo}
            canRedo={canRedo}
            hasSelection={hasSelection}
            onUndo={handleUndo}
            onRedo={handleRedo}
            onDelete={deleteSelected}
            onAddCondition={addConditionNode}
            onAddLogic={addLogicNode}
            onAddBadge={addBadgeNode}
            onZoomIn={() => zoomIn()}
            onZoomOut={() => zoomOut()}
            onFitView={() => fitView()}
            onSave={handleSave}
          />

          {/* 画布主体 */}
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={handleNodesChange}
            onEdgesChange={handleEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnectionCallback}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            defaultEdgeOptions={defaultEdgeOptions}
            fitView
            attributionPosition="bottom-left"
            deleteKeyCode={null} // 禁用默认删除键，使用自定义处理
            multiSelectionKeyCode="Shift"
            selectionKeyCode="Shift"
            panOnScroll
            zoomOnScroll
            selectNodesOnDrag={false}
          >
            <Background color="#f0f0f0" gap={16} />
            <Controls position="bottom-right" />
            <MiniMap
              nodeColor={nodeColor}
              zoomable
              pannable
              position="bottom-left"
              style={{ marginBottom: 10, marginLeft: 10 }}
            />
          </ReactFlow>
        </div>
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

/**
 * 规则画布页面
 *
 * 可视化规则编辑器，使用节点连线方式定义徽章发放规则。
 * 支持节点拖拽、连接验证、撤销重做、快捷键操作、规则测试与预览等交互功能。
 */

import React, { useCallback, useRef, useEffect, useMemo, useState } from 'react';
import { Card, message, App } from 'antd';
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

import {
  nodeTypes,
  CanvasToolbar,
  CustomEdge,
  TestPanel,
  TestResult,
  RuleInfoPanel,
} from './components';
import type { RuleInfo } from './components';
import { useCanvasHistory, useCanvasHotkeys } from './hooks';
import { isValidConnection, validateConnection, canvasToRule, serializeRule, validateRule } from './utils';
import type {
  ConditionNodeData,
  LogicNodeData,
  BadgeNodeData,
} from '../../types/rule-canvas';
import { useTestRuleDefinition, useCreateRule, useUpdateRule, usePublishRule } from '@/hooks';
import type { TestContext, RuleTestResult } from '@/services/rule';

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
  const { message: antMessage } = App.useApp();
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const { zoomIn, zoomOut, fitView, getNodes, getEdges } = useReactFlow();

  // 规则信息状态
  const [ruleInfo, setRuleInfo] = useState<RuleInfo>({
    name: '示例规则',
    description: '当用户等级>=5且订单数>=10时，发放忠实用户徽章',
    status: 'DRAFT',
    version: 1,
  });

  // 测试面板状态
  const [testPanelOpen, setTestPanelOpen] = useState(false);
  const [testResultOpen, setTestResultOpen] = useState(false);
  const [testResult, setTestResult] = useState<RuleTestResult | undefined>();

  // 匹配高亮的节点 ID
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<Set<string>>(new Set());

  // 变更追踪
  const [hasChanges, setHasChanges] = useState(false);
  const savedStateRef = useRef<string>('');

  // API Hooks
  const testRuleMutation = useTestRuleDefinition();
  const createRuleMutation = useCreateRule();
  const updateRuleMutation = useUpdateRule();
  const publishRuleMutation = usePublishRule();

  // 历史记录管理
  const { canUndo, canRedo, undo, redo, pushHistory } = useCanvasHistory();

  // 记录初始状态
  const isInitialized = useRef(false);
  useEffect(() => {
    if (!isInitialized.current) {
      pushHistory(nodes, edges);
      // 保存初始状态用于变更检测
      savedStateRef.current = JSON.stringify({ nodes, edges });
      isInitialized.current = true;
    }
  }, [nodes, edges, pushHistory]);

  /**
   * 检测是否有未保存的变更
   */
  useEffect(() => {
    const currentState = JSON.stringify({ nodes, edges });
    setHasChanges(currentState !== savedStateRef.current);
  }, [nodes, edges]);

  /**
   * 获取选中的节点
   */
  const selectedNodes = useMemo(() => nodes.filter((n) => n.selected), [nodes]);
  const hasSelection = selectedNodes.length > 0;

  /**
   * 验证规则
   */
  const ruleValidation = useMemo(() => {
    const currentNodes = nodes;
    const currentEdges = edges;
    const rule = canvasToRule(currentNodes, currentEdges, ruleInfo.id || 'new', ruleInfo.name);
    return validateRule(rule);
  }, [nodes, edges, ruleInfo.id, ruleInfo.name]);

  /**
   * 根据测试结果更新节点样式
   */
  const styledNodes = useMemo(() => {
    if (highlightedNodeIds.size === 0) return nodes;

    return nodes.map((node) => {
      const isHighlighted = highlightedNodeIds.has(node.id);
      return {
        ...node,
        style: {
          ...node.style,
          // 匹配的节点显示绿色边框
          border: isHighlighted ? '2px solid #52c41a' : undefined,
          boxShadow: isHighlighted ? '0 0 10px rgba(82, 196, 26, 0.5)' : undefined,
          // 未匹配的节点灰显
          opacity: highlightedNodeIds.size > 0 && !isHighlighted ? 0.5 : 1,
        },
      };
    });
  }, [nodes, highlightedNodeIds]);

  /**
   * 根据测试结果更新边样式
   */
  const styledEdges = useMemo(() => {
    if (highlightedNodeIds.size === 0) return edges;

    return edges.map((edge) => {
      const isSourceHighlighted = highlightedNodeIds.has(edge.source);
      const isTargetHighlighted = highlightedNodeIds.has(edge.target);
      const isHighlighted = isSourceHighlighted && isTargetHighlighted;

      return {
        ...edge,
        style: {
          ...edge.style,
          stroke: isHighlighted ? '#52c41a' : undefined,
          strokeWidth: isHighlighted ? 2 : 1,
          opacity: highlightedNodeIds.size > 0 && !isHighlighted ? 0.3 : 1,
        },
        animated: isHighlighted,
      };
    });
  }, [edges, highlightedNodeIds]);

  /**
   * 处理节点变更
   */
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      onNodesChange(changes);

      const hasSubstantialChange = changes.some(
        (change) =>
          change.type === 'remove' ||
          change.type === 'add' ||
          (change.type === 'position' && change.dragging === false)
      );

      if (hasSubstantialChange) {
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
   */
  const onConnect = useCallback(
    (connection: Connection) => {
      const validation = validateConnection(connection, nodes, edges);

      if (!validation.valid) {
        message.warning(validation.reason || '连接无效');
        return;
      }

      setEdges((eds) => addEdge({ ...connection, type: 'custom' }, eds));

      setTimeout(() => {
        pushHistory(getNodes(), getEdges());
      }, 0);
    },
    [nodes, edges, setEdges, pushHistory, getNodes, getEdges]
  );

  /**
   * 连接有效性验证
   */
  const isValidConnectionCallback: IsValidConnection = useCallback(
    (connection) => {
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

  const deleteSelected = useCallback(() => {
    const selectedNodeIds = new Set(nodes.filter((n) => n.selected).map((n) => n.id));
    const selectedEdgeIds = new Set(edges.filter((e) => e.selected).map((e) => e.id));

    if (selectedNodeIds.size === 0 && selectedEdgeIds.size === 0) {
      message.info('请先选中要删除的节点或连线');
      return;
    }

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

  const handleUndo = useCallback(() => {
    const state = undo();
    if (state) {
      setNodes(state.nodes);
      setEdges(state.edges);
      message.info('已撤销');
    }
  }, [undo, setNodes, setEdges]);

  const handleRedo = useCallback(() => {
    const state = redo();
    if (state) {
      setNodes(state.nodes);
      setEdges(state.edges);
      message.info('已重做');
    }
  }, [redo, setNodes, setEdges]);

  // ============ 规则保存 ============

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
    const rule = canvasToRule(currentNodes, currentEdges, ruleInfo.id || 'new', ruleInfo.name);
    const ruleJson = JSON.parse(serializeRule(rule));

    // 根据是否已有 ID 决定创建还是更新
    if (ruleInfo.id) {
      updateRuleMutation.mutate(
        {
          id: ruleInfo.id,
          data: {
            name: ruleInfo.name,
            description: ruleInfo.description,
            ruleJson,
          },
        },
        {
          onSuccess: () => {
            savedStateRef.current = JSON.stringify({ nodes: currentNodes, edges: currentEdges });
            setHasChanges(false);
          },
        }
      );
    } else {
      createRuleMutation.mutate(
        {
          name: ruleInfo.name,
          description: ruleInfo.description,
          ruleJson,
        },
        {
          onSuccess: (newRule) => {
            setRuleInfo((prev) => ({ ...prev, id: newRule.id }));
            savedStateRef.current = JSON.stringify({ nodes: currentNodes, edges: currentEdges });
            setHasChanges(false);
          },
        }
      );
    }
  }, [getNodes, getEdges, ruleInfo, createRuleMutation, updateRuleMutation]);

  // ============ 规则发布 ============

  const handlePublish = useCallback(() => {
    if (!ruleInfo.id) {
      antMessage.warning('请先保存规则');
      return;
    }
    publishRuleMutation.mutate(ruleInfo.id, {
      onSuccess: () => {
        setRuleInfo((prev) => ({ ...prev, status: 'PUBLISHED' }));
      },
    });
  }, [ruleInfo.id, publishRuleMutation, antMessage]);

  // ============ 规则测试 ============

  const handleOpenTest = useCallback(() => {
    setTestPanelOpen(true);
    // 清除之前的高亮
    setHighlightedNodeIds(new Set());
  }, []);

  const handleRunTest = useCallback(
    (context: TestContext) => {
      const currentNodes = getNodes();
      const currentEdges = getEdges();
      const rule = canvasToRule(currentNodes, currentEdges, ruleInfo.id || 'test', ruleInfo.name);
      const ruleJson = JSON.parse(serializeRule(rule));

      testRuleMutation.mutate(
        { ruleJson, context },
        {
          onSuccess: (result) => {
            setTestResult(result);
            setTestPanelOpen(false);
            setTestResultOpen(true);

            // 设置匹配节点高亮
            if (result.matchedNodeIds && result.matchedNodeIds.length > 0) {
              setHighlightedNodeIds(new Set(result.matchedNodeIds));
            } else {
              // 如果后端没有返回匹配节点，模拟高亮逻辑
              const matchedIds = new Set<string>();
              result.conditionResults?.forEach((cr) => {
                if (cr.matched) {
                  matchedIds.add(cr.nodeId);
                }
              });
              // 如果整体匹配，高亮所有节点
              if (result.matched) {
                currentNodes.forEach((n) => matchedIds.add(n.id));
              }
              setHighlightedNodeIds(matchedIds);
            }

            antMessage.success(result.matched ? '规则匹配成功!' : '规则未匹配');
          },
        }
      );
    },
    [getNodes, getEdges, ruleInfo, testRuleMutation, antMessage]
  );

  const handleCloseTestResult = useCallback(() => {
    setTestResultOpen(false);
    // 清除高亮
    setHighlightedNodeIds(new Set());
  }, []);

  // ============ 键盘快捷键 ============

  useCanvasHotkeys({
    onDelete: deleteSelected,
    onUndo: handleUndo,
    onRedo: handleRedo,
    onSave: handleSave,
  });

  // ============ MiniMap 配置 ============

  const nodeColor = useCallback((node: Node) => {
    if (highlightedNodeIds.has(node.id)) {
      return '#52c41a';
    }
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
  }, [highlightedNodeIds]);

  return (
    <PageContainer title="规则画布">
      {/* 响应式高度：桌面端使用 calc，移动端使用 minHeight */}
      <Card bodyStyle={{ padding: 0, height: 'calc(100vh - 220px)', minHeight: 400 }}>
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
            onTest={handleOpenTest}
            saving={createRuleMutation.isPending || updateRuleMutation.isPending}
          />

          {/* 规则信息面板 */}
          <RuleInfoPanel
            ruleInfo={ruleInfo}
            onRuleInfoChange={(info) => setRuleInfo((prev) => ({ ...prev, ...info }))}
            onSave={handleSave}
            onPublish={handlePublish}
            saving={createRuleMutation.isPending || updateRuleMutation.isPending}
            publishing={publishRuleMutation.isPending}
            isValid={ruleValidation.valid}
            validationErrors={ruleValidation.errors}
            hasChanges={hasChanges}
          />

          {/* 画布主体 */}
          <ReactFlow
            nodes={styledNodes}
            edges={styledEdges}
            onNodesChange={handleNodesChange}
            onEdgesChange={handleEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnectionCallback}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            defaultEdgeOptions={defaultEdgeOptions}
            fitView
            attributionPosition="bottom-left"
            deleteKeyCode={null}
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

      {/* 测试数据输入面板 */}
      <TestPanel
        open={testPanelOpen}
        onClose={() => setTestPanelOpen(false)}
        onRunTest={handleRunTest}
        loading={testRuleMutation.isPending}
      />

      {/* 测试结果展示 */}
      <TestResult
        open={testResultOpen}
        onClose={handleCloseTestResult}
        result={testResult}
        loading={testRuleMutation.isPending}
      />
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

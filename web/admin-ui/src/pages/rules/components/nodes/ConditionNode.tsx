/**
 * 条件节点组件
 *
 * 用于定义规则判断条件，包含字段、操作符和比较值的配置。
 * 支持多种操作符类型，如等于、大于、包含等。
 */

import React, { useState, useCallback, memo } from 'react';
import { Handle, Position, useReactFlow } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import { FilterOutlined } from '@ant-design/icons';
import NodeBase from './NodeBase';
import ConditionNodeConfig from './ConditionNodeConfig';
import type { ConditionNodeData, ConditionOperator } from '../../../../types/rule-canvas';
import { OPERATOR_CONFIG } from '../../../../types/rule-canvas';

/**
 * Handle 通用样式
 */
const handleStyle: React.CSSProperties = {
  width: 10,
  height: 10,
  background: '#1890ff',
  border: '2px solid white',
};

/**
 * 格式化显示值
 *
 * 将条件值转换为适合 UI 展示的字符串
 */
const formatValue = (value: string | number | boolean | string[]): string => {
  if (Array.isArray(value)) {
    return value.length > 2 ? `[${value.slice(0, 2).join(', ')}...]` : `[${value.join(', ')}]`;
  }
  if (typeof value === 'boolean') {
    return value ? '是' : '否';
  }
  const str = String(value);
  return str.length > 15 ? str.slice(0, 15) + '...' : str;
};

/**
 * 获取操作符显示文本
 */
const getOperatorLabel = (operator: ConditionOperator): string => {
  return OPERATOR_CONFIG[operator]?.label || operator;
};

type ConditionNodeProps = NodeProps<Node<ConditionNodeData>>;

const ConditionNode: React.FC<ConditionNodeProps> = ({ id, data, selected }) => {
  const [configVisible, setConfigVisible] = useState(false);
  const { setNodes } = useReactFlow();

  const handleClick = useCallback(() => {
    setConfigVisible(true);
  }, []);

  const handleConfigSave = useCallback(
    (newData: ConditionNodeData) => {
      setNodes((nodes) =>
        nodes.map((node) => (node.id === id ? { ...node, data: newData } : node))
      );
      setConfigVisible(false);
    },
    [id, setNodes]
  );

  const handleConfigCancel = useCallback(() => {
    setConfigVisible(false);
  }, []);

  // 类型断言以获取正确的数据类型
  const nodeData = data as ConditionNodeData;

  // 使用字段显示名称，若无则使用字段路径
  const fieldDisplay = nodeData.fieldLabel || nodeData.field || '未配置字段';
  const operatorDisplay = nodeData.operator ? getOperatorLabel(nodeData.operator) : '未配置';
  const valueDisplay = nodeData.value !== undefined ? formatValue(nodeData.value) : '未配置值';

  // 某些操作符（如 is_empty, is_not_empty）不需要显示值
  const showValue = nodeData.operator
    ? OPERATOR_CONFIG[nodeData.operator]?.valueType !== 'none'
    : true;

  return (
    <>
      {/* 输入连接点 */}
      <Handle type="target" position={Position.Left} style={handleStyle} />

      <NodeBase
        selected={selected}
        title="条件"
        icon={<FilterOutlined />}
        borderColor="#1890ff"
        onClick={handleClick}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ color: '#262626', fontWeight: 500 }}>{fieldDisplay}</div>
          <div style={{ color: '#8c8c8c' }}>
            {operatorDisplay}
            {showValue && (
              <span style={{ color: '#1890ff', marginLeft: 4 }}>{valueDisplay}</span>
            )}
          </div>
        </div>
      </NodeBase>

      {/* 输出连接点 */}
      <Handle type="source" position={Position.Right} style={handleStyle} />

      {/* 配置弹窗 */}
      <ConditionNodeConfig
        open={configVisible}
        data={nodeData}
        onSave={handleConfigSave}
        onCancel={handleConfigCancel}
      />
    </>
  );
};

export default memo(ConditionNode);

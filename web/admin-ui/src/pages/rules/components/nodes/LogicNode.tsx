/**
 * 逻辑组节点组件
 *
 * 用于组合多个条件节点的逻辑关系（AND/OR）。
 * 支持多个输入连接点和一个输出连接点。
 */

import React, { useCallback, memo } from 'react';
import { Handle, Position, useReactFlow } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import { BranchesOutlined } from '@ant-design/icons';
import NodeBase from './NodeBase';
import type { LogicNodeData, LogicType } from '../../../../types/rule-canvas';

/**
 * Handle 通用样式
 */
const handleStyle: React.CSSProperties = {
  width: 10,
  height: 10,
  background: '#722ed1',
  border: '2px solid white',
};

/**
 * 逻辑类型配置
 */
const LOGIC_CONFIG: Record<LogicType, { label: string; description: string }> = {
  AND: { label: 'AND', description: '所有条件都满足' },
  OR: { label: 'OR', description: '任一条件满足' },
};

type LogicNodeProps = NodeProps<Node<LogicNodeData>>;

const LogicNode: React.FC<LogicNodeProps> = ({ id, data, selected }) => {
  const { setNodes } = useReactFlow();

  // 类型断言以获取正确的数据类型
  const nodeData = data as LogicNodeData;

  /**
   * 切换逻辑类型
   *
   * 点击时在 AND 和 OR 之间切换
   */
  const handleToggle = useCallback(() => {
    setNodes((nodes) =>
      nodes.map((node) => {
        if (node.id === id) {
          const currentData = node.data as LogicNodeData;
          const newType: LogicType = currentData.logicType === 'AND' ? 'OR' : 'AND';
          return { ...node, data: { ...currentData, logicType: newType } };
        }
        return node;
      })
    );
  }, [id, setNodes]);

  const config = LOGIC_CONFIG[nodeData.logicType] || LOGIC_CONFIG.AND;

  return (
    <>
      {/* 多个输入连接点，垂直分布 */}
      <Handle
        type="target"
        position={Position.Left}
        id="input-1"
        style={{ ...handleStyle, top: '30%' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="input-2"
        style={{ ...handleStyle, top: '50%' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="input-3"
        style={{ ...handleStyle, top: '70%' }}
      />

      <NodeBase
        selected={selected}
        title="逻辑组"
        icon={<BranchesOutlined />}
        borderColor="#722ed1"
        onClick={handleToggle}
      >
        <div style={{ textAlign: 'center' }}>
          <div
            style={{
              fontSize: 20,
              fontWeight: 700,
              color: '#722ed1',
              marginBottom: 4,
              cursor: 'pointer',
              userSelect: 'none',
            }}
          >
            {config.label}
          </div>
          <div style={{ color: '#8c8c8c', fontSize: 11 }}>{config.description}</div>
          <div style={{ color: '#bfbfbf', fontSize: 10, marginTop: 4 }}>
            点击切换
          </div>
        </div>
      </NodeBase>

      {/* 输出连接点 */}
      <Handle type="source" position={Position.Right} style={handleStyle} />
    </>
  );
};

export default memo(LogicNode);

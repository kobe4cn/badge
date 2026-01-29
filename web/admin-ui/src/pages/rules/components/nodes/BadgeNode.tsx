/**
 * 徽章节点组件
 *
 * 作为规则画布的终点节点，配置当条件满足时发放的徽章信息。
 * 只有输入连接点，表示规则的执行动作。
 */

import React, { useState, useCallback, memo } from 'react';
import { Handle, Position, useReactFlow } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import { TrophyOutlined } from '@ant-design/icons';
import { Tag } from 'antd';
import NodeBase from './NodeBase';
import BadgeNodeConfig from './BadgeNodeConfig';
import type { BadgeNodeData } from '../../../../types/rule-canvas';

/**
 * Handle 样式
 */
const handleStyle: React.CSSProperties = {
  width: 10,
  height: 10,
  background: '#faad14',
  border: '2px solid white',
};

type BadgeNodeProps = NodeProps<Node<BadgeNodeData>>;

const BadgeNode: React.FC<BadgeNodeProps> = ({ id, data, selected }) => {
  const [configVisible, setConfigVisible] = useState(false);
  const { setNodes } = useReactFlow();

  // 类型断言以获取正确的数据类型
  const nodeData = data as BadgeNodeData;

  const handleClick = useCallback(() => {
    setConfigVisible(true);
  }, []);

  const handleConfigSave = useCallback(
    (newData: BadgeNodeData) => {
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

  const badgeName = nodeData.badgeName || '未选择徽章';
  const quantity = nodeData.quantity || 1;

  return (
    <>
      {/* 输入连接点（终点节点只有输入） */}
      <Handle type="target" position={Position.Left} style={handleStyle} />

      <NodeBase
        selected={selected}
        title="发放徽章"
        icon={<TrophyOutlined />}
        borderColor="#faad14"
        onClick={handleClick}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
            }}
          >
            <TrophyOutlined style={{ fontSize: 24, color: '#faad14' }} />
            <span style={{ color: '#262626', fontWeight: 500 }}>{badgeName}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            <span style={{ color: '#8c8c8c' }}>发放数量:</span>
            <Tag color="gold">{quantity}</Tag>
          </div>
        </div>
      </NodeBase>

      {/* 配置弹窗 */}
      <BadgeNodeConfig
        open={configVisible}
        data={nodeData}
        onSave={handleConfigSave}
        onCancel={handleConfigCancel}
      />
    </>
  );
};

export default memo(BadgeNode);

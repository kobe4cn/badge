/**
 * 节点面板组件
 *
 * 提供可拖拽的节点列表，用于向画布添加新节点
 */

import React from 'react';
import { Typography } from 'antd';
import {
  FilterOutlined,
  BranchesOutlined,
  TrophyOutlined,
} from '@ant-design/icons';

const { Text } = Typography;

/**
 * 节点类型配置
 */
const NODE_TYPES = [
  {
    type: 'condition',
    label: '条件节点',
    icon: <FilterOutlined style={{ color: '#1890ff', fontSize: 20 }} />,
    description: '定义触发条件',
    color: '#e6f7ff',
    borderColor: '#1890ff',
  },
  {
    type: 'combiner',
    label: '逻辑组节点',
    icon: <BranchesOutlined style={{ color: '#722ed1', fontSize: 20 }} />,
    description: 'AND/OR 逻辑',
    color: '#f9f0ff',
    borderColor: '#722ed1',
  },
  {
    type: 'action',
    label: '动作节点',
    icon: <TrophyOutlined style={{ color: '#faad14', fontSize: 20 }} />,
    description: '发放徽章',
    color: '#fffbe6',
    borderColor: '#faad14',
  },
];

/**
 * 节点面板属性
 */
export interface NodePanelProps {
  /** 添加条件节点回调 */
  onAddCondition: () => void;
  /** 添加逻辑节点回调 */
  onAddLogic: () => void;
  /** 添加徽章节点回调 */
  onAddBadge: () => void;
}

/**
 * 节点面板样式
 */
const panelStyle: React.CSSProperties = {
  position: 'absolute',
  top: 70,
  left: 10,
  width: 160,
  zIndex: 10,
  background: 'white',
  borderRadius: 8,
  boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
  padding: 12,
};

const nodeItemStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  padding: '10px 12px',
  marginBottom: 8,
  borderRadius: 6,
  cursor: 'grab',
  transition: 'all 0.2s',
};

const NodePanel: React.FC<NodePanelProps> = ({
  onAddCondition,
  onAddLogic,
  onAddBadge,
}) => {
  /**
   * 处理拖拽开始
   */
  const handleDragStart = (
    event: React.DragEvent,
    nodeType: string
  ) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  /**
   * 处理点击添加节点
   */
  const handleClick = (nodeType: string) => {
    switch (nodeType) {
      case 'condition':
        onAddCondition();
        break;
      case 'combiner':
        onAddLogic();
        break;
      case 'action':
        onAddBadge();
        break;
    }
  };

  return (
    <div className="node-panel" style={panelStyle}>
      <Text strong style={{ display: 'block', marginBottom: 12, color: '#666' }}>
        节点类型
      </Text>
      {NODE_TYPES.map((node) => (
        <div
          key={node.type}
          data-nodetype={node.type}
          draggable
          onDragStart={(e) => handleDragStart(e, node.type)}
          onClick={() => handleClick(node.type)}
          style={{
            ...nodeItemStyle,
            background: node.color,
            border: `1px solid ${node.borderColor}`,
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.transform = 'scale(1.02)';
            e.currentTarget.style.boxShadow = '0 2px 8px rgba(0, 0, 0, 0.15)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.transform = 'scale(1)';
            e.currentTarget.style.boxShadow = 'none';
          }}
        >
          {node.icon}
          <div>
            <div style={{ fontWeight: 500, fontSize: 13 }}>{node.label}</div>
            <div style={{ fontSize: 11, color: '#888' }}>{node.description}</div>
          </div>
        </div>
      ))}
    </div>
  );
};

export default NodePanel;

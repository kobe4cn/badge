/**
 * 节点基础包装器组件
 *
 * 提供规则画布中所有节点的通用样式和结构，
 * 统一处理选中状态、标题栏和内容区域的渲染
 */

import React from 'react';
import type { ReactNode } from 'react';

export interface NodeBaseProps {
  /** 节点是否被选中 */
  selected?: boolean;
  /** 节点标题 */
  title: string;
  /** 标题栏图标 */
  icon: ReactNode;
  /** 边框颜色（不包含 # 前缀），用于区分不同类型节点 */
  borderColor: string;
  /** 子节点内容 */
  children: ReactNode;
  /** 点击事件 */
  onClick?: () => void;
}

/**
 * 节点基础样式
 */
const baseStyle: React.CSSProperties = {
  minWidth: 180,
  borderRadius: 8,
  background: 'white',
  boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
  overflow: 'hidden',
  cursor: 'pointer',
  transition: 'box-shadow 0.2s, transform 0.2s',
};

/**
 * 选中状态样式
 */
const selectedStyle: React.CSSProperties = {
  boxShadow: '0 4px 16px rgba(0, 0, 0, 0.2)',
  transform: 'scale(1.02)',
};

/**
 * 标题栏样式
 */
const headerStyle: React.CSSProperties = {
  padding: '8px 12px',
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  fontSize: 13,
  fontWeight: 500,
  borderBottom: '1px solid #f0f0f0',
};

/**
 * 内容区域样式
 */
const contentStyle: React.CSSProperties = {
  padding: '10px 12px',
  fontSize: 12,
};

const NodeBase: React.FC<NodeBaseProps> = ({
  selected,
  title,
  icon,
  borderColor,
  children,
  onClick,
}) => {
  const containerStyle: React.CSSProperties = {
    ...baseStyle,
    border: `2px solid ${borderColor}`,
    ...(selected ? selectedStyle : {}),
  };

  const titleBarStyle: React.CSSProperties = {
    ...headerStyle,
    color: borderColor,
  };

  return (
    <div style={containerStyle} onClick={onClick}>
      <div style={titleBarStyle}>
        {icon}
        <span>{title}</span>
      </div>
      <div style={contentStyle}>{children}</div>
    </div>
  );
};

export default NodeBase;

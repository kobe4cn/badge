/**
 * 自定义边组件
 *
 * 在标准边的基础上添加动画效果和删除按钮，
 * 提供更好的交互体验
 */

import React, { useState, useCallback } from 'react';
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  useReactFlow,
} from '@xyflow/react';
import type { EdgeProps, Edge } from '@xyflow/react';
import { CloseCircleFilled } from '@ant-design/icons';

/**
 * 边类型颜色配置
 *
 * 根据源节点类型设置不同的边颜色
 */
const EDGE_COLORS: Record<string, string> = {
  condition: '#1890ff',
  logic: '#722ed1',
  badge: '#faad14',
  default: '#b1b1b7',
};

/**
 * 自定义边组件
 *
 * 特性：
 * - 流动动画效果
 * - 悬停时显示删除按钮
 * - 根据源节点类型变换颜色
 */
const CustomEdge: React.FC<EdgeProps<Edge>> = ({
  id,
  source,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  markerEnd,
  selected,
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const { setEdges, getNode } = useReactFlow();

  // 获取源节点类型以确定边的颜色
  const sourceNode = getNode(source);
  const edgeColor = EDGE_COLORS[sourceNode?.type || 'default'] || EDGE_COLORS.default;

  // 计算贝塞尔曲线路径
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  /**
   * 删除边
   */
  const onEdgeDelete = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      setEdges((edges) => edges.filter((edge) => edge.id !== id));
    },
    [id, setEdges]
  );

  return (
    <>
      {/* 边的主体 */}
      <BaseEdge
        path={edgePath}
        markerEnd={markerEnd}
        style={{
          ...style,
          strokeWidth: selected ? 3 : 2,
          stroke: edgeColor,
        }}
      />

      {/* 动画层 - 流动效果 */}
      <path
        d={edgePath}
        fill="none"
        stroke={edgeColor}
        strokeWidth={selected ? 3 : 2}
        strokeDasharray="5 5"
        style={{
          animation: 'edge-flow 1s linear infinite',
          opacity: 0.5,
        }}
      />

      {/* 交互层 - 更宽的可点击区域 */}
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={20}
        style={{ cursor: 'pointer' }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      />

      {/* 删除按钮 */}
      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            pointerEvents: 'all',
            opacity: isHovered || selected ? 1 : 0,
            transition: 'opacity 0.2s',
          }}
          onMouseEnter={() => setIsHovered(true)}
          onMouseLeave={() => setIsHovered(false)}
        >
          <CloseCircleFilled
            style={{
              fontSize: 18,
              color: '#ff4d4f',
              background: 'white',
              borderRadius: '50%',
              cursor: 'pointer',
            }}
            onClick={onEdgeDelete}
          />
        </div>
      </EdgeLabelRenderer>

      {/* 注入动画 CSS */}
      <style>
        {`
          @keyframes edge-flow {
            from {
              stroke-dashoffset: 10;
            }
            to {
              stroke-dashoffset: 0;
            }
          }
        `}
      </style>
    </>
  );
};

export default CustomEdge;

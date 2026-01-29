/**
 * 画布工具栏组件
 *
 * 提供规则画布的操作按钮，包括添加节点、撤销/重做、缩放控制和保存功能
 */

import React, { useMemo } from 'react';
import { Button, Dropdown, Space, Tooltip, Divider, message } from 'antd';
import type { MenuProps } from 'antd';
import {
  PlusOutlined,
  UndoOutlined,
  RedoOutlined,
  DeleteOutlined,
  ZoomInOutlined,
  ZoomOutOutlined,
  ExpandOutlined,
  SaveOutlined,
  PlayCircleOutlined,
  FilterOutlined,
  BranchesOutlined,
  TrophyOutlined,
} from '@ant-design/icons';

/**
 * 工具栏属性
 */
export interface CanvasToolbarProps {
  /** 是否可以撤销 */
  canUndo: boolean;
  /** 是否可以重做 */
  canRedo: boolean;
  /** 是否有选中的节点 */
  hasSelection: boolean;
  /** 撤销回调 */
  onUndo: () => void;
  /** 重做回调 */
  onRedo: () => void;
  /** 删除选中节点 */
  onDelete: () => void;
  /** 添加条件节点 */
  onAddCondition: () => void;
  /** 添加逻辑节点 */
  onAddLogic: () => void;
  /** 添加徽章节点 */
  onAddBadge: () => void;
  /** 放大 */
  onZoomIn: () => void;
  /** 缩小 */
  onZoomOut: () => void;
  /** 适应画布 */
  onFitView: () => void;
  /** 保存规则 */
  onSave: () => void;
  /** 测试规则 */
  onTest?: () => void;
  /** 是否正在保存 */
  saving?: boolean;
}

/**
 * 工具栏样式
 */
const toolbarStyle: React.CSSProperties = {
  position: 'absolute',
  top: 10,
  left: 10,
  zIndex: 10,
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  padding: '8px 12px',
  background: 'white',
  borderRadius: 8,
  boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
};

const CanvasToolbar: React.FC<CanvasToolbarProps> = ({
  canUndo,
  canRedo,
  hasSelection,
  onUndo,
  onRedo,
  onDelete,
  onAddCondition,
  onAddLogic,
  onAddBadge,
  onZoomIn,
  onZoomOut,
  onFitView,
  onSave,
  onTest,
  saving = false,
}) => {
  /**
   * 添加节点下拉菜单配置
   */
  const addNodeMenuItems: MenuProps['items'] = useMemo(
    () => [
      {
        key: 'condition',
        icon: <FilterOutlined style={{ color: '#1890ff' }} />,
        label: '条件节点',
        onClick: () => {
          onAddCondition();
          message.info('已添加条件节点');
        },
      },
      {
        key: 'logic',
        icon: <BranchesOutlined style={{ color: '#722ed1' }} />,
        label: '逻辑组节点',
        onClick: () => {
          onAddLogic();
          message.info('已添加逻辑组节点');
        },
      },
      {
        key: 'badge',
        icon: <TrophyOutlined style={{ color: '#faad14' }} />,
        label: '徽章节点',
        onClick: () => {
          onAddBadge();
          message.info('已添加徽章节点');
        },
      },
    ],
    [onAddCondition, onAddLogic, onAddBadge]
  );

  return (
    <div style={toolbarStyle}>
      {/* 添加节点 */}
      <Dropdown menu={{ items: addNodeMenuItems }} placement="bottomLeft">
        <Button type="primary" icon={<PlusOutlined />}>
          添加节点
        </Button>
      </Dropdown>

      <Divider type="vertical" style={{ height: 24, margin: '0 4px' }} />

      {/* 撤销/重做 */}
      <Space.Compact>
        <Tooltip title="撤销 (Ctrl+Z)">
          <Button icon={<UndoOutlined />} disabled={!canUndo} onClick={onUndo} />
        </Tooltip>
        <Tooltip title="重做 (Ctrl+Y)">
          <Button icon={<RedoOutlined />} disabled={!canRedo} onClick={onRedo} />
        </Tooltip>
      </Space.Compact>

      {/* 删除选中 */}
      <Tooltip title="删除选中 (Delete)">
        <Button
          icon={<DeleteOutlined />}
          disabled={!hasSelection}
          onClick={onDelete}
          danger
        />
      </Tooltip>

      <Divider type="vertical" style={{ height: 24, margin: '0 4px' }} />

      {/* 缩放控制 */}
      <Space.Compact>
        <Tooltip title="放大">
          <Button icon={<ZoomInOutlined />} onClick={onZoomIn} />
        </Tooltip>
        <Tooltip title="缩小">
          <Button icon={<ZoomOutOutlined />} onClick={onZoomOut} />
        </Tooltip>
        <Tooltip title="适应画布">
          <Button icon={<ExpandOutlined />} onClick={onFitView} />
        </Tooltip>
      </Space.Compact>

      <Divider type="vertical" style={{ height: 24, margin: '0 4px' }} />

      {/* 测试与保存 */}
      {onTest && (
        <Tooltip title="测试规则">
          <Button icon={<PlayCircleOutlined />} onClick={onTest}>
            测试
          </Button>
        </Tooltip>
      )}
      <Tooltip title="保存规则 (Ctrl+S)">
        <Button
          type="primary"
          icon={<SaveOutlined />}
          loading={saving}
          onClick={onSave}
        >
          保存
        </Button>
      </Tooltip>
    </div>
  );
};

export default CanvasToolbar;

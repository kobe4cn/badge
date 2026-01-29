/**
 * 规则信息面板组件
 *
 * 在画布顶部显示规则的基本信息，支持编辑规则名称和描述，
 * 提供保存和发布操作入口
 */

import React, { useState, useEffect } from 'react';
import {
  Card,
  Input,
  Space,
  Button,
  Tooltip,
  Tag,
  Popconfirm,
  message,
} from 'antd';
import {
  SaveOutlined,
  CloudUploadOutlined,
  EditOutlined,
  CheckOutlined,
  CloseOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import type { RuleStatus } from '@/services/rule';

const { TextArea } = Input;

/**
 * 规则信息
 */
export interface RuleInfo {
  id?: string;
  name: string;
  description?: string;
  status?: RuleStatus;
  version?: number;
}

/**
 * 规则信息面板属性
 */
export interface RuleInfoPanelProps {
  /** 规则信息 */
  ruleInfo: RuleInfo;
  /** 规则信息变更 */
  onRuleInfoChange: (info: Partial<RuleInfo>) => void;
  /** 保存回调 */
  onSave: () => void;
  /** 发布回调 */
  onPublish: () => void;
  /** 是否正在保存 */
  saving?: boolean;
  /** 是否正在发布 */
  publishing?: boolean;
  /** 规则是否有效（通过验证） */
  isValid?: boolean;
  /** 验证错误信息 */
  validationErrors?: string[];
  /** 是否有未保存的变更 */
  hasChanges?: boolean;
}

/**
 * 状态标签配置
 */
const statusConfig: Record<RuleStatus, { color: string; text: string }> = {
  DRAFT: { color: 'default', text: '草稿' },
  PUBLISHED: { color: 'success', text: '已发布' },
  DISABLED: { color: 'warning', text: '已禁用' },
  ARCHIVED: { color: 'default', text: '已归档' },
};

const RuleInfoPanel: React.FC<RuleInfoPanelProps> = ({
  ruleInfo,
  onRuleInfoChange,
  onSave,
  onPublish,
  saving = false,
  publishing = false,
  isValid = true,
  validationErrors = [],
  hasChanges = false,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(ruleInfo.name);
  const [editDescription, setEditDescription] = useState(ruleInfo.description || '');

  // 同步外部变更
  useEffect(() => {
    setEditName(ruleInfo.name);
    setEditDescription(ruleInfo.description || '');
  }, [ruleInfo.name, ruleInfo.description]);

  /**
   * 进入编辑模式
   */
  const handleStartEdit = () => {
    setIsEditing(true);
    setEditName(ruleInfo.name);
    setEditDescription(ruleInfo.description || '');
  };

  /**
   * 保存编辑
   */
  const handleSaveEdit = () => {
    if (!editName.trim()) {
      message.warning('规则名称不能为空');
      return;
    }
    onRuleInfoChange({
      name: editName.trim(),
      description: editDescription.trim() || undefined,
    });
    setIsEditing(false);
  };

  /**
   * 取消编辑
   */
  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditName(ruleInfo.name);
    setEditDescription(ruleInfo.description || '');
  };

  /**
   * 发布前确认
   */
  const handlePublishConfirm = () => {
    if (!isValid) {
      message.error('规则验证不通过，请修复后再发布');
      return;
    }
    if (hasChanges) {
      message.warning('请先保存规则再发布');
      return;
    }
    onPublish();
  };

  const statusInfo = ruleInfo.status ? statusConfig[ruleInfo.status] : null;

  return (
    <Card
      size="small"
      style={{
        position: 'absolute',
        top: 60,
        right: 10,
        zIndex: 10,
        width: 320,
        boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
      }}
      bodyStyle={{ padding: 12 }}
    >
      {isEditing ? (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          <Input
            placeholder="规则名称"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            maxLength={50}
            showCount
          />
          <TextArea
            placeholder="规则描述（可选）"
            value={editDescription}
            onChange={(e) => setEditDescription(e.target.value)}
            rows={2}
            maxLength={200}
            showCount
          />
          <Space>
            <Button
              type="primary"
              size="small"
              icon={<CheckOutlined />}
              onClick={handleSaveEdit}
            >
              确定
            </Button>
            <Button size="small" icon={<CloseOutlined />} onClick={handleCancelEdit}>
              取消
            </Button>
          </Space>
        </Space>
      ) : (
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          {/* 标题行 */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontWeight: 600, fontSize: 14, flex: 1 }}>
              {ruleInfo.name || '未命名规则'}
            </span>
            {statusInfo && <Tag color={statusInfo.color}>{statusInfo.text}</Tag>}
            <Tooltip title="编辑信息">
              <Button
                type="text"
                size="small"
                icon={<EditOutlined />}
                onClick={handleStartEdit}
              />
            </Tooltip>
          </div>

          {/* 描述 */}
          {ruleInfo.description && (
            <span style={{ fontSize: 12, color: '#666' }}>{ruleInfo.description}</span>
          )}

          {/* 版本信息 */}
          {ruleInfo.version && (
            <span style={{ fontSize: 11, color: '#999' }}>版本: v{ruleInfo.version}</span>
          )}

          {/* 验证状态 */}
          {!isValid && validationErrors.length > 0 && (
            <div style={{ background: '#fff2e8', padding: 8, borderRadius: 4 }}>
              <Space direction="vertical" size={2}>
                <span style={{ color: '#fa541c', fontSize: 12 }}>
                  <ExclamationCircleOutlined /> 规则验证不通过:
                </span>
                {validationErrors.slice(0, 3).map((err, idx) => (
                  <span key={idx} style={{ fontSize: 11, color: '#fa541c' }}>
                    - {err}
                  </span>
                ))}
                {validationErrors.length > 3 && (
                  <span style={{ fontSize: 11, color: '#999' }}>
                    还有 {validationErrors.length - 3} 个错误...
                  </span>
                )}
              </Space>
            </div>
          )}

          {/* 操作按钮 */}
          <Space style={{ marginTop: 4 }}>
            <Tooltip title={hasChanges ? '有未保存的变更' : '保存规则'}>
              <Button
                type={hasChanges ? 'primary' : 'default'}
                size="small"
                icon={<SaveOutlined />}
                loading={saving}
                onClick={onSave}
              >
                {hasChanges ? '保存*' : '保存'}
              </Button>
            </Tooltip>
            <Popconfirm
              title="确认发布规则?"
              description="发布后规则将立即生效"
              onConfirm={handlePublishConfirm}
              okText="确认发布"
              cancelText="取消"
              disabled={!isValid || hasChanges || publishing}
            >
              <Tooltip
                title={
                  hasChanges
                    ? '请先保存规则'
                    : !isValid
                    ? '规则验证不通过'
                    : '发布规则'
                }
              >
                <Button
                  size="small"
                  icon={<CloudUploadOutlined />}
                  loading={publishing}
                  disabled={!isValid || hasChanges}
                >
                  发布
                </Button>
              </Tooltip>
            </Popconfirm>
          </Space>
        </Space>
      )}
    </Card>
  );
};

export default RuleInfoPanel;

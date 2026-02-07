/**
 * 规则信息面板组件
 *
 * 在画布顶部显示规则的基本信息，支持编辑规则名称和描述，
 * 提供保存和发布操作入口。包含规则必填元数据的收集。
 */

import React, { useState, useEffect, useMemo } from 'react';
import {
  Card,
  Input,
  Space,
  Button,
  Tooltip,
  Tag,
  Popconfirm,
  message,
  Select,
  Divider,
  DatePicker,
  InputNumber,
} from 'antd';
import {
  SaveOutlined,
  CloudUploadOutlined,
  EditOutlined,
  CheckOutlined,
  CloseOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import { useEventTypes, useBadgeList } from '@/hooks';
import dayjs from 'dayjs';

const { TextArea } = Input;

/**
 * 规则信息
 *
 * 包含规则的基本信息和必填元数据（用于后端存储）
 */
export interface RuleInfo {
  id?: number;
  /** 关联的徽章 ID（必填，新建时需要选择） */
  badgeId?: number;
  /** 关联的徽章名称（显示用） */
  badgeName?: string;
  /** 事件类型（必填，如 purchase, login） */
  eventType?: string;
  /** 规则编码（必填，唯一标识） */
  ruleCode?: string;
  /** 规则名称（必填，显示用） */
  name: string;
  /** 规则描述 */
  description?: string;
  /** 是否启用 */
  enabled?: boolean;
  /** 每用户最大获取次数 */
  maxCountPerUser?: number;
  /** 全局配额限制 */
  globalQuota?: number;
  /** 生效开始时间 */
  startTime?: string;
  /** 生效结束时间 */
  endTime?: string;
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
 * 生成默认规则编码
 */
const generateRuleCode = (badgeId?: number): string => {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 6);
  return `rule_${badgeId || 'new'}_${timestamp}_${random}`;
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

  // 编辑状态
  const [editName, setEditName] = useState(ruleInfo.name);
  const [editDescription, setEditDescription] = useState(ruleInfo.description || '');
  const [editBadgeId, setEditBadgeId] = useState(ruleInfo.badgeId);
  const [editEventType, setEditEventType] = useState(ruleInfo.eventType || '');
  const [editRuleCode, setEditRuleCode] = useState(ruleInfo.ruleCode || '');
  const [editStartTime, setEditStartTime] = useState(ruleInfo.startTime);
  const [editEndTime, setEditEndTime] = useState(ruleInfo.endTime);
  const [editMaxCountPerUser, setEditMaxCountPerUser] = useState(ruleInfo.maxCountPerUser);
  const [editGlobalQuota, setEditGlobalQuota] = useState(ruleInfo.globalQuota);

  // 获取徽章和事件类型列表
  const { data: badgesData } = useBadgeList({ page: 1, pageSize: 100 });
  const { data: eventTypesData } = useEventTypes();

  const badges = badgesData?.items || [];
  const eventTypes = eventTypesData || [];

  // 同步外部变更
  useEffect(() => {
    setEditName(ruleInfo.name);
    setEditDescription(ruleInfo.description || '');
    setEditBadgeId(ruleInfo.badgeId);
    setEditEventType(ruleInfo.eventType || '');
    setEditRuleCode(ruleInfo.ruleCode || '');
    setEditStartTime(ruleInfo.startTime);
    setEditEndTime(ruleInfo.endTime);
    setEditMaxCountPerUser(ruleInfo.maxCountPerUser);
    setEditGlobalQuota(ruleInfo.globalQuota);
  }, [ruleInfo]);

  // 检查必填元数据是否完整
  const metadataComplete = useMemo(() => {
    // 已存在的规则不需要再检查（已经有 ID）
    if (ruleInfo.id) return true;
    return !!(ruleInfo.badgeId && ruleInfo.eventType && ruleInfo.ruleCode && ruleInfo.name);
  }, [ruleInfo]);

  // 验证错误（包含元数据缺失）
  const allErrors = useMemo(() => {
    const errors = [...validationErrors];
    if (!ruleInfo.id) {
      if (!ruleInfo.badgeId) errors.push('请选择关联徽章');
      if (!ruleInfo.eventType) errors.push('请选择事件类型');
      if (!ruleInfo.ruleCode) errors.push('请输入规则编码');
    }
    return errors;
  }, [ruleInfo, validationErrors]);

  /**
   * 进入编辑模式
   */
  const handleStartEdit = () => {
    setIsEditing(true);
  };

  /**
   * 保存编辑
   */
  const handleSaveEdit = () => {
    if (!editName.trim()) {
      message.warning('规则名称不能为空');
      return;
    }
    if (!ruleInfo.id) {
      // 新建规则时需要验证必填字段
      if (!editBadgeId) {
        message.warning('请选择关联徽章');
        return;
      }
      if (!editEventType) {
        message.warning('请选择事件类型');
        return;
      }
      if (!editRuleCode) {
        message.warning('请输入规则编码');
        return;
      }
    }

    const selectedBadge = badges.find((b) => b.id === editBadgeId);
    onRuleInfoChange({
      name: editName.trim(),
      description: editDescription.trim() || undefined,
      badgeId: editBadgeId,
      badgeName: selectedBadge?.name,
      eventType: editEventType,
      ruleCode: editRuleCode,
      startTime: editStartTime,
      endTime: editEndTime,
      maxCountPerUser: editMaxCountPerUser,
      globalQuota: editGlobalQuota,
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
    setEditBadgeId(ruleInfo.badgeId);
    setEditEventType(ruleInfo.eventType || '');
    setEditRuleCode(ruleInfo.ruleCode || '');
    setEditStartTime(ruleInfo.startTime);
    setEditEndTime(ruleInfo.endTime);
    setEditMaxCountPerUser(ruleInfo.maxCountPerUser);
    setEditGlobalQuota(ruleInfo.globalQuota);
  };

  /**
   * 自动生成规则编码
   */
  const handleGenerateRuleCode = () => {
    const newCode = generateRuleCode(editBadgeId);
    setEditRuleCode(newCode);
  };

  /**
   * 发布前确认
   */
  const handlePublishConfirm = () => {
    if (!isValid || !metadataComplete) {
      message.error('规则验证不通过，请修复后再发布');
      return;
    }
    if (hasChanges) {
      message.warning('请先保存规则再发布');
      return;
    }
    onPublish();
  };

  // 状态标签
  const statusTag = ruleInfo.enabled !== undefined ? (
    <Tag color={ruleInfo.enabled ? 'success' : 'default'}>
      {ruleInfo.enabled ? '已启用' : '未启用'}
    </Tag>
  ) : null;

  return (
    <Card
      size="small"
      style={{
        position: 'absolute',
        top: 60,
        right: 10,
        zIndex: 10,
        width: 360,
        maxHeight: 'calc(100vh - 280px)',
        overflow: 'auto',
        boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
      }}
      bodyStyle={{ padding: 12 }}
    >
      {isEditing ? (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          {/* 基本信息 */}
          <Input
            placeholder="规则名称 *"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            maxLength={200}
          />
          <TextArea
            placeholder="规则描述（可选）"
            value={editDescription}
            onChange={(e) => setEditDescription(e.target.value)}
            rows={2}
            maxLength={500}
          />

          {/* 新建规则时显示必填元数据 */}
          {!ruleInfo.id && (
            <>
              <Divider style={{ margin: '8px 0' }}>
                <span style={{ fontSize: 12, color: '#666' }}>规则配置</span>
              </Divider>

              <Select
                placeholder="选择关联徽章 *"
                style={{ width: '100%' }}
                value={editBadgeId}
                onChange={setEditBadgeId}
                showSearch
                optionFilterProp="children"
                filterOption={(input, option) =>
                  (option?.children as unknown as string)?.toLowerCase().includes(input.toLowerCase())
                }
              >
                {badges.map((badge) => (
                  <Select.Option key={badge.id} value={badge.id}>
                    {badge.name}
                  </Select.Option>
                ))}
              </Select>

              <Select
                placeholder="选择事件类型 *"
                style={{ width: '100%' }}
                value={editEventType || undefined}
                onChange={setEditEventType}
                showSearch
                optionFilterProp="children"
              >
                {eventTypes.map((et) => (
                  <Select.Option key={et.code} value={et.code}>
                    {et.name} ({et.code})
                  </Select.Option>
                ))}
              </Select>

              <Space.Compact style={{ width: '100%' }}>
                <Input
                  placeholder="规则编码 *"
                  value={editRuleCode}
                  onChange={(e) => setEditRuleCode(e.target.value)}
                  maxLength={100}
                  style={{ flex: 1 }}
                />
                <Button onClick={handleGenerateRuleCode}>生成</Button>
              </Space.Compact>
            </>
          )}

          {/* 高级配置：时间范围和配额 */}
          <Divider style={{ margin: '8px 0' }}>
            <span style={{ fontSize: 12, color: '#666' }}>高级配置</span>
          </Divider>

          <div style={{ display: 'flex', gap: 8 }}>
            <DatePicker
              placeholder="生效开始时间"
              showTime
              style={{ flex: 1 }}
              value={editStartTime ? dayjs(editStartTime) : null}
              onChange={(date) => setEditStartTime(date?.toISOString())}
            />
            <DatePicker
              placeholder="生效结束时间"
              showTime
              style={{ flex: 1 }}
              value={editEndTime ? dayjs(editEndTime) : null}
              onChange={(date) => setEditEndTime(date?.toISOString())}
            />
          </div>

          <div style={{ display: 'flex', gap: 8 }}>
            <InputNumber
              placeholder="每用户上限"
              min={1}
              style={{ flex: 1 }}
              value={editMaxCountPerUser}
              onChange={(value) => setEditMaxCountPerUser(value ?? undefined)}
              addonBefore="每人"
              addonAfter="次"
            />
            <InputNumber
              placeholder="全局配额"
              min={1}
              style={{ flex: 1 }}
              value={editGlobalQuota}
              onChange={(value) => setEditGlobalQuota(value ?? undefined)}
              addonBefore="总计"
              addonAfter="份"
            />
          </div>

          <Space style={{ marginTop: 8 }}>
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
            {statusTag}
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

          {/* 规则元数据 */}
          <div style={{ fontSize: 11, color: '#999' }}>
            {ruleInfo.badgeName && <div>徽章: {ruleInfo.badgeName}</div>}
            {ruleInfo.eventType && <div>事件: {ruleInfo.eventType}</div>}
            {ruleInfo.ruleCode && <div>编码: {ruleInfo.ruleCode}</div>}
            {(ruleInfo.startTime || ruleInfo.endTime) && (
              <div>
                时间: {ruleInfo.startTime ? dayjs(ruleInfo.startTime).format('YYYY-MM-DD HH:mm') : '无限制'} ~{' '}
                {ruleInfo.endTime ? dayjs(ruleInfo.endTime).format('YYYY-MM-DD HH:mm') : '无限制'}
              </div>
            )}
            {ruleInfo.maxCountPerUser && <div>每用户上限: {ruleInfo.maxCountPerUser} 次</div>}
            {ruleInfo.globalQuota && <div>全局配额: {ruleInfo.globalQuota} 份</div>}
          </div>

          {/* 验证状态 */}
          {allErrors.length > 0 && (
            <div style={{ background: '#fff2e8', padding: 8, borderRadius: 4 }}>
              <Space direction="vertical" size={2}>
                <span style={{ color: '#fa541c', fontSize: 12 }}>
                  <ExclamationCircleOutlined /> 请完善规则配置:
                </span>
                {allErrors.slice(0, 3).map((err, idx) => (
                  <span key={idx} style={{ fontSize: 11, color: '#fa541c' }}>
                    - {err}
                  </span>
                ))}
                {allErrors.length > 3 && (
                  <span style={{ fontSize: 11, color: '#999' }}>
                    还有 {allErrors.length - 3} 个问题...
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
                disabled={!metadataComplete}
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
              disabled={!isValid || !metadataComplete || hasChanges || publishing}
            >
              <Tooltip
                title={
                  !metadataComplete
                    ? '请先完善规则配置'
                    : hasChanges
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
                  disabled={!isValid || !metadataComplete || hasChanges}
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

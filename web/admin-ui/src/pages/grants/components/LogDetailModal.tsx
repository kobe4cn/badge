/**
 * 发放日志详情弹窗
 *
 * 展示单条发放日志的完整信息，包括用户、徽章和操作详情
 */

import React from 'react';
import {
  Modal,
  Descriptions,
  Tag,
  Space,
  Typography,
  Spin,
  Empty,
  Card,
  Avatar,
  Divider,
} from 'antd';
import {
  UserOutlined,
  TrophyOutlined,
  ClockCircleOutlined,
  FileTextOutlined,
} from '@ant-design/icons';
import { useGrantLogDetail } from '@/hooks/useGrant';
import { formatDateTime } from '@/utils/format';
import type { SourceType, LogAction } from '@/types';

const { Text } = Typography;

interface LogDetailModalProps {
  open: boolean;
  logId: number | null;
  onClose: () => void;
}

/**
 * 获取来源类型标签配置
 */
const getSourceTypeConfig = (sourceType: SourceType) => {
  const configs: Record<SourceType, { color: string; text: string }> = {
    MANUAL: { color: 'blue', text: '手动发放' },
    EVENT: { color: 'purple', text: '事件触发' },
    SCHEDULED: { color: 'cyan', text: '定时任务' },
    REDEMPTION: { color: 'orange', text: '兑换' },
    SYSTEM: { color: 'default', text: '系统' },
  };
  return configs[sourceType] || { color: 'default', text: sourceType };
};

/**
 * 获取操作动作标签配置
 */
const getActionConfig = (action: LogAction) => {
  const configs: Record<LogAction, { color: string; text: string }> = {
    GRANT: { color: 'success', text: '发放' },
    REVOKE: { color: 'error', text: '撤回' },
    REDEEM: { color: 'warning', text: '兑换' },
    EXPIRE: { color: 'default', text: '过期' },
  };
  return configs[action] || { color: 'default', text: action };
};

/**
 * 发放日志详情弹窗组件
 */
const LogDetailModal: React.FC<LogDetailModalProps> = ({
  open,
  logId,
  onClose,
}) => {
  const {
    data: log,
    isLoading,
    isError,
  } = useGrantLogDetail(logId || 0, open && !!logId);

  // 渲染加载状态
  if (isLoading) {
    return (
      <Modal
        title="日志详情"
        open={open}
        onCancel={onClose}
        footer={null}
        width={640}
      >
        <div style={{ textAlign: 'center', padding: 48 }}>
          <Spin size="large" />
          <p style={{ marginTop: 16 }}>加载中...</p>
        </div>
      </Modal>
    );
  }

  // 渲染错误状态
  if (isError || !log) {
    return (
      <Modal
        title="日志详情"
        open={open}
        onCancel={onClose}
        footer={null}
        width={640}
      >
        <Empty description="日志不存在或加载失败" />
      </Modal>
    );
  }

  const actionConfig = getActionConfig(log.action);
  const sourceConfig = getSourceTypeConfig(log.sourceType);

  return (
    <Modal
      title={
        <Space>
          <FileTextOutlined />
          <span>日志详情</span>
          <Tag color={actionConfig.color}>{actionConfig.text}</Tag>
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={640}
    >
      {/* 基本信息 */}
      <Card size="small" style={{ marginBottom: 16 }}>
        <Descriptions column={2} size="small">
          <Descriptions.Item label="日志 ID">{log.id}</Descriptions.Item>
          <Descriptions.Item label="操作时间">
            <Space>
              <ClockCircleOutlined />
              {formatDateTime(log.createdAt)}
            </Space>
          </Descriptions.Item>
        </Descriptions>
      </Card>

      {/* 用户信息 */}
      <Card
        size="small"
        title={
          <Space>
            <UserOutlined />
            用户信息
          </Space>
        }
        style={{ marginBottom: 16 }}
      >
        <Space size={16} align="start">
          <Avatar
            size={48}
            src={log.userAvatar}
            icon={<UserOutlined />}
          />
          <div>
            <div>
              <Text strong style={{ fontSize: 16 }}>
                {log.userName || '-'}
              </Text>
            </div>
            <div style={{ marginTop: 4 }}>
              <Text type="secondary">用户 ID: </Text>
              <Text copyable={{ text: log.userId }}>{log.userId}</Text>
            </div>
            {log.userMembershipLevel && (
              <div style={{ marginTop: 4 }}>
                <Text type="secondary">会员等级: </Text>
                <Tag>{log.userMembershipLevel}</Tag>
              </div>
            )}
          </div>
        </Space>
      </Card>

      {/* 徽章信息 */}
      <Card
        size="small"
        title={
          <Space>
            <TrophyOutlined />
            徽章信息
          </Space>
        }
        style={{ marginBottom: 16 }}
      >
        <Space size={16} align="start">
          <Avatar
            size={48}
            src={log.badgeIcon}
            icon={<TrophyOutlined />}
            style={{ backgroundColor: '#f0f0f0' }}
          />
          <div>
            <div>
              <Text strong style={{ fontSize: 16 }}>
                {log.badgeName || '-'}
              </Text>
            </div>
            <div style={{ marginTop: 4 }}>
              <Text type="secondary">徽章 ID: </Text>
              <Text>{log.badgeId}</Text>
            </div>
            {log.badgeType && (
              <div style={{ marginTop: 4 }}>
                <Text type="secondary">徽章类型: </Text>
                <Tag>{log.badgeType}</Tag>
              </div>
            )}
            <div style={{ marginTop: 4 }}>
              <Text type="secondary">操作数量: </Text>
              <Text strong style={{ color: log.action === 'REVOKE' ? '#ff4d4f' : '#52c41a' }}>
                {log.action === 'REVOKE' ? '-' : '+'}{log.quantity}
              </Text>
            </div>
          </div>
        </Space>
      </Card>

      {/* 发放来源 */}
      <Card
        size="small"
        title="发放来源"
        style={{ marginBottom: 16 }}
      >
        <Descriptions column={1} size="small">
          <Descriptions.Item label="来源类型">
            <Tag color={sourceConfig.color}>{sourceConfig.text}</Tag>
          </Descriptions.Item>
          {log.sourceRefId && (
            <Descriptions.Item label="关联 ID">
              <Text copyable>{log.sourceRefId}</Text>
            </Descriptions.Item>
          )}
          {log.batchTaskName && (
            <Descriptions.Item label="批量任务">
              {log.batchTaskName}
            </Descriptions.Item>
          )}
          {log.ruleName && (
            <Descriptions.Item label="触发规则">
              {log.ruleName}
            </Descriptions.Item>
          )}
        </Descriptions>
      </Card>

      {/* 操作信息 */}
      <Card size="small" title="操作信息">
        <Descriptions column={1} size="small">
          {(log.operatorId || log.operatorName) && (
            <Descriptions.Item label="操作人">
              {log.operatorName || log.operatorId}
              {log.operatorName && log.operatorId && (
                <Text type="secondary" style={{ marginLeft: 8 }}>
                  ({log.operatorId})
                </Text>
              )}
            </Descriptions.Item>
          )}
          {log.reason && (
            <>
              <Divider style={{ margin: '8px 0' }} />
              <Descriptions.Item label="备注">
                <Text>{log.reason}</Text>
              </Descriptions.Item>
            </>
          )}
        </Descriptions>
      </Card>
    </Modal>
  );
};

export default LogDetailModal;

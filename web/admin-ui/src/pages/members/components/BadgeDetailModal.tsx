/**
 * 徽章详情弹窗
 *
 * 展示用户持有徽章的完整信息，包括获取时间、来源、状态等
 * 提供撤销入口（当徽章状态为 ACTIVE 时）
 */

import React from 'react';
import {
  Modal,
  Descriptions,
  Avatar,
  Tag,
  Space,
  Button,
  Typography,
  Divider,
  Timeline,
} from 'antd';
import {
  TrophyOutlined,
  ClockCircleOutlined,
  UserOutlined,
  StopOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import type { UserBadgeDetail } from '@/types';
import dayjs from 'dayjs';

const { Text } = Typography;

interface BadgeDetailModalProps {
  /** 是否显示 */
  open: boolean;
  /** 徽章详情数据 */
  badge: UserBadgeDetail | null;
  /** 关闭回调 */
  onClose: () => void;
  /** 点击撤销按钮回调 */
  onRevoke?: (badge: UserBadgeDetail) => void;
}

/**
 * 徽章类型标签
 */
const BadgeTypeTag: React.FC<{ type: string }> = ({ type }) => {
  const typeConfig: Record<string, { color: string; text: string }> = {
    NORMAL: { color: 'default', text: '普通徽章' },
    LIMITED: { color: 'gold', text: '限定徽章' },
    ACHIEVEMENT: { color: 'purple', text: '成就徽章' },
    EVENT: { color: 'blue', text: '活动徽章' },
  };

  const config = typeConfig[type] || { color: 'default', text: type };
  return <Tag color={config.color}>{config.text}</Tag>;
};

/**
 * 状态标签
 */
const StatusTag: React.FC<{ status: string }> = ({ status }) => {
  const statusConfig: Record<string, { color: string; text: string; icon: React.ReactNode }> = {
    ACTIVE: { color: 'success', text: '有效', icon: <CheckCircleOutlined /> },
    EXPIRED: { color: 'default', text: '已过期', icon: <ClockCircleOutlined /> },
    REVOKED: { color: 'error', text: '已撤销', icon: <StopOutlined /> },
    REDEEMED: { color: 'processing', text: '已兑换', icon: <CheckCircleOutlined /> },
  };

  const config = statusConfig[status] || { color: 'default', text: status, icon: null };
  return (
    <Tag color={config.color} icon={config.icon}>
      {config.text}
    </Tag>
  );
};

/**
 * 来源类型文本
 */
const getSourceTypeText = (sourceType: string): string => {
  const sourceMap: Record<string, string> = {
    EVENT: '事件触发',
    SCHEDULED: '定时任务',
    MANUAL: '手动发放',
    REDEMPTION: '兑换获得',
    SYSTEM: '系统发放',
  };
  return sourceMap[sourceType] || sourceType;
};

/**
 * 徽章详情弹窗组件
 */
const BadgeDetailModal: React.FC<BadgeDetailModalProps> = ({
  open,
  badge,
  onClose,
  onRevoke,
}) => {
  if (!badge) return null;

  const canRevoke = badge.status === 'ACTIVE';

  /**
   * 构建时间线数据
   */
  const timelineItems = [
    {
      color: 'green',
      children: (
        <Space direction="vertical" size={0}>
          <Text strong>获取徽章</Text>
          <Text type="secondary">
            {dayjs(badge.grantedAt).format('YYYY-MM-DD HH:mm:ss')}
          </Text>
          {badge.grantReason && <Text type="secondary">原因：{badge.grantReason}</Text>}
        </Space>
      ),
    },
  ];

  if (badge.expiresAt && badge.status !== 'REVOKED') {
    const isExpired = dayjs(badge.expiresAt).isBefore(dayjs());
    timelineItems.push({
      color: isExpired ? 'gray' : 'blue',
      children: (
        <Space direction="vertical" size={0}>
          <Text strong>{isExpired ? '已过期' : '将过期'}</Text>
          <Text type="secondary">
            {dayjs(badge.expiresAt).format('YYYY-MM-DD HH:mm:ss')}
          </Text>
        </Space>
      ),
    });
  }

  if (badge.status === 'REVOKED' && badge.revokedAt) {
    timelineItems.push({
      color: 'red',
      children: (
        <Space direction="vertical" size={0}>
          <Text strong>徽章撤销</Text>
          <Text type="secondary">
            {dayjs(badge.revokedAt).format('YYYY-MM-DD HH:mm:ss')}
          </Text>
          {badge.revokedReason && <Text type="secondary">原因：{badge.revokedReason}</Text>}
        </Space>
      ),
    });
  }

  return (
    <Modal
      title={
        <Space>
          <TrophyOutlined />
          徽章详情
        </Space>
      }
      open={open}
      onCancel={onClose}
      width={600}
      footer={
        <Space>
          <Button onClick={onClose}>关闭</Button>
          {canRevoke && onRevoke && (
            <Button
              danger
              icon={<StopOutlined />}
              onClick={() => onRevoke(badge)}
            >
              撤销徽章
            </Button>
          )}
        </Space>
      }
    >
      {/* 徽章基本信息 */}
      <div style={{ textAlign: 'center', marginBottom: 24 }}>
        <Avatar
          src={badge.badgeIcon}
          size={80}
          shape="square"
          icon={<TrophyOutlined />}
          style={{
            backgroundColor: badge.status !== 'ACTIVE' ? '#f5f5f5' : undefined,
            filter: badge.status !== 'ACTIVE' ? 'grayscale(100%)' : undefined,
          }}
        />
        <div style={{ marginTop: 12 }}>
          <Space>
            <Text strong style={{ fontSize: 18 }}>{badge.badgeName}</Text>
            <StatusTag status={badge.status} />
          </Space>
        </div>
        {badge.badgeDescription && (
          <Text type="secondary" style={{ display: 'block', marginTop: 8 }}>
            {badge.badgeDescription}
          </Text>
        )}
      </div>

      <Divider />

      {/* 详细信息 */}
      <Descriptions column={2} size="small">
        <Descriptions.Item label="徽章类型">
          <BadgeTypeTag type={badge.badgeType} />
        </Descriptions.Item>
        <Descriptions.Item label="持有数量">
          <Text strong>{badge.quantity}</Text> 个
        </Descriptions.Item>
        <Descriptions.Item label="获取来源">
          {getSourceTypeText(badge.sourceType)}
        </Descriptions.Item>
        <Descriptions.Item label="操作人">
          <Space>
            <UserOutlined />
            {badge.operatorName || '-'}
          </Space>
        </Descriptions.Item>
        {badge.sourceRefId && (
          <Descriptions.Item label="来源引用" span={2}>
            <Text code>{badge.sourceRefId}</Text>
          </Descriptions.Item>
        )}
      </Descriptions>

      <Divider orientation="left">时间线</Divider>

      <Timeline items={timelineItems} />

      {/* 撤销提示 */}
      {badge.status === 'REVOKED' && (
        <div
          style={{
            padding: 12,
            backgroundColor: '#fff2f0',
            borderRadius: 4,
            marginTop: 16,
          }}
        >
          <Space>
            <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />
            <Text type="danger">
              此徽章已被撤销
              {badge.revokedReason && `，原因：${badge.revokedReason}`}
            </Text>
          </Space>
        </div>
      )}
    </Modal>
  );
};

export default BadgeDetailModal;

/**
 * 系列徽章预览抽屉组件
 *
 * 展示系列基本信息和系列下的所有徽章
 */

import React from 'react';
import {
  Drawer,
  Card,
  Descriptions,
  Tag,
  Empty,
  Spin,
  Space,
  Image,
  Row,
  Col,
} from 'antd';
import { useSeriesBadges } from '@/hooks/useSeries';
import { formatDate, getBadgeStatusText, getBadgeTypeText } from '@/utils/format';
import type { BadgeSeries, Badge, BadgeStatus, BadgeType } from '@/types';

interface SeriesBadgesDrawerProps {
  /** 抽屉是否可见 */
  open: boolean;
  /** 关闭抽屉回调 */
  onClose: () => void;
  /** 系列数据 */
  series: BadgeSeries | null;
  /** 所属分类名称 */
  categoryName?: string;
}

/**
 * 徽章状态颜色映射
 */
const BADGE_STATUS_COLOR: Record<BadgeStatus, string> = {
  DRAFT: 'default',
  ACTIVE: 'success',
  INACTIVE: 'warning',
  ARCHIVED: 'error',
};

/**
 * 徽章类型颜色映射
 */
const BADGE_TYPE_COLOR: Record<BadgeType, string> = {
  NORMAL: 'blue',
  LIMITED: 'orange',
  ACHIEVEMENT: 'purple',
  EVENT: 'green',
};

/**
 * 徽章卡片组件
 */
const BadgeCard: React.FC<{ badge: Badge }> = ({ badge }) => {
  return (
    <Card
      size="small"
      hoverable
      cover={
        badge.assets.iconUrl ? (
          <div style={{ padding: 16, textAlign: 'center', background: '#fafafa' }}>
            <Image
              src={badge.assets.iconUrl}
              alt={badge.name}
              width={80}
              height={80}
              style={{ objectFit: 'contain' }}
              preview={{
                mask: '预览',
              }}
            />
          </div>
        ) : (
          <div
            style={{
              padding: 16,
              textAlign: 'center',
              background: '#fafafa',
              height: 112,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#999',
            }}
          >
            暂无图标
          </div>
        )
      }
    >
      <Card.Meta
        title={
          <Space size={4} wrap>
            <span style={{ fontSize: 14 }}>{badge.name}</span>
          </Space>
        }
        description={
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Space size={4}>
              <Tag color={BADGE_TYPE_COLOR[badge.badgeType]}>
                {getBadgeTypeText(badge.badgeType)}
              </Tag>
              <Tag color={BADGE_STATUS_COLOR[badge.status]}>
                {getBadgeStatusText(badge.status)}
              </Tag>
            </Space>
            <div style={{ fontSize: 12, color: '#999' }}>
              已发放: {badge.issuedCount}
              {badge.maxSupply && ` / ${badge.maxSupply}`}
            </div>
          </Space>
        }
      />
    </Card>
  );
};

/**
 * 系列徽章预览抽屉
 *
 * 显示系列基本信息和系列下所有徽章的卡片列表
 */
const SeriesBadgesDrawer: React.FC<SeriesBadgesDrawerProps> = ({
  open,
  onClose,
  series,
  categoryName,
}) => {
  // 查询系列下的徽章
  const { data: badges, isLoading } = useSeriesBadges(series?.id ?? 0, open && !!series);

  return (
    <Drawer
      title="系列详情"
      placement="right"
      width={640}
      open={open}
      onClose={onClose}
      destroyOnClose
    >
      {series ? (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          {/* 系列基本信息 */}
          <Card size="small" title="基本信息">
            <Descriptions column={2} size="small">
              <Descriptions.Item label="系列名称" span={2}>
                {series.name}
              </Descriptions.Item>
              <Descriptions.Item label="所属分类">
                {categoryName || `-`}
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={series.status === 'ACTIVE' ? 'success' : 'default'}>
                  {series.status === 'ACTIVE' ? '启用' : '禁用'}
                </Tag>
              </Descriptions.Item>
              {series.coverUrl && (
                <Descriptions.Item label="封面图" span={2}>
                  <Image
                    src={series.coverUrl}
                    alt={series.name}
                    width={100}
                    style={{ borderRadius: 4 }}
                  />
                </Descriptions.Item>
              )}
              {series.description && (
                <Descriptions.Item label="描述" span={2}>
                  {series.description}
                </Descriptions.Item>
              )}
              <Descriptions.Item label="排序值">{series.sortOrder}</Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDate(series.createdAt)}
              </Descriptions.Item>
              {series.startTime && (
                <Descriptions.Item label="开始时间">
                  {formatDate(series.startTime)}
                </Descriptions.Item>
              )}
              {series.endTime && (
                <Descriptions.Item label="结束时间">
                  {formatDate(series.endTime)}
                </Descriptions.Item>
              )}
            </Descriptions>
          </Card>

          {/* 徽章列表 */}
          <Card
            size="small"
            title={`系列徽章 (${badges?.length ?? 0})`}
            bodyStyle={{ padding: badges?.length ? 12 : 24 }}
          >
            {isLoading ? (
              <div style={{ textAlign: 'center', padding: 40 }}>
                <Spin tip="加载中..." />
              </div>
            ) : badges && badges.length > 0 ? (
              <Row gutter={[12, 12]}>
                {badges.map((badge) => (
                  <Col key={badge.id} xs={24} sm={12}>
                    <BadgeCard badge={badge} />
                  </Col>
                ))}
              </Row>
            ) : (
              <Empty
                description="暂无徽章"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            )}
          </Card>
        </Space>
      ) : (
        <Empty description="请选择系列" />
      )}
    </Drawer>
  );
};

export default SeriesBadgesDrawer;

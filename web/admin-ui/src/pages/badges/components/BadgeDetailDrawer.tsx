/**
 * 徽章详情抽屉组件
 *
 * 展示徽章的完整信息，包括基本信息、素材、有效期、库存和发放统计
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
  Progress,
  Statistic,
  Row,
  Col,
} from 'antd';
import {
  TrophyOutlined,
  PictureOutlined,
  ClockCircleOutlined,
  InboxOutlined,
} from '@ant-design/icons';
import { useBadgeDetail } from '@/hooks/useBadge';
import {
  formatDate,
  getBadgeStatusText,
  getBadgeTypeText,
} from '@/utils/format';
import type { BadgeStatus, BadgeType, ValidityType } from '@/types';

interface BadgeDetailDrawerProps {
  /** 抽屉是否可见 */
  open: boolean;
  /** 关闭抽屉回调 */
  onClose: () => void;
  /** 徽章 ID */
  badgeId: number | null;
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
 * 获取有效期类型文本
 */
function getValidityTypeText(type: ValidityType): string {
  const map: Record<ValidityType, string> = {
    PERMANENT: '永久有效',
    RELATIVE_DAYS: '固定天数',
    FIXED_DATE: '固定日期',
  };
  return map[type] || type;
}

/**
 * 徽章详情抽屉
 *
 * 分模块展示徽章的各项配置和统计信息
 */
const BadgeDetailDrawer: React.FC<BadgeDetailDrawerProps> = ({
  open,
  onClose,
  badgeId,
}) => {
  // 查询徽章详情
  const { data: badge, isLoading } = useBadgeDetail(badgeId ?? 0, open && !!badgeId);

  /**
   * 计算库存使用百分比
   */
  const getStockPercent = (): number => {
    if (!badge?.maxSupply) return 0;
    return Math.round((badge.issuedCount / badge.maxSupply) * 100);
  };

  /**
   * 获取库存进度条状态颜色
   */
  const getStockStatus = (): 'success' | 'normal' | 'exception' | 'active' => {
    const percent = getStockPercent();
    if (percent >= 100) return 'exception';
    if (percent >= 80) return 'normal';
    return 'success';
  };

  return (
    <Drawer
      title="徽章详情"
      placement="right"
      width={640}
      open={open}
      onClose={onClose}
      destroyOnClose
    >
      {isLoading ? (
        <div style={{ textAlign: 'center', padding: 100 }}>
          <Spin tip="加载中..." />
        </div>
      ) : badge ? (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          {/* 基本信息 */}
          <Card
            size="small"
            title={
              <Space>
                <TrophyOutlined />
                基本信息
              </Space>
            }
          >
            <Descriptions column={2} size="small">
              <Descriptions.Item label="徽章 ID">{badge.id}</Descriptions.Item>
              <Descriptions.Item label="徽章名称" span={1}>
                {badge.name}
              </Descriptions.Item>
              <Descriptions.Item label="徽章类型">
                <Tag color={BADGE_TYPE_COLOR[badge.badgeType]}>
                  {getBadgeTypeText(badge.badgeType)}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={BADGE_STATUS_COLOR[badge.status]}>
                  {getBadgeStatusText(badge.status)}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="所属分类">
                {badge.categoryName || '-'}
              </Descriptions.Item>
              <Descriptions.Item label="所属系列">
                {badge.seriesName || `-`}
              </Descriptions.Item>
              {badge.description && (
                <Descriptions.Item label="描述" span={2}>
                  {badge.description}
                </Descriptions.Item>
              )}
              {badge.obtainDescription && (
                <Descriptions.Item label="获取条件" span={2}>
                  {badge.obtainDescription}
                </Descriptions.Item>
              )}
              <Descriptions.Item label="排序值">{badge.sortOrder}</Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDate(badge.createdAt)}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间" span={2}>
                {formatDate(badge.updatedAt)}
              </Descriptions.Item>
            </Descriptions>
          </Card>

          {/* 素材信息 */}
          <Card
            size="small"
            title={
              <Space>
                <PictureOutlined />
                素材信息
              </Space>
            }
          >
            <Row gutter={[16, 16]}>
              <Col span={12}>
                <div style={{ marginBottom: 8, color: '#666' }}>徽章图标</div>
                {badge.assets.iconUrl ? (
                  <Image
                    src={badge.assets.iconUrl}
                    alt="徽章图标"
                    width={100}
                    height={100}
                    style={{ objectFit: 'contain', background: '#fafafa' }}
                    preview={{ mask: '预览' }}
                  />
                ) : (
                  <div
                    style={{
                      width: 100,
                      height: 100,
                      background: '#fafafa',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: '#999',
                    }}
                  >
                    暂无图标
                  </div>
                )}
              </Col>
              <Col span={12}>
                <div style={{ marginBottom: 8, color: '#666' }}>详情大图</div>
                {badge.assets.imageUrl ? (
                  <Image
                    src={badge.assets.imageUrl}
                    alt="详情大图"
                    width={100}
                    height={100}
                    style={{ objectFit: 'contain', background: '#fafafa' }}
                    preview={{ mask: '预览' }}
                  />
                ) : (
                  <div
                    style={{
                      width: 100,
                      height: 100,
                      background: '#fafafa',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: '#999',
                    }}
                  >
                    未设置
                  </div>
                )}
              </Col>
              {badge.assets.animationUrl && (
                <Col span={24}>
                  <Descriptions size="small" column={1}>
                    <Descriptions.Item label="动效资源">
                      <a
                        href={badge.assets.animationUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        {badge.assets.animationUrl}
                      </a>
                    </Descriptions.Item>
                  </Descriptions>
                </Col>
              )}
              {badge.assets.disabledIconUrl && (
                <Col span={24}>
                  <div style={{ marginBottom: 8, color: '#666' }}>灰态图标</div>
                  <Image
                    src={badge.assets.disabledIconUrl}
                    alt="灰态图标"
                    width={60}
                    height={60}
                    style={{ objectFit: 'contain', background: '#fafafa' }}
                    preview={{ mask: '预览' }}
                  />
                </Col>
              )}
            </Row>
          </Card>

          {/* 有效期配置 */}
          <Card
            size="small"
            title={
              <Space>
                <ClockCircleOutlined />
                有效期配置
              </Space>
            }
          >
            <Descriptions column={2} size="small">
              <Descriptions.Item label="有效期类型">
                {getValidityTypeText(badge.validityConfig.validityType)}
              </Descriptions.Item>
              {badge.validityConfig.validityType === 'RELATIVE_DAYS' && (
                <Descriptions.Item label="有效天数">
                  {badge.validityConfig.relativeDays} 天
                </Descriptions.Item>
              )}
              {badge.validityConfig.validityType === 'FIXED_DATE' && (
                <Descriptions.Item label="过期日期">
                  {formatDate(badge.validityConfig.fixedDate, 'YYYY-MM-DD')}
                </Descriptions.Item>
              )}
            </Descriptions>
          </Card>

          {/* 库存配置 */}
          <Card
            size="small"
            title={
              <Space>
                <InboxOutlined />
                库存与发放
              </Space>
            }
          >
            <Row gutter={[16, 16]}>
              <Col span={8}>
                <Statistic
                  title="已发放"
                  value={badge.issuedCount}
                  suffix={badge.maxSupply ? ` / ${badge.maxSupply}` : ''}
                />
              </Col>
              {badge.holderCount !== undefined && (
                <Col span={8}>
                  <Statistic title="持有用户数" value={badge.holderCount} />
                </Col>
              )}
              <Col span={8}>
                <Statistic
                  title="库存模式"
                  value={badge.maxSupply ? '限量' : '不限量'}
                  valueStyle={{
                    color: badge.maxSupply ? '#fa8c16' : '#52c41a',
                    fontSize: 16,
                  }}
                />
              </Col>
            </Row>
            {badge.maxSupply && (
              <div style={{ marginTop: 16 }}>
                <div style={{ marginBottom: 8, color: '#666' }}>库存使用率</div>
                <Progress
                  percent={getStockPercent()}
                  status={getStockStatus()}
                  format={(percent) => `${percent}%`}
                />
                <div style={{ marginTop: 8, color: '#999', fontSize: 12 }}>
                  剩余库存: {badge.maxSupply - badge.issuedCount}
                </div>
              </div>
            )}
          </Card>
        </Space>
      ) : (
        <Empty description="请选择徽章" />
      )}
    </Drawer>
  );
};

export default BadgeDetailDrawer;

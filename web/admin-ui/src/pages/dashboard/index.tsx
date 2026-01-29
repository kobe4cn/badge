/**
 * 数据看板页面
 *
 * 展示徽章系统的整体运营数据统计概览，包括：
 * - 今日统计卡片（发放、新增持有者、兑换）
 * - 总量统计卡片（总发放、活跃徽章、持有用户、覆盖率）
 * - 徽章发放排行榜 Top 5
 * 支持自动刷新（5分钟）和手动刷新
 */

import React, { useCallback, useMemo } from 'react';
import { Card, Row, Col, Statistic, Table, Button, Tooltip, Spin, Avatar, Space, Typography } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  TrophyOutlined,
  UserOutlined,
  GiftOutlined,
  RiseOutlined,
  FallOutlined,
  ReloadOutlined,
  FireOutlined,
  CrownOutlined,
  TeamOutlined,
  PercentageOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import {
  useDashboardStats,
  useTodayStats,
  useBadgeRanking,
  useRefreshDashboard,
} from '@/hooks/useDashboard';
import { formatCount, formatRelativeTime } from '@/utils/format';
import type { BadgeRanking } from '@/types/dashboard';

const { Text } = Typography;

/**
 * 变化趋势显示组件
 *
 * 根据变化值显示上涨/下降箭头和颜色
 */
const TrendDisplay: React.FC<{ value: number; suffix?: string }> = ({
  value,
  suffix = '%',
}) => {
  if (value === 0) {
    return <Text type="secondary">-- {suffix}</Text>;
  }

  const isPositive = value > 0;
  const Icon = isPositive ? RiseOutlined : FallOutlined;
  const color = isPositive ? '#52c41a' : '#ff4d4f';
  const displayValue = Math.abs(value).toFixed(1);

  return (
    <span style={{ color, fontSize: 14 }}>
      <Icon style={{ marginRight: 4 }} />
      {displayValue}{suffix}
    </span>
  );
};

/**
 * 今日统计卡片组件
 *
 * 展示单个今日统计指标及其环比变化
 */
interface TodayCardProps {
  title: string;
  value: number | undefined;
  change: number | undefined;
  icon: React.ReactNode;
  iconColor: string;
  loading?: boolean;
}

const TodayCard: React.FC<TodayCardProps> = ({
  title,
  value,
  change,
  icon,
  iconColor,
  loading,
}) => {
  return (
    <Card hoverable>
      <Spin spinning={loading}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <div
            style={{
              width: 56,
              height: 56,
              borderRadius: 12,
              backgroundColor: `${iconColor}15`,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <span style={{ fontSize: 28, color: iconColor }}>{icon}</span>
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ color: '#8c8c8c', fontSize: 14, marginBottom: 4 }}>
              {title}
            </div>
            <div style={{ fontSize: 28, fontWeight: 600, lineHeight: 1.2 }}>
              {value !== undefined ? formatCount(value) : '--'}
            </div>
            <div style={{ marginTop: 4 }}>
              <Text type="secondary" style={{ marginRight: 8 }}>
                较昨日
              </Text>
              {change !== undefined ? (
                <TrendDisplay value={change} />
              ) : (
                <Text type="secondary">--</Text>
              )}
            </div>
          </div>
        </div>
      </Spin>
    </Card>
  );
};

/**
 * 总量统计卡片组件
 *
 * 展示单个总量统计指标
 */
interface SummaryCardProps {
  title: string;
  value: number | string | undefined;
  icon: React.ReactNode;
  iconColor: string;
  suffix?: string;
  loading?: boolean;
}

const SummaryCard: React.FC<SummaryCardProps> = ({
  title,
  value,
  icon,
  iconColor,
  suffix,
  loading,
}) => {
  return (
    <Card hoverable>
      <Spin spinning={loading}>
        <Statistic
          title={
            <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ color: iconColor }}>{icon}</span>
              {title}
            </span>
          }
          value={value !== undefined ? value : '--'}
          suffix={suffix}
          valueStyle={{ fontSize: 24, fontWeight: 600 }}
        />
      </Spin>
    </Card>
  );
};

/**
 * 数据看板页面主组件
 */
const DashboardPage: React.FC = () => {
  // 获取统计数据
  const {
    data: stats,
    isLoading: statsLoading,
    dataUpdatedAt: statsUpdatedAt,
  } = useDashboardStats();

  const {
    data: todayStats,
    isLoading: todayLoading,
    dataUpdatedAt: todayUpdatedAt,
  } = useTodayStats();

  const {
    data: ranking,
    isLoading: rankingLoading,
    dataUpdatedAt: rankingUpdatedAt,
  } = useBadgeRanking({ type: 'grant', limit: 5 });

  const { refresh } = useRefreshDashboard();

  // 计算最后更新时间（取三个查询中最新的时间）
  const lastUpdatedAt = useMemo(() => {
    const times = [statsUpdatedAt, todayUpdatedAt, rankingUpdatedAt].filter(Boolean);
    if (times.length === 0) return null;
    return new Date(Math.max(...times));
  }, [statsUpdatedAt, todayUpdatedAt, rankingUpdatedAt]);

  // 手动刷新处理
  const handleRefresh = useCallback(() => {
    refresh();
  }, [refresh]);

  // 排行榜表格列定义
  const rankingColumns: ColumnsType<BadgeRanking> = [
    {
      title: '排名',
      dataIndex: 'rank',
      key: 'rank',
      width: 60,
      render: (rank: number) => {
        // 前三名使用特殊样式
        if (rank <= 3) {
          const colors = ['#ffd700', '#c0c0c0', '#cd7f32'];
          return (
            <CrownOutlined
              style={{ fontSize: 18, color: colors[rank - 1] }}
            />
          );
        }
        return <Text type="secondary">{rank}</Text>;
      },
    },
    {
      title: '徽章',
      key: 'badge',
      render: (_: unknown, record: BadgeRanking) => (
        <Space>
          <Avatar
            src={record.badgeIcon}
            size={32}
            icon={<TrophyOutlined />}
            style={{ backgroundColor: '#f0f0f0' }}
          />
          <Text strong>{record.badgeName}</Text>
        </Space>
      ),
    },
    {
      title: '发放次数',
      dataIndex: 'grantCount',
      key: 'grantCount',
      width: 120,
      align: 'right',
      render: (count: number) => (
        <Text strong style={{ color: '#1677ff' }}>
          {formatCount(count)}
        </Text>
      ),
    },
    {
      title: '持有人数',
      dataIndex: 'holderCount',
      key: 'holderCount',
      width: 120,
      align: 'right',
      render: (count: number) => formatCount(count),
    },
  ];

  return (
    <PageContainer
      title="数据看板"
      extra={
        <Space>
          {lastUpdatedAt && (
            <Text type="secondary">
              最后更新: {formatRelativeTime(lastUpdatedAt)}
            </Text>
          )}
          <Tooltip title="刷新数据">
            <Button
              icon={<ReloadOutlined />}
              onClick={handleRefresh}
              loading={statsLoading || todayLoading || rankingLoading}
            >
              刷新
            </Button>
          </Tooltip>
        </Space>
      }
    >
      {/* 第一行：今日统计 */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={24} md={8}>
          <TodayCard
            title="今日发放"
            value={todayStats?.grants}
            change={todayStats?.grantsChange}
            icon={<GiftOutlined />}
            iconColor="#1677ff"
            loading={todayLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={8}>
          <TodayCard
            title="新增持有者"
            value={todayStats?.newHolders}
            change={todayStats?.holdersChange}
            icon={<UserOutlined />}
            iconColor="#52c41a"
            loading={todayLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={8}>
          <TodayCard
            title="今日兑换"
            value={todayStats?.redemptions}
            change={todayStats?.redemptionsChange}
            icon={<FireOutlined />}
            iconColor="#fa8c16"
            loading={todayLoading}
          />
        </Col>
      </Row>

      {/* 第二行：总量统计 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} sm={12} md={6}>
          <SummaryCard
            title="总发放数"
            value={stats ? formatCount(stats.totalGrants) : undefined}
            icon={<TrophyOutlined />}
            iconColor="#1677ff"
            loading={statsLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={6}>
          <SummaryCard
            title="活跃徽章"
            value={stats?.activeBadges}
            icon={<RiseOutlined />}
            iconColor="#52c41a"
            suffix="种"
            loading={statsLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={6}>
          <SummaryCard
            title="持有用户"
            value={stats ? formatCount(stats.badgeHolders) : undefined}
            icon={<TeamOutlined />}
            iconColor="#722ed1"
            loading={statsLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={6}>
          <SummaryCard
            title="用户覆盖率"
            value={
              stats?.userCoverageRate !== undefined
                ? (stats.userCoverageRate * 100).toFixed(1)
                : undefined
            }
            icon={<PercentageOutlined />}
            iconColor="#eb2f96"
            suffix="%"
            loading={statsLoading}
          />
        </Col>
      </Row>

      {/* 第三行：徽章发放排行 */}
      <Card
        title={
          <Space>
            <CrownOutlined style={{ color: '#faad14' }} />
            徽章发放排行 Top 5
          </Space>
        }
        style={{ marginTop: 16 }}
      >
        <Table<BadgeRanking>
          columns={rankingColumns}
          dataSource={ranking || []}
          rowKey="badgeId"
          loading={rankingLoading}
          pagination={false}
          size="middle"
          locale={{
            emptyText: '暂无数据',
          }}
        />
      </Card>
    </PageContainer>
  );
};

export default DashboardPage;

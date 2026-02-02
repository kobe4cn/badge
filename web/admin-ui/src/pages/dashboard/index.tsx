/**
 * 数据看板页面
 *
 * 展示徽章系统的整体运营数据统计概览，包括：
 * - 今日统计卡片（发放、新增持有者、兑换）
 * - 总量统计卡片（总发放、活跃徽章、持有用户、覆盖率）
 * - 徽章发放排行榜 Top 5
 * - 发放趋势折线图（支持时间范围切换）
 * - 徽章类型分布饼图
 * - 热门徽章排行榜 Top 10
 * - 用户活跃度趋势
 * 支持自动刷新（5分钟）和手动刷新
 */

import React, { useCallback, useMemo, useState } from 'react';
import {
  Card,
  Row,
  Col,
  Statistic,
  Table,
  Button,
  Tooltip,
  Spin,
  Avatar,
  Space,
  Typography,
  Segmented,
  DatePicker,
  Empty,
} from 'antd';
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
  LineChartOutlined,
  PieChartOutlined,
  BarChartOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import {
  useDashboardStats,
  useTodayStats,
  useBadgeRanking,
  useRefreshDashboard,
  useGrantTrend,
  useBadgeTypeDistribution,
  useUserActivityTrend,
  useTopBadges,
} from '@/hooks/useDashboard';
import { formatCount, formatRelativeTime } from '@/utils/format';
import { LineChart, PieChart, BarChart } from '@/components';
import type { BadgeRanking, TimeRangePreset } from '@/types/dashboard';

const { RangePicker } = DatePicker;

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
    <Card hoverable className="stat-card">
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
    <Card hoverable className="stat-card">
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
 * 时间范围预设选项配置
 */
const TIME_RANGE_OPTIONS = [
  { label: '7天', value: '7d' },
  { label: '30天', value: '30d' },
  { label: '90天', value: '90d' },
  { label: '自定义', value: 'custom' },
];

/**
 * 根据预设选项计算日期范围
 */
function getDateRangeByPreset(preset: TimeRangePreset): { startDate: string; endDate: string } {
  const endDate = dayjs().format('YYYY-MM-DD');
  let startDate: string;

  switch (preset) {
    case '7d':
      startDate = dayjs().subtract(6, 'day').format('YYYY-MM-DD');
      break;
    case '30d':
      startDate = dayjs().subtract(29, 'day').format('YYYY-MM-DD');
      break;
    case '90d':
      startDate = dayjs().subtract(89, 'day').format('YYYY-MM-DD');
      break;
    default:
      startDate = dayjs().subtract(6, 'day').format('YYYY-MM-DD');
  }

  return { startDate, endDate };
}

/**
 * 数据看板页面主组件
 */
const DashboardPage: React.FC = () => {
  // 时间范围状态
  const [trendTimeRange, setTrendTimeRange] = useState<TimeRangePreset>('7d');
  const [customDateRange, setCustomDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs] | null>(null);
  const [activityTimeRange, setActivityTimeRange] = useState<TimeRangePreset>('7d');
  const [activityCustomRange, setActivityCustomRange] = useState<[dayjs.Dayjs, dayjs.Dayjs] | null>(null);

  // 计算趋势图的日期参数
  const trendParams = useMemo(() => {
    if (trendTimeRange === 'custom' && customDateRange) {
      return {
        startDate: customDateRange[0].format('YYYY-MM-DD'),
        endDate: customDateRange[1].format('YYYY-MM-DD'),
      };
    }
    return getDateRangeByPreset(trendTimeRange);
  }, [trendTimeRange, customDateRange]);

  // 计算活跃度图的日期参数
  const activityParams = useMemo(() => {
    if (activityTimeRange === 'custom' && activityCustomRange) {
      return {
        startDate: activityCustomRange[0].format('YYYY-MM-DD'),
        endDate: activityCustomRange[1].format('YYYY-MM-DD'),
      };
    }
    return getDateRangeByPreset(activityTimeRange);
  }, [activityTimeRange, activityCustomRange]);

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

  // 获取趋势图表数据
  const {
    data: grantTrendData,
    isLoading: grantTrendLoading,
  } = useGrantTrend(trendParams);

  const {
    data: typeDistributionData,
    isLoading: typeDistributionLoading,
  } = useBadgeTypeDistribution();

  const {
    data: topBadgesData,
    isLoading: topBadgesLoading,
  } = useTopBadges(10);

  const {
    data: activityTrendData,
    isLoading: activityTrendLoading,
  } = useUserActivityTrend(activityParams);

  const { refresh } = useRefreshDashboard();

  // 计算最后更新时间（取所有查询中最新的时间）
  const lastUpdatedAt = useMemo(() => {
    const times = [statsUpdatedAt, todayUpdatedAt, rankingUpdatedAt].filter(Boolean);
    if (times.length === 0) return null;
    return new Date(Math.max(...times));
  }, [statsUpdatedAt, todayUpdatedAt, rankingUpdatedAt]);

  // 手动刷新处理
  const handleRefresh = useCallback(() => {
    refresh();
  }, [refresh]);

  // 处理趋势图时间范围变化
  const handleTrendTimeRangeChange = useCallback((value: string | number) => {
    setTrendTimeRange(value as TimeRangePreset);
    if (value !== 'custom') {
      setCustomDateRange(null);
    }
  }, []);

  // 处理活跃度图时间范围变化
  const handleActivityTimeRangeChange = useCallback((value: string | number) => {
    setActivityTimeRange(value as TimeRangePreset);
    if (value !== 'custom') {
      setActivityCustomRange(null);
    }
  }, []);

  // 转换饼图数据格式
  const pieChartData = useMemo(() => {
    if (!typeDistributionData) return [];
    return typeDistributionData.map((item) => ({
      name: item.typeName,
      value: item.count,
    }));
  }, [typeDistributionData]);

  // 转换柱状图数据格式
  const barChartData = useMemo(() => {
    if (!topBadgesData) return [];
    return topBadgesData.map((item) => ({
      name: item.badgeName,
      value: item.grantCount,
    }));
  }, [topBadgesData]);

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

  const isAnyLoading = statsLoading || todayLoading || rankingLoading;

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
              loading={isAnyLoading}
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

      {/* 第四行：趋势图表（2列） */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        {/* 左侧：发放趋势折线图 */}
        <Col xs={24} lg={12}>
          <Card
            title={
              <Space>
                <LineChartOutlined style={{ color: '#1677ff' }} />
                发放趋势
              </Space>
            }
            extra={
              <Space>
                <Segmented
                  options={TIME_RANGE_OPTIONS}
                  value={trendTimeRange}
                  onChange={handleTrendTimeRangeChange}
                  size="small"
                />
                {trendTimeRange === 'custom' && (
                  <RangePicker
                    size="small"
                    value={customDateRange}
                    onChange={(dates) => setCustomDateRange(dates as [dayjs.Dayjs, dayjs.Dayjs] | null)}
                    allowClear={false}
                  />
                )}
              </Space>
            }
          >
            {grantTrendData && grantTrendData.length > 0 ? (
              <LineChart
                data={grantTrendData}
                height={320}
                loading={grantTrendLoading}
                color="#1677ff"
              />
            ) : (
              <div style={{ height: 320, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {grantTrendLoading ? <Spin /> : <Empty description="暂无数据" />}
              </div>
            )}
          </Card>
        </Col>

        {/* 右侧：徽章类型分布饼图 */}
        <Col xs={24} lg={12}>
          <Card
            title={
              <Space>
                <PieChartOutlined style={{ color: '#52c41a' }} />
                徽章类型分布
              </Space>
            }
          >
            {pieChartData.length > 0 ? (
              <PieChart
                data={pieChartData}
                height={320}
                loading={typeDistributionLoading}
                donut
              />
            ) : (
              <div style={{ height: 320, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {typeDistributionLoading ? <Spin /> : <Empty description="暂无数据" />}
              </div>
            )}
          </Card>
        </Col>
      </Row>

      {/* 第五行：排行榜和活跃度（2列） */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        {/* 左侧：热门徽章排行（横向柱状图 Top 10） */}
        <Col xs={24} lg={12}>
          <Card
            title={
              <Space>
                <BarChartOutlined style={{ color: '#fa8c16' }} />
                热门徽章 Top 10
              </Space>
            }
          >
            {barChartData.length > 0 ? (
              <BarChart
                data={barChartData}
                height={320}
                loading={topBadgesLoading}
                horizontal
                color="#fa8c16"
                gradient
              />
            ) : (
              <div style={{ height: 320, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {topBadgesLoading ? <Spin /> : <Empty description="暂无数据" />}
              </div>
            )}
          </Card>
        </Col>

        {/* 右侧：用户活跃度趋势 */}
        <Col xs={24} lg={12}>
          <Card
            title={
              <Space>
                <LineChartOutlined style={{ color: '#722ed1' }} />
                用户活跃度趋势
              </Space>
            }
            extra={
              <Space>
                <Segmented
                  options={TIME_RANGE_OPTIONS}
                  value={activityTimeRange}
                  onChange={handleActivityTimeRangeChange}
                  size="small"
                />
                {activityTimeRange === 'custom' && (
                  <RangePicker
                    size="small"
                    value={activityCustomRange}
                    onChange={(dates) => setActivityCustomRange(dates as [dayjs.Dayjs, dayjs.Dayjs] | null)}
                    allowClear={false}
                  />
                )}
              </Space>
            }
          >
            {activityTrendData && activityTrendData.length > 0 ? (
              <LineChart
                data={activityTrendData}
                height={320}
                loading={activityTrendLoading}
                color="#722ed1"
              />
            ) : (
              <div style={{ height: 320, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {activityTrendLoading ? <Spin /> : <Empty description="暂无数据" />}
              </div>
            )}
          </Card>
        </Col>
      </Row>
    </PageContainer>
  );
};

export default DashboardPage;

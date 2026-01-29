/**
 * 会员徽章查询页面
 *
 * 支持搜索用户、展示用户徽章墙、查看徽章详情和撤销徽章
 */

import React, { useState, useMemo, useCallback } from 'react';
import {
  Card,
  Space,
  Avatar,
  Tag,
  Row,
  Col,
  Statistic,
  Empty,
  Spin,
  Select,
  Typography,
  Button,
  Tooltip,
  Segmented,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  UserOutlined,
  SearchOutlined,
  TrophyOutlined,
  ClockCircleOutlined,
  StopOutlined,
  CheckCircleOutlined,
  GiftOutlined,
  FilterOutlined,
  AppstoreOutlined,
  BarsOutlined,
} from '@ant-design/icons';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  useSearchMembers,
  useMemberDetail,
  useMemberBadges,
  useMemberBadgeStats,
  useRevokeBadge,
} from '@/hooks/useMember';
import { getMembershipConfig } from '@/types/user';
import type { User, UserBadgeDetail, UserBadgeStatus, MembershipLevel } from '@/types';
import BadgeDetailModal from './components/BadgeDetailModal';
import RevokeBadgeModal from './components/RevokeBadgeModal';
import dayjs from 'dayjs';

const { Text, Title } = Typography;

/**
 * 徽章状态筛选选项
 */
const STATUS_FILTER_OPTIONS = [
  { label: '全部', value: 'all' },
  { label: '有效', value: 'ACTIVE' },
  { label: '已过期', value: 'EXPIRED' },
  { label: '已撤销', value: 'REVOKED' },
];

/**
 * 视图模式
 */
type ViewMode = 'grid' | 'list';

/**
 * 徽章状态标签
 */
const BadgeStatusTag: React.FC<{ status: UserBadgeStatus }> = ({ status }) => {
  const config: Record<UserBadgeStatus, { color: string; text: string; icon: React.ReactNode }> = {
    ACTIVE: { color: 'success', text: '有效', icon: <CheckCircleOutlined /> },
    EXPIRED: { color: 'default', text: '已过期', icon: <ClockCircleOutlined /> },
    REVOKED: { color: 'error', text: '已撤销', icon: <StopOutlined /> },
    REDEEMED: { color: 'processing', text: '已兑换', icon: <CheckCircleOutlined /> },
  };

  const statusConfig = config[status] || { color: 'default', text: status, icon: null };
  return (
    <Tag color={statusConfig.color} icon={statusConfig.icon} style={{ margin: 0 }}>
      {statusConfig.text}
    </Tag>
  );
};

/**
 * 徽章卡片组件（网格视图）
 */
const BadgeCard: React.FC<{
  badge: UserBadgeDetail;
  onClick: () => void;
}> = ({ badge, onClick }) => {
  const isInactive = badge.status !== 'ACTIVE';

  return (
    <Card
      hoverable
      size="small"
      onClick={onClick}
      style={{
        textAlign: 'center',
        cursor: 'pointer',
        opacity: isInactive ? 0.6 : 1,
      }}
      styles={{
        body: { padding: 16 },
      }}
    >
      <Avatar
        src={badge.badgeIcon}
        size={64}
        shape="square"
        icon={<TrophyOutlined />}
        style={{
          backgroundColor: isInactive ? '#f5f5f5' : undefined,
          filter: isInactive ? 'grayscale(100%)' : undefined,
        }}
      />
      <div style={{ marginTop: 12 }}>
        <Tooltip title={badge.badgeName}>
          <Text
            strong
            ellipsis
            style={{ display: 'block', marginBottom: 4 }}
          >
            {badge.badgeName}
          </Text>
        </Tooltip>
        <BadgeStatusTag status={badge.status} />
        <Text
          type="secondary"
          style={{ display: 'block', fontSize: 12, marginTop: 4 }}
        >
          {dayjs(badge.grantedAt).format('YYYY-MM-DD')}
        </Text>
      </div>
    </Card>
  );
};

/**
 * 徽章列表项组件（列表视图）
 */
const BadgeListItem: React.FC<{
  badge: UserBadgeDetail;
  onClick: () => void;
}> = ({ badge, onClick }) => {
  const isInactive = badge.status !== 'ACTIVE';

  return (
    <Card
      hoverable
      size="small"
      onClick={onClick}
      style={{ cursor: 'pointer', opacity: isInactive ? 0.7 : 1 }}
      styles={{ body: { padding: '12px 16px' } }}
    >
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space>
          <Avatar
            src={badge.badgeIcon}
            size={48}
            shape="square"
            icon={<TrophyOutlined />}
            style={{
              backgroundColor: isInactive ? '#f5f5f5' : undefined,
              filter: isInactive ? 'grayscale(100%)' : undefined,
            }}
          />
          <div>
            <Text strong>{badge.badgeName}</Text>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                获取时间：{dayjs(badge.grantedAt).format('YYYY-MM-DD HH:mm')}
              </Text>
            </div>
          </div>
        </Space>
        <Space>
          <Text type="secondary">x{badge.quantity}</Text>
          <BadgeStatusTag status={badge.status} />
        </Space>
      </Space>
    </Card>
  );
};

/**
 * 会员徽章查询页面组件
 */
const MemberSearchPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  // 从 URL 获取初始用户 ID
  const initialUserId = searchParams.get('userId') || '';

  // 状态管理
  const [searchKeyword, setSearchKeyword] = useState('');
  const [selectedUserId, setSelectedUserId] = useState(initialUserId);
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [selectedBadge, setSelectedBadge] = useState<UserBadgeDetail | null>(null);
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [revokeModalOpen, setRevokeModalOpen] = useState(false);
  const [badgeToRevoke, setBadgeToRevoke] = useState<UserBadgeDetail | null>(null);

  // 数据查询
  const { data: searchResults, isLoading: isSearching } = useSearchMembers(searchKeyword);
  const { data: memberDetail, isLoading: isLoadingDetail } = useMemberDetail(selectedUserId);
  const { data: memberBadges, isLoading: isLoadingBadges } = useMemberBadges(selectedUserId);
  const { data: badgeStats, isLoading: isLoadingStats } = useMemberBadgeStats(selectedUserId);

  // 撤销 mutation
  const { mutateAsync: doRevoke, isPending: isRevoking } = useRevokeBadge();

  /**
   * 筛选后的徽章列表
   */
  const filteredBadges = useMemo(() => {
    if (!memberBadges) return [];
    if (statusFilter === 'all') return memberBadges;
    return memberBadges.filter((badge) => badge.status === statusFilter);
  }, [memberBadges, statusFilter]);

  /**
   * 处理用户选择
   */
  const handleUserSelect = useCallback((userId: string) => {
    setSelectedUserId(userId);
    setSearchParams({ userId });
    setSearchKeyword('');
  }, [setSearchParams]);

  /**
   * 处理搜索
   */
  const handleSearch = useCallback((value: string) => {
    setSearchKeyword(value);
  }, []);

  /**
   * 打开徽章详情
   */
  const handleBadgeClick = useCallback((badge: UserBadgeDetail) => {
    setSelectedBadge(badge);
    setDetailModalOpen(true);
  }, []);

  /**
   * 打开撤销弹窗
   */
  const handleOpenRevoke = useCallback((badge: UserBadgeDetail) => {
    setBadgeToRevoke(badge);
    setDetailModalOpen(false);
    setRevokeModalOpen(true);
  }, []);

  /**
   * 确认撤销
   */
  const handleConfirmRevoke = useCallback(async (userBadgeId: number, reason: string) => {
    try {
      await doRevoke({ userBadgeId, reason });
      setRevokeModalOpen(false);
      setBadgeToRevoke(null);
    } catch {
      // 错误已在 hook 中处理
    }
  }, [doRevoke]);

  /**
   * 跳转到发放页面
   */
  const handleGoToGrant = useCallback(() => {
    if (selectedUserId) {
      navigate(`/grants/manual?userId=${selectedUserId}`);
    } else {
      navigate('/grants/manual');
    }
  }, [navigate, selectedUserId]);

  /**
   * 渲染用户搜索下拉选项
   */
  const renderUserOptions = () => {
    if (!searchResults) return [];

    return searchResults.map((user: User) => {
      const memberConfig = getMembershipConfig(user.membershipLevel);
      return {
        value: user.userId,
        label: (
          <Space>
            <Avatar size="small" icon={<UserOutlined />} />
            <div>
              <div>
                <Text strong>{user.username}</Text>
                <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                  {user.userId}
                </Text>
              </div>
              <Space size={4}>
                {user.phone && (
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {user.phone}
                  </Text>
                )}
                <Tag color={memberConfig.color} style={{ fontSize: 10, margin: 0 }}>
                  {memberConfig.name}
                </Tag>
              </Space>
            </div>
          </Space>
        ),
      };
    });
  };

  /**
   * 渲染用户信息卡片
   */
  const renderUserInfo = () => {
    if (!memberDetail) return null;

    const memberConfig = getMembershipConfig(memberDetail.membershipLevel as MembershipLevel);

    return (
      <Card style={{ marginBottom: 24 }}>
        <Space size="large" align="start">
          <Avatar
            src={memberDetail.avatar}
            size={80}
            icon={<UserOutlined />}
          />
          <div>
            <Space align="center" style={{ marginBottom: 8 }}>
              <Title level={4} style={{ margin: 0 }}>
                {memberDetail.nickname}
              </Title>
              <Tag color={memberConfig.color}>{memberConfig.name}</Tag>
            </Space>
            <div style={{ color: '#8c8c8c' }}>
              <Space split={<span style={{ color: '#d9d9d9' }}>|</span>}>
                <span>ID: {memberDetail.userId}</span>
                {memberDetail.phone && <span>手机: {memberDetail.phone}</span>}
                <span>
                  注册时间: {dayjs(memberDetail.registeredAt).format('YYYY-MM-DD')}
                </span>
                <span>
                  最后活跃: {dayjs(memberDetail.lastActiveAt).format('YYYY-MM-DD HH:mm')}
                </span>
              </Space>
            </div>
          </div>
        </Space>
      </Card>
    );
  };

  /**
   * 渲染徽章统计
   */
  const renderBadgeStats = () => {
    if (!badgeStats) return null;

    return (
      <Card style={{ marginBottom: 24 }}>
        <Row gutter={24}>
          <Col span={4}>
            <Statistic
              title="总徽章数"
              value={badgeStats.totalBadges}
              prefix={<TrophyOutlined />}
            />
          </Col>
          <Col span={4}>
            <Statistic
              title="徽章种类"
              value={badgeStats.totalTypes}
              valueStyle={{ color: '#1890ff' }}
            />
          </Col>
          <Col span={4}>
            <Statistic
              title="有效徽章"
              value={badgeStats.activeBadges}
              valueStyle={{ color: '#52c41a' }}
              prefix={<CheckCircleOutlined />}
            />
          </Col>
          <Col span={4}>
            <Statistic
              title="已过期"
              value={badgeStats.expiredBadges}
              valueStyle={{ color: '#8c8c8c' }}
              prefix={<ClockCircleOutlined />}
            />
          </Col>
          <Col span={4}>
            <Statistic
              title="已撤销"
              value={badgeStats.revokedBadges}
              valueStyle={{ color: '#ff4d4f' }}
              prefix={<StopOutlined />}
            />
          </Col>
        </Row>
      </Card>
    );
  };

  /**
   * 渲染徽章墙
   */
  const renderBadgeWall = () => {
    if (isLoadingBadges) {
      return (
        <div style={{ textAlign: 'center', padding: 48 }}>
          <Spin size="large" />
        </div>
      );
    }

    if (!memberBadges || memberBadges.length === 0) {
      return (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="该用户暂无徽章"
          style={{ padding: 48 }}
        >
          <Button type="primary" icon={<GiftOutlined />} onClick={handleGoToGrant}>
            发放徽章
          </Button>
        </Empty>
      );
    }

    if (filteredBadges.length === 0) {
      return (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="没有符合条件的徽章"
          style={{ padding: 48 }}
        />
      );
    }

    if (viewMode === 'grid') {
      return (
        <Row gutter={[16, 16]}>
          {filteredBadges.map((badge) => (
            <Col key={badge.id} xs={12} sm={8} md={6} lg={4} xl={3}>
              <BadgeCard badge={badge} onClick={() => handleBadgeClick(badge)} />
            </Col>
          ))}
        </Row>
      );
    }

    return (
      <Space direction="vertical" style={{ width: '100%' }} size={8}>
        {filteredBadges.map((badge) => (
          <BadgeListItem
            key={badge.id}
            badge={badge}
            onClick={() => handleBadgeClick(badge)}
          />
        ))}
      </Space>
    );
  };

  /**
   * 渲染空状态（未选择用户）
   */
  const renderEmptyState = () => (
    <Card>
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description="请搜索并选择用户查看徽章"
        style={{ padding: 48 }}
      >
        <Text type="secondary">
          支持通过用户 ID、手机号、昵称进行搜索
        </Text>
      </Empty>
    </Card>
  );

  return (
    <PageContainer
      title="用户徽章查询"
      extra={
        selectedUserId && (
          <Button type="primary" icon={<GiftOutlined />} onClick={handleGoToGrant}>
            发放徽章
          </Button>
        )
      }
    >
      {/* 搜索栏 */}
      <Card style={{ marginBottom: 24 }}>
        <Select
          showSearch
          placeholder="输入用户 ID、手机号或昵称搜索"
          style={{ width: '100%', maxWidth: 600 }}
          size="large"
          suffixIcon={<SearchOutlined />}
          filterOption={false}
          onSearch={handleSearch}
          onChange={handleUserSelect}
          value={selectedUserId || undefined}
          options={renderUserOptions()}
          loading={isSearching}
          notFoundContent={
            searchKeyword.length < 2 ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="请输入至少 2 个字符开始搜索"
              />
            ) : isSearching ? (
              <Spin size="small" />
            ) : (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="未找到匹配的用户"
              />
            )
          }
          allowClear
          onClear={() => {
            setSelectedUserId('');
            setSearchParams({});
          }}
        />
      </Card>

      {/* 加载状态 */}
      {selectedUserId && (isLoadingDetail || isLoadingStats) && (
        <div style={{ textAlign: 'center', padding: 48 }}>
          <Spin size="large" />
        </div>
      )}

      {/* 用户信息和徽章 */}
      {selectedUserId && memberDetail && (
        <>
          {renderUserInfo()}
          {renderBadgeStats()}

          {/* 徽章墙 */}
          <Card
            title={
              <Space>
                <TrophyOutlined />
                徽章墙
              </Space>
            }
            extra={
              <Space>
                <Space>
                  <FilterOutlined />
                  <Segmented
                    options={STATUS_FILTER_OPTIONS}
                    value={statusFilter}
                    onChange={(value) => setStatusFilter(value as string)}
                  />
                </Space>
                <Segmented
                  options={[
                    { value: 'grid', icon: <AppstoreOutlined /> },
                    { value: 'list', icon: <BarsOutlined /> },
                  ]}
                  value={viewMode}
                  onChange={(value) => setViewMode(value as ViewMode)}
                />
              </Space>
            }
          >
            {renderBadgeWall()}
          </Card>
        </>
      )}

      {/* 未选择用户时的空状态 */}
      {!selectedUserId && renderEmptyState()}

      {/* 徽章详情弹窗 */}
      <BadgeDetailModal
        open={detailModalOpen}
        badge={selectedBadge}
        onClose={() => {
          setDetailModalOpen(false);
          setSelectedBadge(null);
        }}
        onRevoke={handleOpenRevoke}
      />

      {/* 撤销确认弹窗 */}
      <RevokeBadgeModal
        open={revokeModalOpen}
        badge={badgeToRevoke}
        loading={isRevoking}
        onClose={() => {
          setRevokeModalOpen(false);
          setBadgeToRevoke(null);
        }}
        onConfirm={handleConfirmRevoke}
      />
    </PageContainer>
  );
};

export default MemberSearchPage;

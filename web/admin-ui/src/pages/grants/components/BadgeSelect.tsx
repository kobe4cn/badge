/**
 * 徽章选择组件
 *
 * 支持按分类/系列筛选，显示徽章图标、名称和库存信息
 * 只显示已上架的徽章供发放使用
 */

import React, { useState, useMemo } from 'react';
import { Select, Space, Avatar, Tag, Typography, Spin, Empty, Divider } from 'antd';
import { useBadgeList } from '@/hooks/useBadge';
import { useAllCategories } from '@/hooks/useCategory';
import { useAllSeries } from '@/hooks/useSeries';
import type { BadgeListItem } from '@/services/badge';
import type { SelectProps } from 'antd';

const { Text } = Typography;

/**
 * 徽章选择值类型
 */
export interface BadgeSelectValue {
  value: number;
  label: React.ReactNode;
  badge: BadgeListItem;
}

interface BadgeSelectProps {
  /** 选中的徽章 */
  value?: BadgeSelectValue;
  /** 值变更回调 */
  onChange?: (value: BadgeSelectValue | undefined) => void;
  /** 占位符 */
  placeholder?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 自定义样式 */
  style?: React.CSSProperties;
}

/**
 * 计算徽章可用库存
 */
const getAvailableStock = (badge: BadgeListItem): string => {
  if (badge.maxSupply === undefined || badge.maxSupply === null) {
    return '不限量';
  }
  const available = badge.maxSupply - badge.issuedCount;
  return `剩余 ${available}`;
};

/**
 * 检查徽章是否有库存
 */
const hasStock = (badge: BadgeListItem): boolean => {
  if (badge.maxSupply === undefined || badge.maxSupply === null) {
    return true;
  }
  return badge.maxSupply - badge.issuedCount > 0;
};

/**
 * 渲染徽章选项
 */
const renderBadgeOption = (badge: BadgeListItem) => {
  const available = hasStock(badge);
  const stockText = getAvailableStock(badge);

  return (
    <Space>
      <Avatar
        size={32}
        src={badge.assets.iconUrl}
        shape="square"
        style={{ opacity: available ? 1 : 0.5 }}
      />
      <div>
        <div>
          <Text strong style={{ color: available ? undefined : '#999' }}>
            {badge.name}
          </Text>
          {badge.seriesName && (
            <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
              {badge.seriesName}
            </Text>
          )}
        </div>
        <Space size={4}>
          <Tag
            color={available ? 'green' : 'default'}
            style={{ fontSize: 10, margin: 0 }}
          >
            {stockText}
          </Tag>
          <Tag style={{ fontSize: 10, margin: 0 }}>
            {badge.badgeType === 'NORMAL'
              ? '普通'
              : badge.badgeType === 'LIMITED'
              ? '限定'
              : badge.badgeType === 'ACHIEVEMENT'
              ? '成就'
              : '活动'}
          </Tag>
        </Space>
      </div>
    </Space>
  );
};

/**
 * 徽章选择组件
 *
 * 提供分类/系列筛选和徽章选择功能
 */
const BadgeSelect: React.FC<BadgeSelectProps> = ({
  value,
  onChange,
  placeholder = '请选择徽章',
  disabled = false,
  style,
}) => {
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | undefined>();
  const [selectedSeriesId, setSelectedSeriesId] = useState<number | undefined>();
  const [searchKeyword, setSearchKeyword] = useState('');

  // 查询分类列表
  const { data: categories, isLoading: categoriesLoading } = useAllCategories();

  // 查询系列列表（根据分类筛选）
  const { data: seriesList, isLoading: seriesLoading } = useAllSeries(selectedCategoryId);

  // 查询徽章列表（只查询已上架的）
  const { data: badgesData, isLoading: badgesLoading } = useBadgeList({
    status: 'ACTIVE',
    seriesId: selectedSeriesId,
    categoryId: selectedCategoryId,
    name: searchKeyword || undefined,
    page: 1,
    pageSize: 100,
  });

  // 分类选项
  const categoryOptions = useMemo(
    () =>
      categories?.map((cat) => ({
        label: cat.name,
        value: cat.id,
      })) || [],
    [categories]
  );

  // 系列选项
  const seriesOptions = useMemo(
    () =>
      seriesList?.map((series) => ({
        label: series.name,
        value: series.id,
      })) || [],
    [seriesList]
  );

  // 徽章选项
  const badgeOptions = useMemo(() => {
    if (!badgesData?.items) return [];

    return badgesData.items.map((badge) => ({
      value: badge.id,
      label: renderBadgeOption(badge),
      badge,
      disabled: !hasStock(badge),
    }));
  }, [badgesData]);

  /**
   * 处理分类变更
   */
  const handleCategoryChange = (catId: number | undefined) => {
    setSelectedCategoryId(catId);
    setSelectedSeriesId(undefined);
  };

  /**
   * 处理系列变更
   */
  const handleSeriesChange = (seriesId: number | undefined) => {
    setSelectedSeriesId(seriesId);
  };

  /**
   * 处理徽章选择变更
   */
  const handleBadgeChange: SelectProps<BadgeSelectValue>['onChange'] = (
    _selectedValue,
    selectedOption
  ) => {
    if (!selectedOption) {
      onChange?.(undefined);
      return;
    }
    const opt = selectedOption as BadgeSelectValue & { badge: BadgeListItem };
    onChange?.({
      value: opt.value,
      label: opt.badge.name,
      badge: opt.badge,
    });
  };

  /**
   * 处理搜索
   */
  const handleSearch = (keyword: string) => {
    setSearchKeyword(keyword);
  };

  /**
   * 渲染下拉头部（筛选区域）
   */
  const dropdownRender = (menu: React.ReactNode) => (
    <div>
      <div style={{ padding: '8px 12px' }}>
        <Space direction="vertical" style={{ width: '100%' }} size={8}>
          <Select
            placeholder="按分类筛选"
            style={{ width: '100%' }}
            allowClear
            options={categoryOptions}
            loading={categoriesLoading}
            value={selectedCategoryId}
            onChange={handleCategoryChange}
            size="small"
          />
          <Select
            placeholder="按系列筛选"
            style={{ width: '100%' }}
            allowClear
            options={seriesOptions}
            loading={seriesLoading}
            value={selectedSeriesId}
            onChange={handleSeriesChange}
            disabled={!selectedCategoryId && seriesOptions.length === 0}
            size="small"
          />
        </Space>
      </div>
      <Divider style={{ margin: '8px 0' }} />
      {badgesLoading ? (
        <div style={{ padding: 16, textAlign: 'center' }}>
          <Spin size="small" />
          <span style={{ marginLeft: 8 }}>加载中...</span>
        </div>
      ) : badgeOptions.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无可用徽章"
          style={{ padding: 24 }}
        />
      ) : (
        menu
      )}
    </div>
  );

  return (
    <Select<BadgeSelectValue>
      value={value}
      onChange={handleBadgeChange}
      placeholder={placeholder}
      disabled={disabled}
      style={{ width: '100%', ...style }}
      showSearch
      filterOption={false}
      onSearch={handleSearch}
      options={badgeOptions}
      labelInValue
      dropdownRender={dropdownRender}
      notFoundContent={null}
      allowClear
      optionLabelProp="label"
    />
  );
};

export default BadgeSelect;

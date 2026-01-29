/**
 * 用户选择组件
 *
 * 支持远程搜索用户，显示用户头像、昵称、会员等级
 * 用于手动发放页面选择发放目标用户
 */

import React, { useState, useMemo } from 'react';
import { Select, Avatar, Tag, Space, Spin, Empty, Typography } from 'antd';
import { UserOutlined } from '@ant-design/icons';
import { useSearchUsers } from '@/hooks/useGrant';
import { getMembershipConfig } from '@/types/user';
import type { User } from '@/types';
import type { SelectProps } from 'antd';

const { Text } = Typography;

/**
 * 用户选择值类型
 *
 * 使用 labelInValue 模式，值包含完整用户信息
 */
export interface UserSelectValue {
  value: string;
  label: React.ReactNode;
  user: User;
}

interface UserSelectProps {
  /** 选中的用户 */
  value?: UserSelectValue[];
  /** 值变更回调 */
  onChange?: (value: UserSelectValue[]) => void;
  /** 是否多选，默认 true */
  multiple?: boolean;
  /** 占位符 */
  placeholder?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 自定义样式 */
  style?: React.CSSProperties;
}

/**
 * 渲染用户选项
 */
const renderUserOption = (user: User) => {
  const memberConfig = getMembershipConfig(user.membershipLevel);

  return (
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
  );
};

/**
 * 渲染已选中的用户标签
 */
const renderSelectedTag = (props: {
  value: string;
  label: React.ReactNode;
  closable: boolean;
  onClose: () => void;
}) => {
  const { label, closable, onClose } = props;

  return (
    <Tag
      closable={closable}
      onClose={onClose}
      style={{ marginRight: 4, display: 'inline-flex', alignItems: 'center' }}
    >
      {label}
    </Tag>
  );
};

/**
 * 用户选择组件
 *
 * 使用 Select 组件实现远程搜索，支持多选
 */
const UserSelect: React.FC<UserSelectProps> = ({
  value = [],
  onChange,
  multiple = true,
  placeholder = '请输入用户 ID、手机号或昵称搜索',
  disabled = false,
  style,
}) => {
  const [searchKeyword, setSearchKeyword] = useState('');

  // 搜索用户
  const { data: searchResults, isLoading } = useSearchUsers(searchKeyword);

  // 构建选项列表，排除已选中的用户
  const options = useMemo(() => {
    if (!searchResults) return [];

    const selectedIds = new Set(value.map((v) => v.value));

    return searchResults
      .filter((user) => !selectedIds.has(user.userId))
      .map((user) => ({
        value: user.userId,
        label: renderUserOption(user),
        user,
      }));
  }, [searchResults, value]);

  /**
   * 处理搜索
   */
  const handleSearch = (keyword: string) => {
    setSearchKeyword(keyword);
  };

  /**
   * 处理选择变更
   */
  const handleChange: SelectProps<UserSelectValue[]>['onChange'] = (
    _selectedValues,
    selectedOptions
  ) => {
    // 由于使用 labelInValue，selectedOptions 包含完整信息
    const newValue = (selectedOptions as UserSelectValue[]).map((opt) => ({
      value: opt.value,
      label: opt.user?.username || opt.value,
      user: opt.user,
    }));
    onChange?.(newValue);
  };

  /**
   * 渲染下拉内容
   */
  const dropdownRender = (menu: React.ReactElement): React.ReactElement => {
    if (isLoading) {
      return (
        <div style={{ padding: 16, textAlign: 'center' }}>
          <Spin size="small" />
          <span style={{ marginLeft: 8 }}>搜索中...</span>
        </div>
      );
    }

    if (searchKeyword.length < 2) {
      return (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="请输入至少 2 个字符开始搜索"
          style={{ padding: 24 }}
        />
      );
    }

    if (searchResults && searchResults.length === 0) {
      return (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="未找到匹配的用户"
          style={{ padding: 24 }}
        />
      );
    }

    return menu;
  };

  return (
    <Select<UserSelectValue[]>
      mode={multiple ? 'multiple' : undefined}
      value={value}
      onChange={handleChange}
      placeholder={placeholder}
      disabled={disabled}
      style={{ width: '100%', ...style }}
      showSearch
      filterOption={false}
      onSearch={handleSearch}
      options={options}
      labelInValue
      tagRender={renderSelectedTag}
      dropdownRender={dropdownRender}
      notFoundContent={null}
      allowClear
    />
  );
};

export default UserSelect;

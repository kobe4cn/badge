/**
 * 用户头像下拉菜单组件
 *
 * 显示当前登录用户信息和操作菜单
 * 包含个人设置和退出登录功能
 */

import React from 'react';
import { Avatar, Dropdown, Space, App } from 'antd';
import { UserOutlined, SettingOutlined, LogoutOutlined } from '@ant-design/icons';
import type { MenuProps } from 'antd';

/**
 * 当前用户信息
 *
 * 实际项目中应从用户状态管理或 Context 获取
 */
interface CurrentUser {
  name: string;
  avatar?: string;
  role?: string;
}

interface AvatarDropdownProps {
  /** 当前用户信息 */
  currentUser?: CurrentUser;
  /** 退出登录回调 */
  onLogout?: () => void;
  /** 进入个人设置回调 */
  onSettings?: () => void;
}

const AvatarDropdown: React.FC<AvatarDropdownProps> = ({
  currentUser = { name: '管理员', role: 'Admin' },
  onLogout,
  onSettings,
}) => {
  const { message, modal } = App.useApp();

  // 处理退出登录确认
  const handleLogout = () => {
    modal.confirm({
      title: '退出登录',
      content: '确定要退出当前账号吗？',
      okText: '确定',
      cancelText: '取消',
      onOk: () => {
        if (onLogout) {
          onLogout();
        } else {
          message.success('已退出登录');
          // 实际项目中应跳转到登录页
        }
      },
    });
  };

  // 处理个人设置
  const handleSettings = () => {
    if (onSettings) {
      onSettings();
    } else {
      message.info('个人设置功能开发中');
    }
  };

  // 下拉菜单项配置
  const menuItems: MenuProps['items'] = [
    {
      key: 'user-info',
      label: (
        <div style={{ padding: '4px 0' }}>
          <div style={{ fontWeight: 500 }}>{currentUser.name}</div>
          {currentUser.role && (
            <div style={{ fontSize: 12, color: '#8c8c8c' }}>{currentUser.role}</div>
          )}
        </div>
      ),
      disabled: true,
    },
    {
      type: 'divider',
    },
    {
      key: 'settings',
      icon: <SettingOutlined />,
      label: '个人设置',
      onClick: handleSettings,
    },
    {
      type: 'divider',
    },
    {
      key: 'logout',
      icon: <LogoutOutlined />,
      label: '退出登录',
      onClick: handleLogout,
      danger: true,
    },
  ];

  return (
    <Dropdown
      menu={{ items: menuItems }}
      placement="bottomRight"
      trigger={['click']}
      overlayStyle={{ minWidth: 160 }}
    >
      <Space style={{ cursor: 'pointer', padding: '0 12px' }}>
        <Avatar
          size="small"
          src={currentUser.avatar}
          icon={!currentUser.avatar && <UserOutlined />}
          style={{ backgroundColor: '#1677ff' }}
        />
        <span style={{ color: 'rgba(255, 255, 255, 0.85)' }}>{currentUser.name}</span>
      </Space>
    </Dropdown>
  );
};

export default AvatarDropdown;

/**
 * 用户头像下拉菜单组件
 *
 * 显示当前登录用户信息和操作菜单
 * 包含个人设置和退出登录功能
 */

import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Avatar, Dropdown, Space, App } from 'antd';
import { UserOutlined, SettingOutlined, LogoutOutlined } from '@ant-design/icons';
import type { MenuProps } from 'antd';

import { useAuthStore } from '@/stores/authStore';
import { logout as logoutApi } from '@/services/auth';

/**
 * 头像下拉菜单组件
 *
 * 从 authStore 获取当前用户信息，支持退出登录操作
 */
const AvatarDropdown: React.FC = () => {
  const navigate = useNavigate();
  const { message } = App.useApp();
  const { user, clearAuth } = useAuthStore();

  // 获取用户显示信息
  const displayName = user?.displayName || user?.username || '管理员';
  const userRole = user?.roles?.includes('admin')
    ? '管理员'
    : user?.roles?.includes('operator')
      ? '操作员'
      : '访客';
  const avatarUrl = user?.avatar;

  // 处理退出登录
  const handleLogout = async () => {
    try {
      // 调用后端登出 API（可选，清除服务端 session）
      await logoutApi().catch(() => {
        // 忽略网络错误，仍然清除本地状态
      });
    } finally {
      // 清除本地认证状态
      clearAuth();
      message.success('已退出登录');
      // 跳转到登录页
      navigate('/login', { replace: true });
    }
  };

  // 处理个人设置
  const handleSettings = () => {
    message.info('个人设置功能开发中');
  };

  // 下拉菜单项配置
  const menuItems: MenuProps['items'] = [
    {
      key: 'user-info',
      label: (
        <div style={{ padding: '4px 0' }}>
          <div style={{ fontWeight: 500 }}>{displayName}</div>
          {userRole && <div style={{ fontSize: 12, color: '#8c8c8c' }}>{userRole}</div>}
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
      overlayClassName="user-dropdown"
    >
      <Space className="user-dropdown" style={{ cursor: 'pointer', padding: '0 12px' }}>
        <Avatar
          size="small"
          src={avatarUrl}
          icon={!avatarUrl && <UserOutlined />}
          style={{ backgroundColor: '#1677ff' }}
        />
        <span style={{ color: 'rgba(255, 255, 255, 0.85)' }}>{displayName}</span>
      </Space>
    </Dropdown>
  );
};

export default AvatarDropdown;

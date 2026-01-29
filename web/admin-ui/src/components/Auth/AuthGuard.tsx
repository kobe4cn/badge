/**
 * 路由权限控制组件
 *
 * 保护需要认证的路由，检查用户登录状态和权限：
 * - 未登录用户重定向到登录页
 * - 无权限用户显示 403 页面
 * - 支持开发模式下的 mock 认证跳过
 */

import React, { useCallback } from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { Result, Button } from 'antd';

import { isAuthenticated, hasPermission } from './authUtils';

/**
 * AuthGuard 组件属性
 */
interface AuthGuardProps {
  children: React.ReactNode;
}

/**
 * 403 无权限页面
 */
const ForbiddenPage: React.FC = () => {
  const handleGoBack = useCallback(() => {
    window.history.back();
  }, []);

  const handleGoHome = useCallback(() => {
    window.location.href = '/dashboard';
  }, []);

  return (
    <Result
      status="403"
      title="403"
      subTitle="抱歉，您没有权限访问此页面"
      extra={[
        <Button key="back" onClick={handleGoBack}>
          返回上页
        </Button>,
        <Button key="home" type="primary" onClick={handleGoHome}>
          返回首页
        </Button>,
      ]}
    />
  );
};

/**
 * 路由权限守卫组件
 *
 * 包裹需要保护的路由组件，自动处理认证和权限检查
 */
const AuthGuard: React.FC<AuthGuardProps> = ({ children }) => {
  const location = useLocation();

  // 检查登录状态
  if (!isAuthenticated()) {
    // 保存当前路径，登录后跳回
    const returnUrl = encodeURIComponent(location.pathname + location.search);
    return <Navigate to={`/login?returnUrl=${returnUrl}`} replace />;
  }

  // 检查权限
  if (!hasPermission(location.pathname)) {
    return <ForbiddenPage />;
  }

  return <>{children}</>;
};

export default AuthGuard;

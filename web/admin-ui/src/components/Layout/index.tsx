/**
 * 管理后台主布局组件
 *
 * 基于 @ant-design/pro-layout 实现侧边栏布局
 * 包含菜单导航、面包屑、用户头像等功能
 */

import React, { Suspense, useState, useMemo } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom';
import ProLayout from '@ant-design/pro-layout';
import type { ProLayoutProps, MenuDataItem } from '@ant-design/pro-layout';
import { TrophyOutlined } from '@ant-design/icons';

import { routes, flattenRoutes, NotFoundPage, type RouteConfig } from '@/config/routes';
import { PageLoading } from '@/components/Loading';
import ErrorBoundary from '@/components/ErrorBoundary';
import AvatarDropdown from './AvatarDropdown';
import PageTransition from './PageTransition';

/**
 * 将路由配置转换为 ProLayout 菜单数据格式
 */
function convertToMenuData(routeList: RouteConfig[]): MenuDataItem[] {
  return routeList.map((route) => ({
    path: route.path,
    name: route.name,
    icon: route.icon,
    hideInMenu: route.hideInMenu,
    children: route.children ? convertToMenuData(route.children) : undefined,
  }));
}


/**
 * 主布局组件
 *
 * 负责整体页面框架，包括侧边菜单、顶部栏、内容区域
 */
const AdminLayout: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(false);

  // 将路由配置转换为菜单数据
  const menuData = useMemo(() => convertToMenuData(routes), []);

  // 获取所有扁平化路由用于渲染
  const flatRoutes = useMemo(() => flattenRoutes(routes), []);

  // ProLayout 配置
  const layoutSettings: ProLayoutProps = {
    title: '徽章管理系统',
    logo: <TrophyOutlined style={{ fontSize: 28, color: '#1677ff' }} />,
    layout: 'mix',
    fixSiderbar: true,
    fixedHeader: true,
    collapsed,
    onCollapse: setCollapsed,
    siderWidth: 220,

    // 使用自定义头像渲染
    avatarProps: {
      render: () => <AvatarDropdown />,
    },

    // 菜单项点击跳转
    menuItemRender: (item, dom) => (
      <div onClick={() => item.path && navigate(item.path)}>{dom}</div>
    ),

    // 子菜单项渲染（子菜单不需要点击跳转，只需展开折叠）
    subMenuItemRender: (_item, dom) => (
      <div>{dom}</div>
    ),

    // 面包屑配置
    breadcrumbRender: (routers = []) => [
      {
        path: '/',
        breadcrumbName: '首页',
      },
      ...routers,
    ],
    itemRender: (route, _, routes) => {
      const isLast = routes.indexOf(route) === routes.length - 1;
      return isLast ? (
        <span>{route.breadcrumbName}</span>
      ) : (
        <a onClick={() => route.path && navigate(route.path)}>{route.breadcrumbName}</a>
      );
    },

    // 页脚配置
    footerRender: () => (
      <div style={{ textAlign: 'center', padding: '16px 0', color: '#8c8c8c' }}>
        徽章管理系统 ©2024
      </div>
    ),
  };

  return (
    <ProLayout
      {...layoutSettings}
      location={location}
      route={{ routes: menuData }}
      menuDataRender={() => menuData}
    >
      {/* 错误边界包裹页面内容 */}
      <ErrorBoundary>
        {/* 页面切换动画 */}
        <PageTransition>
          {/* Suspense 处理懒加载组件 */}
          <Suspense fallback={<PageLoading tip="页面加载中..." />}>
            <Routes>
              {/* 根路径重定向到数据看板 */}
              <Route path="/" element={<Navigate to="/dashboard" replace />} />

              {/* 动态渲染所有路由 */}
              {flatRoutes.map((route) => {
                const Component = route.component;
                return Component ? (
                  <Route key={route.path} path={route.path} element={<Component />} />
                ) : null;
              })}

              {/* 404 回退路由 */}
              <Route path="*" element={<NotFoundPage />} />
            </Routes>
          </Suspense>
        </PageTransition>
      </ErrorBoundary>
    </ProLayout>
  );
};

export default AdminLayout;

// 导出子组件供外部使用
export { AvatarDropdown, PageTransition };

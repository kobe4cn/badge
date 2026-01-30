/**
 * 路由配置
 *
 * 定义管理后台的路由结构和菜单层级
 * 使用 React.lazy 实现代码分割，提升首屏加载性能
 */

import React from 'react';
import {
  DashboardOutlined,
  TrophyOutlined,
  ApartmentOutlined,
  GiftOutlined,
  UserOutlined,
} from '@ant-design/icons';

import type { RouteConfig } from './routeConfig';

// 使用 React.lazy 按需加载页面组件，减少首屏体积
const DashboardPage = React.lazy(() => import('@/pages/dashboard'));
const CategoriesPage = React.lazy(() => import('@/pages/badges/Categories'));
const SeriesPage = React.lazy(() => import('@/pages/badges/Series'));
const DefinitionsPage = React.lazy(() => import('@/pages/badges/Definitions'));
const DependenciesPage = React.lazy(() => import('@/pages/badges/Dependencies'));
const CanvasPage = React.lazy(() => import('@/pages/rules/Canvas'));
const ManualGrantPage = React.lazy(() => import('@/pages/grants/Manual'));
const BatchGrantPage = React.lazy(() => import('@/pages/grants/Batch'));
const GrantLogsPage = React.lazy(() => import('@/pages/grants/Logs'));
const MemberSearchPage = React.lazy(() => import('@/pages/members/Search'));

/**
 * 404 页面组件
 */
export const NotFoundPage = React.lazy(() => import('@/pages/404'));

/**
 * 路由配置表
 *
 * 层级结构与业务模块对应，便于权限控制和菜单生成
 */
export const routes: RouteConfig[] = [
  {
    path: '/dashboard',
    name: '数据看板',
    icon: <DashboardOutlined />,
    component: DashboardPage,
  },
  {
    path: '/badges',
    name: '徽章管理',
    icon: <TrophyOutlined />,
    children: [
      {
        path: '/badges/categories',
        name: '分类管理',
        component: CategoriesPage,
      },
      {
        path: '/badges/series',
        name: '系列管理',
        component: SeriesPage,
      },
      {
        path: '/badges/definitions',
        name: '徽章定义',
        component: DefinitionsPage,
      },
      {
        path: '/badges/:badgeId/dependencies',
        name: '依赖配置',
        component: DependenciesPage,
        hideInMenu: true,
      },
    ],
  },
  {
    path: '/rules',
    name: '规则管理',
    icon: <ApartmentOutlined />,
    children: [
      {
        path: '/rules/canvas',
        name: '规则画布',
        component: CanvasPage,
      },
    ],
  },
  {
    path: '/grants',
    name: '发放管理',
    icon: <GiftOutlined />,
    children: [
      {
        path: '/grants/manual',
        name: '手动发放',
        component: ManualGrantPage,
      },
      {
        path: '/grants/batch',
        name: '批量任务',
        component: BatchGrantPage,
      },
      {
        path: '/grants/logs',
        name: '发放日志',
        component: GrantLogsPage,
      },
    ],
  },
  {
    path: '/members',
    name: '会员视图',
    icon: <UserOutlined />,
    children: [
      {
        path: '/members/search',
        name: '用户查询',
        component: MemberSearchPage,
      },
    ],
  },
];

// 重新导出辅助函数
export { findRouteByPath, getBreadcrumbRoutes, flattenRoutes } from './routeConfig';
export type { RouteConfig } from './routeConfig';

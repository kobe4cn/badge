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
  FileTextOutlined,
  ShoppingOutlined,
  SettingOutlined,
  NotificationOutlined,
  RollbackOutlined,
  FolderOutlined,
} from '@ant-design/icons';

import type { RouteConfig } from './routeConfig';

// 使用 React.lazy 按需加载页面组件，减少首屏体积
const LoginPage = React.lazy(() => import('@/pages/auth/Login'));
const DashboardPage = React.lazy(() => import('@/pages/dashboard'));
const CategoriesPage = React.lazy(() => import('@/pages/badges/Categories'));
const SeriesPage = React.lazy(() => import('@/pages/badges/Series'));
const DefinitionsPage = React.lazy(() => import('@/pages/badges/Definitions'));
const DependenciesPage = React.lazy(() => import('@/pages/badges/Dependencies'));
const CanvasPage = React.lazy(() => import('@/pages/rules/Canvas'));
const TemplatesPage = React.lazy(() => import('@/pages/rules/Templates'));
const ManualGrantPage = React.lazy(() => import('@/pages/grants/Manual'));
const BatchGrantPage = React.lazy(() => import('@/pages/grants/Batch'));
const GrantLogsPage = React.lazy(() => import('@/pages/grants/Logs'));
const MemberSearchPage = React.lazy(() => import('@/pages/members/Search'));
const BenefitsListPage = React.lazy(() => import('@/pages/benefits/List'));
const BenefitGrantsPage = React.lazy(() => import('@/pages/benefits/Grants'));
const AutoBenefitsPage = React.lazy(() => import('@/pages/benefits/Auto'));
const RedemptionRulesPage = React.lazy(() => import('@/pages/redemptions/Rules'));
const RedemptionRecordsPage = React.lazy(() => import('@/pages/redemptions/Records'));
const ManualRedemptionPage = React.lazy(() => import('@/pages/redemptions/Manual'));
const SystemUsersPage = React.lazy(() => import('@/pages/system/Users'));
const SystemRolesPage = React.lazy(() => import('@/pages/system/Roles'));
const SystemApiKeysPage = React.lazy(() => import('@/pages/system/ApiKeys'));
const NotificationConfigsPage = React.lazy(() => import('@/pages/notifications/Configs'));
const NotificationTasksPage = React.lazy(() => import('@/pages/notifications/Tasks'));
const BatchRevokePage = React.lazy(() => import('@/pages/revokes/Batch'));
const RevokeLogsPage = React.lazy(() => import('@/pages/revokes/Logs'));
const AssetLibraryPage = React.lazy(() => import('@/pages/assets/Library'));

/**
 * 404 页面组件
 */
export const NotFoundPage = React.lazy(() => import('@/pages/404'));

/**
 * 登录页面组件（独立导出，不需要 Layout 包裹）
 */
export { LoginPage };

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
      {
        path: '/rules/templates',
        name: '规则模板',
        component: TemplatesPage,
      },
      {
        path: '/rules/create',
        name: '创建规则',
        component: CanvasPage,
        hideInMenu: true,
      },
      {
        path: '/rules/:ruleId/edit',
        name: '编辑规则',
        component: CanvasPage,
        hideInMenu: true,
      },
    ],
  },
  {
    path: '/benefits',
    name: '权益管理',
    icon: <FileTextOutlined />,
    children: [
      {
        path: '/benefits',
        name: '权益列表',
        component: BenefitsListPage,
        hideInMenu: true,
      },
      {
        path: '/benefits/list',
        name: '权益列表',
        component: BenefitsListPage,
      },
      {
        path: '/benefits/grants',
        name: '发放记录',
        component: BenefitGrantsPage,
      },
      {
        path: '/benefits/auto',
        name: '自动权益',
        component: AutoBenefitsPage,
      },
    ],
  },
  {
    path: '/redemptions',
    name: '兑换管理',
    icon: <ShoppingOutlined />,
    children: [
      {
        path: '/redemptions/rules',
        name: '兑换规则',
        component: RedemptionRulesPage,
      },
      {
        path: '/redemptions/manual',
        name: '手动兑换',
        component: ManualRedemptionPage,
      },
      {
        path: '/redemptions/records',
        name: '兑换记录',
        component: RedemptionRecordsPage,
      },
    ],
  },
  {
    path: '/notifications',
    name: '通知管理',
    icon: <NotificationOutlined />,
    children: [
      {
        path: '/notifications/configs',
        name: '通知配置',
        component: NotificationConfigsPage,
      },
      {
        path: '/notifications/tasks',
        name: '发送记录',
        component: NotificationTasksPage,
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
    path: '/revokes',
    name: '撤销管理',
    icon: <RollbackOutlined />,
    children: [
      {
        path: '/revokes/batch',
        name: '批量撤销',
        component: BatchRevokePage,
      },
      {
        path: '/revokes/logs',
        name: '撤销记录',
        component: RevokeLogsPage,
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
  {
    path: '/assets',
    name: '素材库',
    icon: <FolderOutlined />,
    component: AssetLibraryPage,
  },
  {
    path: '/system',
    name: '系统管理',
    icon: <SettingOutlined />,
    children: [
      {
        path: '/system/users',
        name: '用户管理',
        component: SystemUsersPage,
      },
      {
        path: '/system/roles',
        name: '角色管理',
        component: SystemRolesPage,
      },
      {
        path: '/system/api-keys',
        name: 'API Key 管理',
        component: SystemApiKeysPage,
      },
    ],
  },
];

/**
 * 重定向路由配置
 *
 * 用于兼容旧 URL 或提供简短 URL
 */
export const redirectRoutes: Array<{ from: string; to: string }> = [
  { from: '/categories', to: '/badges/categories' },
  { from: '/series', to: '/badges/series' },
  { from: '/badges', to: '/badges/definitions' },
  { from: '/templates', to: '/rules/templates' },
];

// 重新导出辅助函数
export { findRouteByPath, getBreadcrumbRoutes, flattenRoutes } from './routeConfig';
export type { RouteConfig } from './routeConfig';

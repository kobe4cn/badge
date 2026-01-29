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

/**
 * 路由配置项
 *
 * 同时用于 React Router 路由定义和 ProLayout 菜单渲染
 */
export interface RouteConfig {
  /** 路由路径 */
  path: string;
  /** 菜单名称 */
  name: string;
  /** 菜单图标 */
  icon?: React.ReactNode;
  /** 页面组件（懒加载） */
  component?: React.LazyExoticComponent<React.ComponentType>;
  /** 子路由 */
  children?: RouteConfig[];
  /** 是否在菜单中隐藏 */
  hideInMenu?: boolean;
  /** 重定向目标 */
  redirect?: string;
}

// 使用 React.lazy 按需加载页面组件，减少首屏体积
const DashboardPage = React.lazy(() => import('@/pages/dashboard'));
const CategoriesPage = React.lazy(() => import('@/pages/badges/Categories'));
const SeriesPage = React.lazy(() => import('@/pages/badges/Series'));
const DefinitionsPage = React.lazy(() => import('@/pages/badges/Definitions'));
const CanvasPage = React.lazy(() => import('@/pages/rules/Canvas'));
const ManualGrantPage = React.lazy(() => import('@/pages/grants/Manual'));
const BatchGrantPage = React.lazy(() => import('@/pages/grants/Batch'));
const GrantLogsPage = React.lazy(() => import('@/pages/grants/Logs'));
const MemberSearchPage = React.lazy(() => import('@/pages/members/Search'));

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

/**
 * 根据路径获取路由配置
 *
 * 支持嵌套路由查找，用于面包屑导航等场景
 */
export function findRouteByPath(
  path: string,
  routeList: RouteConfig[] = routes
): RouteConfig | undefined {
  for (const route of routeList) {
    if (route.path === path) {
      return route;
    }
    if (route.children) {
      const found = findRouteByPath(path, route.children);
      if (found) return found;
    }
  }
  return undefined;
}

/**
 * 获取路由的面包屑路径
 *
 * 返回从根到当前路由的完整路径数组
 */
export function getBreadcrumbRoutes(
  path: string,
  routeList: RouteConfig[] = routes,
  parents: RouteConfig[] = []
): RouteConfig[] {
  for (const route of routeList) {
    if (route.path === path) {
      return [...parents, route];
    }
    if (route.children) {
      const found = getBreadcrumbRoutes(path, route.children, [...parents, route]);
      if (found.length > 0) return found;
    }
  }
  return [];
}

/**
 * 扁平化路由配置
 *
 * 将嵌套路由结构展开为一维数组，用于路由注册
 */
export function flattenRoutes(routeList: RouteConfig[] = routes): RouteConfig[] {
  const result: RouteConfig[] = [];

  function flatten(items: RouteConfig[]) {
    for (const item of items) {
      if (item.component) {
        result.push(item);
      }
      if (item.children) {
        flatten(item.children);
      }
    }
  }

  flatten(routeList);
  return result;
}

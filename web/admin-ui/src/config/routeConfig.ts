/**
 * 路由配置类型和辅助函数
 *
 * 与路由组件定义分离，避免 fast refresh 警告
 */

import type React from 'react';

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

/**
 * 根据路径获取路由配置
 *
 * 支持嵌套路由查找，用于面包屑导航等场景
 */
export function findRouteByPath(
  path: string,
  routeList: RouteConfig[]
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
  routeList: RouteConfig[],
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
export function flattenRoutes(routeList: RouteConfig[]): RouteConfig[] {
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

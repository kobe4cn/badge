/**
 * 配置模块统一导出
 */

export { env, default as envConfig } from './env';
export { routes, flattenRoutes, findRouteByPath, getBreadcrumbRoutes, type RouteConfig } from './routes';

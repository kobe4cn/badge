/**
 * 应用根组件
 *
 * 配置全局 Provider 和路由容器
 * 使用 ProLayout 实现管理后台布局
 * 集成认证状态检查和路由守卫
 */

import React, { Suspense, useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App as AntdApp } from 'antd';

import AdminLayout from '@/components/Layout';
import { PageLoading } from '@/components/Loading';
import { useAuthStore } from '@/stores/authStore';
import { LoginPage } from '@/config/routes';

/**
 * React Query 客户端配置
 *
 * staleTime: 数据过期时间，5分钟内不会重新请求
 * retry: 失败重试次数
 */
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      retry: 1,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 0,
    },
  },
});

/**
 * 认证路由守卫
 *
 * 检查用户登录状态，未登录时重定向到登录页
 */
const AuthenticatedRoute: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const { isAuthenticated } = useAuthStore();

  if (!isAuthenticated) {
    // 保存当前路径用于登录后跳转
    const returnUrl = encodeURIComponent(location.pathname + location.search);
    return <Navigate to={`/login?returnUrl=${returnUrl}`} replace />;
  }

  return <>{children}</>;
};

/**
 * 应用路由组件
 *
 * 配置登录页和主布局的路由结构
 */
const AppRoutes: React.FC = () => {
  const { restoreAuth } = useAuthStore();

  // 应用启动时恢复认证状态
  useEffect(() => {
    restoreAuth();
  }, [restoreAuth]);

  return (
    <Routes>
      {/* 登录页面（无需认证） */}
      <Route
        path="/login"
        element={
          <Suspense fallback={<PageLoading tip="加载中..." />}>
            <LoginPage />
          </Suspense>
        }
      />

      {/* 需要认证的路由 */}
      <Route
        path="/*"
        element={
          <AuthenticatedRoute>
            <AdminLayout />
          </AuthenticatedRoute>
        }
      />
    </Routes>
  );
};

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      {/* AntdApp 组件提供 message/notification/modal 的静态方法访问 */}
      <AntdApp>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </AntdApp>
    </QueryClientProvider>
  );
}

export default App;

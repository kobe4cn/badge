/**
 * 应用根组件
 *
 * 配置全局 Provider 和路由容器
 * 使用 ProLayout 实现管理后台布局
 */

import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App as AntdApp } from 'antd';

import AdminLayout from '@/components/Layout';

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

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      {/* AntdApp 组件提供 message/notification/modal 的静态方法访问 */}
      <AntdApp>
        <BrowserRouter>
          <AdminLayout />
        </BrowserRouter>
      </AntdApp>
    </QueryClientProvider>
  );
}

export default App;

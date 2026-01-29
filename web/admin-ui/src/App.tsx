/**
 * 应用根组件
 *
 * 配置全局 Provider 和路由容器
 * 路由配置将在 Task 8.2 中完善
 */

import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App as AntdApp } from 'antd';

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
          {/* 路由配置将在 Task 8.2 中实现 */}
          <div style={{ padding: 24, minHeight: '100vh', background: '#f0f2f5' }}>
            <h1>徽章管理系统</h1>
            <p>前端基础结构已初始化，布局与路由将在后续任务中实现。</p>
            <p style={{ color: '#666', marginTop: 16 }}>
              已完成配置：
            </p>
            <ul style={{ color: '#666' }}>
              <li>目录结构规划</li>
              <li>TypeScript 类型定义</li>
              <li>Axios API 客户端</li>
              <li>Ant Design 主题配置</li>
              <li>环境变量配置</li>
              <li>React Query 数据管理</li>
            </ul>
          </div>
        </BrowserRouter>
      </AntdApp>
    </QueryClientProvider>
  );
}

export default App;

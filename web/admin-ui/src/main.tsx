/**
 * 应用入口文件
 *
 * 配置 React 根节点渲染和全局 Provider
 */

import React from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import dayjs from 'dayjs';
import 'dayjs/locale/zh-cn';

import App from './App';
import { themeConfig } from './theme';
import { env } from './config';

// 配置 dayjs 中文语言包
dayjs.locale('zh-cn');

// 设置页面标题
document.title = env.appTitle;

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ConfigProvider locale={zhCN} theme={themeConfig}>
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);

/**
 * 全局页面加载组件
 *
 * 用于路由懒加载的 fallback 和页面级加载状态展示
 * 支持全屏模式和自定义提示文字
 */

import React from 'react';
import { Spin } from 'antd';

/**
 * PageLoading 组件属性
 */
interface PageLoadingProps {
  /** 加载提示文字 */
  tip?: string;
  /** 是否全屏显示 */
  fullScreen?: boolean;
  /** 自定义样式 */
  style?: React.CSSProperties;
}

/**
 * 页面加载组件
 *
 * 提供统一的加载状态视觉反馈，用于：
 * - 路由懒加载时的 Suspense fallback
 * - 页面数据初始加载
 * - 全屏遮罩加载
 */
const PageLoading: React.FC<PageLoadingProps> = ({
  tip = '加载中...',
  fullScreen = false,
  style,
}) => {
  const containerStyle: React.CSSProperties = fullScreen
    ? {
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: 'rgba(255, 255, 255, 0.85)',
        zIndex: 9999,
        ...style,
      }
    : {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        minHeight: 400,
        padding: 48,
        ...style,
      };

  return (
    <div style={containerStyle}>
      <Spin size="large" tip={tip}>
        {/* Spin 需要子元素才能显示 tip */}
        <div style={{ width: 200, height: 100 }} />
      </Spin>
    </div>
  );
};

export default PageLoading;

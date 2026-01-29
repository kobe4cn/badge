/**
 * 全局错误边界组件
 *
 * 捕获 React 组件树中的 JavaScript 错误，防止整个应用崩溃
 * 显示友好的错误页面并提供重试选项
 */

import React, { Component, ErrorInfo } from 'react';
import { Result, Button, Typography, Space, Card } from 'antd';
import { ReloadOutlined, HomeOutlined, BugOutlined } from '@ant-design/icons';
import { env } from '@/config';

const { Text, Paragraph } = Typography;

/**
 * 错误边界组件属性
 */
interface ErrorBoundaryProps {
  children: React.ReactNode;
  /** 自定义错误回退 UI */
  fallback?: React.ReactNode;
  /** 错误发生时的回调 */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

/**
 * 错误边界组件状态
 */
interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

/**
 * 全局错误边界
 *
 * 在组件树顶层捕获错误，避免白屏：
 * - 生产环境显示友好的错误提示
 * - 开发环境显示详细的错误堆栈
 * - 提供重试和返回首页操作
 */
class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    this.setState({ errorInfo });

    // 调用外部错误回调（可用于上报错误）
    this.props.onError?.(error, errorInfo);

    // 开发模式下打印错误信息
    if (env.isDev) {
      console.error('ErrorBoundary caught an error:', error);
      console.error('Component stack:', errorInfo.componentStack);
    }

    // 生产环境可以在这里上报错误到监控服务
    // 例如: reportErrorToService(error, errorInfo);
  }

  /**
   * 重置错误状态并重试
   */
  handleRetry = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  /**
   * 返回首页
   */
  handleGoHome = (): void => {
    window.location.href = '/dashboard';
  };

  /**
   * 刷新页面
   */
  handleRefresh = (): void => {
    window.location.reload();
  };

  render(): React.ReactNode {
    const { hasError, error, errorInfo } = this.state;
    const { children, fallback } = this.props;

    if (hasError) {
      // 使用自定义 fallback
      if (fallback) {
        return fallback;
      }

      // 默认错误 UI
      return (
        <div
          style={{
            minHeight: '100vh',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 24,
            backgroundColor: '#f5f5f5',
          }}
        >
          <Result
            status="error"
            title="页面出错了"
            subTitle="抱歉，页面发生了错误。请尝试刷新页面或返回首页。"
            extra={
              <Space>
                <Button icon={<ReloadOutlined />} onClick={this.handleRefresh}>
                  刷新页面
                </Button>
                <Button type="primary" icon={<HomeOutlined />} onClick={this.handleGoHome}>
                  返回首页
                </Button>
              </Space>
            }
          >
            {/* 开发环境显示错误详情 */}
            {env.isDev && error && (
              <Card
                size="small"
                title={
                  <Space>
                    <BugOutlined />
                    <span>错误详情（仅开发环境可见）</span>
                  </Space>
                }
                style={{ marginTop: 24, textAlign: 'left' }}
              >
                <Paragraph>
                  <Text strong>错误信息：</Text>
                  <br />
                  <Text type="danger">{error.message}</Text>
                </Paragraph>

                {error.stack && (
                  <Paragraph>
                    <Text strong>错误堆栈：</Text>
                    <pre
                      style={{
                        background: '#f5f5f5',
                        padding: 12,
                        borderRadius: 4,
                        overflow: 'auto',
                        fontSize: 12,
                        maxHeight: 200,
                      }}
                    >
                      {error.stack}
                    </pre>
                  </Paragraph>
                )}

                {errorInfo?.componentStack && (
                  <Paragraph>
                    <Text strong>组件堆栈：</Text>
                    <pre
                      style={{
                        background: '#f5f5f5',
                        padding: 12,
                        borderRadius: 4,
                        overflow: 'auto',
                        fontSize: 12,
                        maxHeight: 200,
                      }}
                    >
                      {errorInfo.componentStack}
                    </pre>
                  </Paragraph>
                )}
              </Card>
            )}
          </Result>
        </div>
      );
    }

    return children;
  }
}

export default ErrorBoundary;

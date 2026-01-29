/**
 * 404 页面
 *
 * 当访问不存在的路由时显示的友好提示页面
 */

import React, { useCallback } from 'react';
import { Result, Button, Space } from 'antd';
import { HomeOutlined, ArrowLeftOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';

/**
 * 404 未找到页面
 */
const NotFoundPage: React.FC = () => {
  const navigate = useNavigate();

  const handleGoBack = useCallback(() => {
    navigate(-1);
  }, [navigate]);

  const handleGoHome = useCallback(() => {
    navigate('/dashboard');
  }, [navigate]);

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        backgroundColor: '#f5f5f5',
      }}
    >
      <Result
        status="404"
        title="404"
        subTitle="抱歉，您访问的页面不存在"
        extra={
          <Space>
            <Button icon={<ArrowLeftOutlined />} onClick={handleGoBack}>
              返回上页
            </Button>
            <Button type="primary" icon={<HomeOutlined />} onClick={handleGoHome}>
              返回首页
            </Button>
          </Space>
        }
      />
    </div>
  );
};

export default NotFoundPage;

/**
 * 手动发放页面
 *
 * 支持管理员手动为用户发放徽章
 * 具体内容将在 Task 8.9 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { GiftOutlined, SearchOutlined } from '@ant-design/icons';

const ManualGrantPage: React.FC = () => {
  return (
    <PageContainer
      title="手动发放"
      extra={
        <Space>
          <Button icon={<SearchOutlined />} disabled>
            查找用户
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <GiftOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>手动发放页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.9 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default ManualGrantPage;

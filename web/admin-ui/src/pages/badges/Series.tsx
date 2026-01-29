/**
 * 徽章系列管理页面
 *
 * 管理徽章系列，系列是同一主题下的徽章集合
 * 具体内容将在 Task 8.4 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { PlusOutlined, AppstoreOutlined } from '@ant-design/icons';

const SeriesPage: React.FC = () => {
  return (
    <PageContainer
      title="系列管理"
      extra={
        <Space>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            新建系列
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <AppstoreOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>系列管理页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.4 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default SeriesPage;

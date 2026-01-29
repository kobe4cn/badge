/**
 * 批量任务页面
 *
 * 管理批量徽章发放任务，支持创建、查看、终止批量任务
 * 具体内容将在 Task 8.10 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { PlusOutlined, UnorderedListOutlined } from '@ant-design/icons';

const BatchGrantPage: React.FC = () => {
  return (
    <PageContainer
      title="批量任务"
      extra={
        <Space>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            新建任务
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <UnorderedListOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>批量任务页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.10 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default BatchGrantPage;

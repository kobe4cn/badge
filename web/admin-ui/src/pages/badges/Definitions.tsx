/**
 * 徽章定义管理页面
 *
 * 管理徽章的定义，包括名称、图标、描述、获取条件等
 * 具体内容将在 Task 8.5 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { PlusOutlined, TrophyOutlined } from '@ant-design/icons';

const DefinitionsPage: React.FC = () => {
  return (
    <PageContainer
      title="徽章定义"
      extra={
        <Space>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            新建徽章
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <TrophyOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>徽章定义页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.5 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default DefinitionsPage;

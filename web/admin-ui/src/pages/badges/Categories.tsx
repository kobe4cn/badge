/**
 * 徽章分类管理页面
 *
 * 管理徽章的分类体系，支持增删改查操作
 * 具体内容将在 Task 8.3 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { PlusOutlined, FolderOutlined } from '@ant-design/icons';

const CategoriesPage: React.FC = () => {
  return (
    <PageContainer
      title="分类管理"
      extra={
        <Space>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            新建分类
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <FolderOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>分类管理页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.3 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default CategoriesPage;

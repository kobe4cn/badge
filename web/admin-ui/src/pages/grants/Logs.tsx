/**
 * 发放日志页面
 *
 * 查看徽章发放的历史记录，支持筛选和导出
 * 具体内容将在 Task 8.11 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { ExportOutlined, FileTextOutlined } from '@ant-design/icons';

const GrantLogsPage: React.FC = () => {
  return (
    <PageContainer
      title="发放日志"
      extra={
        <Space>
          <Button icon={<ExportOutlined />} disabled>
            导出日志
          </Button>
        </Space>
      }
    >
      <Card>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <FileTextOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>发放日志页面开发中...</p>
          <p style={{ fontSize: 12 }}>将在 Task 8.11 中实现完整功能</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default GrantLogsPage;

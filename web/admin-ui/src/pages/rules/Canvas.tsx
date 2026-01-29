/**
 * 规则画布页面
 *
 * 可视化规则编辑器，使用节点连线方式定义徽章发放规则
 * 具体内容将在 Task 8.6/8.7/8.8 中实现
 */

import React from 'react';
import { Card, Button, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { PlusOutlined, PlayCircleOutlined, ApartmentOutlined } from '@ant-design/icons';

const CanvasPage: React.FC = () => {
  return (
    <PageContainer
      title="规则画布"
      extra={
        <Space>
          <Button icon={<PlayCircleOutlined />} disabled>
            测试规则
          </Button>
          <Button type="primary" icon={<PlusOutlined />} disabled>
            新建规则
          </Button>
        </Space>
      }
    >
      <Card bodyStyle={{ padding: 0, height: 'calc(100vh - 260px)', minHeight: 500 }}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: '100%',
            color: '#8c8c8c',
          }}
        >
          <div style={{ textAlign: 'center' }}>
            <ApartmentOutlined style={{ fontSize: 64, marginBottom: 16 }} />
            <p>规则画布开发中...</p>
            <p style={{ fontSize: 12 }}>
              基础节点组件将在 Task 8.6 实现
              <br />
              连接与交互将在 Task 8.7 实现
              <br />
              规则测试与预览将在 Task 8.8 实现
            </p>
          </div>
        </div>
      </Card>
    </PageContainer>
  );
};

export default CanvasPage;

/**
 * 会员查询页面
 *
 * 查询用户信息及其持有的徽章
 * 具体内容将在 Task 8.14 中实现
 */

import React from 'react';
import { Card, Input, Space } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { UserOutlined, SearchOutlined } from '@ant-design/icons';

const { Search } = Input;

const MemberSearchPage: React.FC = () => {
  return (
    <PageContainer title="用户查询">
      <Card>
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <Search
            placeholder="输入用户ID、用户名或邮箱搜索"
            enterButton={<><SearchOutlined /> 搜索</>}
            size="large"
            style={{ maxWidth: 600 }}
            disabled
          />

          <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
            <UserOutlined style={{ fontSize: 48, marginBottom: 16 }} />
            <p>用户查询页面开发中...</p>
            <p style={{ fontSize: 12 }}>将在 Task 8.14 中实现完整功能</p>
          </div>
        </Space>
      </Card>
    </PageContainer>
  );
};

export default MemberSearchPage;

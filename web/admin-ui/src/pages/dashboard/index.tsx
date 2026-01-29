/**
 * 数据看板页面
 *
 * 展示徽章系统的整体运营数据和趋势图表
 * 具体内容将在 Task 8.12/8.13 中实现
 */

import React from 'react';
import { Card, Row, Col } from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  TrophyOutlined,
  UserOutlined,
  GiftOutlined,
  RiseOutlined,
} from '@ant-design/icons';

const DashboardPage: React.FC = () => {
  return (
    <PageContainer title="数据看板">
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <TrophyOutlined style={{ fontSize: 32, color: '#1677ff' }} />
              <div>
                <div style={{ color: '#8c8c8c', fontSize: 14 }}>徽章总数</div>
                <div style={{ fontSize: 24, fontWeight: 600 }}>--</div>
              </div>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <UserOutlined style={{ fontSize: 32, color: '#52c41a' }} />
              <div>
                <div style={{ color: '#8c8c8c', fontSize: 14 }}>持有用户数</div>
                <div style={{ fontSize: 24, fontWeight: 600 }}>--</div>
              </div>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <GiftOutlined style={{ fontSize: 32, color: '#faad14' }} />
              <div>
                <div style={{ color: '#8c8c8c', fontSize: 14 }}>今日发放</div>
                <div style={{ fontSize: 24, fontWeight: 600 }}>--</div>
              </div>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <RiseOutlined style={{ fontSize: 32, color: '#ff4d4f' }} />
              <div>
                <div style={{ color: '#8c8c8c', fontSize: 14 }}>规则触发</div>
                <div style={{ fontSize: 24, fontWeight: 600 }}>--</div>
              </div>
            </div>
          </Card>
        </Col>
      </Row>

      <Card style={{ marginTop: 16 }}>
        <div style={{ textAlign: 'center', padding: 48, color: '#8c8c8c' }}>
          <RiseOutlined style={{ fontSize: 48, marginBottom: 16 }} />
          <p>趋势图表开发中...</p>
          <p style={{ fontSize: 12 }}>统计概览将在 Task 8.12 实现，趋势图表将在 Task 8.13 实现</p>
        </div>
      </Card>
    </PageContainer>
  );
};

export default DashboardPage;

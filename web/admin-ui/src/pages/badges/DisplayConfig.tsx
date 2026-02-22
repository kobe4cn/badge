/**
 * 徽章展示配置页面
 *
 * 配置徽章在 C 端（用户前台）的展示方式，包括：
 * - 各状态下的展示样式（已获得/未获得/已过期）
 * - 动效配置
 * - 排列方式
 *
 * 对应需求 §5（徽章前端展示配置）
 */

import React, { useState } from 'react';
import {
  Card,
  Form,
  Select,
  Switch,
  InputNumber,
  Button,
  Divider,
  Radio,
  message,
  Row,
  Col,
  Alert,
} from 'antd';
import {
  PageContainer,
  ProFormSelect,
} from '@ant-design/pro-components';
import { SaveOutlined } from '@ant-design/icons';

/**
 * 展示布局类型
 */
type DisplayLayout = 'grid' | 'list' | 'carousel';

/**
 * 徽章尺寸
 */
type BadgeSize = 'small' | 'medium' | 'large';

/**
 * 展示配置表单数据
 */
interface DisplayConfigFormValues {
  layout: DisplayLayout;
  badgeSize: BadgeSize;
  columns: number;
  showUnearned: boolean;
  unearnedStyle: 'grayscale' | 'silhouette' | 'hidden';
  showProgress: boolean;
  showExpireCountdown: boolean;
  expireWarningDays: number;
  enableAnimation: boolean;
  animationTrigger: 'hover' | 'auto' | 'click';
  showBadgeCount: boolean;
  showRarityLabel: boolean;
  groupByCategory: boolean;
}

/**
 * 默认配置
 *
 * 提供合理的初始值，运营可以基于此进行调整
 */
const DEFAULT_CONFIG: DisplayConfigFormValues = {
  layout: 'grid',
  badgeSize: 'medium',
  columns: 4,
  showUnearned: true,
  unearnedStyle: 'grayscale',
  showProgress: true,
  showExpireCountdown: true,
  expireWarningDays: 7,
  enableAnimation: true,
  animationTrigger: 'hover',
  showBadgeCount: true,
  showRarityLabel: true,
  groupByCategory: true,
};

const DisplayConfigPage: React.FC = () => {
  const [form] = Form.useForm<DisplayConfigFormValues>();
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      // 展示配置暂存前端，后续接入后端 API 持久化
      console.log('展示配置:', values);
      await new Promise((r) => setTimeout(r, 500));
      message.success('展示配置已保存');
    } catch {
      // 表单校验失败
    } finally {
      setSaving(false);
    }
  };

  return (
    <PageContainer
      title="徽章展示配置"
      extra={
        <Button
          type="primary"
          icon={<SaveOutlined />}
          loading={saving}
          onClick={handleSave}
        >
          保存配置
        </Button>
      }
    >
      <Alert
        message="此页面配置徽章在用户端（C 端）的展示方式，保存后将影响前端展示效果"
        type="info"
        showIcon
        style={{ marginBottom: 24 }}
      />

      <Form
        form={form}
        layout="vertical"
        initialValues={DEFAULT_CONFIG}
      >
        {/* 布局配置 */}
        <Card title="布局配置" style={{ marginBottom: 24 }}>
          <Row gutter={24}>
            <Col span={8}>
              <Form.Item name="layout" label="展示布局">
                <Radio.Group>
                  <Radio.Button value="grid">宫格</Radio.Button>
                  <Radio.Button value="list">列表</Radio.Button>
                  <Radio.Button value="carousel">轮播</Radio.Button>
                </Radio.Group>
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="badgeSize" label="徽章尺寸">
                <Select
                  options={[
                    { value: 'small', label: '小（48px）' },
                    { value: 'medium', label: '中（72px）' },
                    { value: 'large', label: '大（96px）' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="columns" label="每行列数">
                <InputNumber min={2} max={8} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item
            name="groupByCategory"
            label="按分类分组"
            valuePropName="checked"
          >
            <Switch checkedChildren="开启" unCheckedChildren="关闭" />
          </Form.Item>
        </Card>

        {/* 未获得徽章展示 */}
        <Card title="未获得徽章展示" style={{ marginBottom: 24 }}>
          <Row gutter={24}>
            <Col span={8}>
              <Form.Item
                name="showUnearned"
                label="显示未获得徽章"
                valuePropName="checked"
              >
                <Switch checkedChildren="显示" unCheckedChildren="隐藏" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item
                noStyle
                shouldUpdate={(prev, cur) => prev.showUnearned !== cur.showUnearned}
              >
                {({ getFieldValue }) => (
                  <Form.Item
                    name="unearnedStyle"
                    label="未获得展示样式"
                  >
                    <ProFormSelect
                      disabled={!getFieldValue('showUnearned')}
                      options={[
                        { value: 'grayscale', label: '灰度' },
                        { value: 'silhouette', label: '剪影' },
                        { value: 'hidden', label: '问号占位' },
                      ]}
                    />
                  </Form.Item>
                )}
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item
                name="showProgress"
                label="显示获取进度"
                valuePropName="checked"
                tooltip="在未获得的徽章上显示获取条件的完成进度"
              >
                <Switch checkedChildren="显示" unCheckedChildren="隐藏" />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* 有效期展示 */}
        <Card title="有效期展示" style={{ marginBottom: 24 }}>
          <Row gutter={24}>
            <Col span={12}>
              <Form.Item
                name="showExpireCountdown"
                label="显示过期倒计时"
                valuePropName="checked"
                tooltip="在即将过期的徽章上显示剩余天数"
              >
                <Switch checkedChildren="显示" unCheckedChildren="隐藏" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name="expireWarningDays"
                label="过期预警天数"
                tooltip="剩余天数低于此值时显示预警样式"
              >
                <InputNumber min={1} max={90} addonAfter="天" style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* 动效和标签 */}
        <Card title="动效与标签" style={{ marginBottom: 24 }}>
          <Row gutter={24}>
            <Col span={8}>
              <Form.Item
                name="enableAnimation"
                label="启用动效"
                valuePropName="checked"
              >
                <Switch checkedChildren="开启" unCheckedChildren="关闭" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item
                noStyle
                shouldUpdate={(prev, cur) => prev.enableAnimation !== cur.enableAnimation}
              >
                {({ getFieldValue }) => (
                  <Form.Item name="animationTrigger" label="动效触发方式">
                    <Select
                      disabled={!getFieldValue('enableAnimation')}
                      options={[
                        { value: 'hover', label: '鼠标悬停' },
                        { value: 'auto', label: '自动播放' },
                        { value: 'click', label: '点击触发' },
                      ]}
                    />
                  </Form.Item>
                )}
              </Form.Item>
            </Col>
          </Row>

          <Divider />

          <Row gutter={24}>
            <Col span={8}>
              <Form.Item
                name="showBadgeCount"
                label="显示持有数量"
                valuePropName="checked"
                tooltip="当用户持有多个相同徽章时显示数量角标"
              >
                <Switch checkedChildren="显示" unCheckedChildren="隐藏" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item
                name="showRarityLabel"
                label="显示稀有度标签"
                valuePropName="checked"
                tooltip="在限定徽章和成就徽章上显示稀有度标签"
              >
                <Switch checkedChildren="显示" unCheckedChildren="隐藏" />
              </Form.Item>
            </Col>
          </Row>
        </Card>
      </Form>
    </PageContainer>
  );
};

export default DisplayConfigPage;

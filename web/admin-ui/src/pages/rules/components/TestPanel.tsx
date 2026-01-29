/**
 * 规则测试面板组件
 *
 * 提供测试数据输入和预置场景选择功能，
 * 以侧边抽屉形式展示，支持 JSON 编辑和表单输入两种模式
 */

import React, { useState, useMemo } from 'react';
import {
  Drawer,
  Form,
  Input,
  Select,
  Button,
  Space,
  Tabs,
  Card,
  Typography,
  Divider,
  DatePicker,
  InputNumber,
  Alert,
} from 'antd';
import {
  PlayCircleOutlined,
  ThunderboltOutlined,
  UserOutlined,
  ShoppingOutlined,
  CalendarOutlined,
} from '@ant-design/icons';
import type { TestContext } from '@/services/rule';

const { TextArea } = Input;
const { Text } = Typography;

/**
 * 预置测试场景
 */
export interface PresetScenario {
  key: string;
  name: string;
  description: string;
  context: TestContext;
}

/**
 * 预置场景列表
 */
const presetScenarios: PresetScenario[] = [
  {
    key: 'first_checkin',
    name: '首次签到',
    description: '模拟用户首次签到事件',
    context: {
      eventType: 'check_in',
      eventData: {
        type: 'check_in',
        isFirstTime: true,
        consecutiveDays: 1,
      },
      userId: 'test-user-001',
      membershipLevel: 'silver',
      timestamp: new Date().toISOString(),
      user: {
        level: 3,
        points: 1500,
        registerDays: 30,
        tags: ['active', 'new_member'],
      },
    },
  },
  {
    key: 'consecutive_checkin',
    name: '连续签到7天',
    description: '模拟用户连续签到满7天',
    context: {
      eventType: 'check_in',
      eventData: {
        type: 'check_in',
        isFirstTime: false,
        consecutiveDays: 7,
      },
      userId: 'test-user-002',
      membershipLevel: 'gold',
      timestamp: new Date().toISOString(),
      user: {
        level: 5,
        points: 5000,
        registerDays: 90,
        tags: ['loyal', 'vip'],
      },
    },
  },
  {
    key: 'large_purchase',
    name: '大额消费',
    description: '模拟用户完成10000元订单',
    context: {
      eventType: 'purchase',
      eventData: {
        type: 'purchase',
        amount: 10000,
        productCount: 5,
        category: 'electronics',
      },
      userId: 'test-user-003',
      membershipLevel: 'platinum',
      timestamp: new Date().toISOString(),
      user: {
        level: 8,
        points: 20000,
        registerDays: 365,
        tags: ['whale', 'vip'],
      },
      order: {
        amount: 10000,
        count: 50,
        status: 'completed',
      },
    },
  },
  {
    key: 'first_purchase',
    name: '首次消费',
    description: '模拟新用户首次下单',
    context: {
      eventType: 'purchase',
      eventData: {
        type: 'purchase',
        amount: 299,
        productCount: 1,
        isFirstOrder: true,
      },
      userId: 'test-user-004',
      membershipLevel: 'bronze',
      timestamp: new Date().toISOString(),
      user: {
        level: 1,
        points: 100,
        registerDays: 3,
        tags: ['new_member'],
      },
      order: {
        amount: 299,
        count: 1,
        status: 'completed',
      },
    },
  },
  {
    key: 'level_up',
    name: '用户升级',
    description: '模拟用户等级升级事件',
    context: {
      eventType: 'level_up',
      eventData: {
        type: 'level_up',
        previousLevel: 4,
        newLevel: 5,
      },
      userId: 'test-user-005',
      membershipLevel: 'gold',
      timestamp: new Date().toISOString(),
      user: {
        level: 5,
        points: 8000,
        registerDays: 180,
        tags: ['active', 'growing'],
      },
    },
  },
  {
    key: 'birthday',
    name: '生日当天',
    description: '模拟用户生日当天访问',
    context: {
      eventType: 'visit',
      eventData: {
        type: 'visit',
        isBirthday: true,
        source: 'app',
      },
      userId: 'test-user-006',
      membershipLevel: 'silver',
      timestamp: new Date().toISOString(),
      user: {
        level: 4,
        points: 3000,
        registerDays: 200,
        tags: ['birthday'],
      },
    },
  },
];

/**
 * 测试面板属性
 */
export interface TestPanelProps {
  /** 是否可见 */
  open: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 运行测试回调 */
  onRunTest: (context: TestContext) => void;
  /** 是否正在测试 */
  loading?: boolean;
}

const TestPanel: React.FC<TestPanelProps> = ({
  open,
  onClose,
  onRunTest,
  loading = false,
}) => {
  const [form] = Form.useForm();
  const [activeTab, setActiveTab] = useState<'preset' | 'custom' | 'json'>('preset');
  const [selectedPreset, setSelectedPreset] = useState<string>();
  const [jsonInput, setJsonInput] = useState<string>(
    JSON.stringify(presetScenarios[0].context, null, 2)
  );
  const [jsonError, setJsonError] = useState<string>();

  /**
   * 选中预置场景
   */
  const handleSelectPreset = (key: string) => {
    setSelectedPreset(key);
    const scenario = presetScenarios.find((s) => s.key === key);
    if (scenario) {
      setJsonInput(JSON.stringify(scenario.context, null, 2));
      // 同步更新表单
      form.setFieldsValue({
        eventType: scenario.context.eventType,
        userId: scenario.context.userId,
        membershipLevel: scenario.context.membershipLevel,
        userLevel: scenario.context.user?.level,
        userPoints: scenario.context.user?.points,
        orderAmount: scenario.context.order?.amount,
        orderCount: scenario.context.order?.count,
      });
    }
  };

  /**
   * 运行预置场景测试
   */
  const handleRunPreset = () => {
    if (!selectedPreset) return;
    const scenario = presetScenarios.find((s) => s.key === selectedPreset);
    if (scenario) {
      onRunTest(scenario.context);
    }
  };

  /**
   * 运行自定义表单测试
   */
  const handleRunCustom = async () => {
    try {
      const values = await form.validateFields();
      const context: TestContext = {
        eventType: values.eventType,
        eventData: {
          type: values.eventType,
          ...values.eventData,
        },
        userId: values.userId,
        membershipLevel: values.membershipLevel,
        timestamp: values.timestamp?.toISOString() || new Date().toISOString(),
        user: {
          level: values.userLevel,
          points: values.userPoints,
          registerDays: values.registerDays,
          tags: values.userTags?.split(',').map((t: string) => t.trim()) || [],
        },
        order: {
          amount: values.orderAmount,
          count: values.orderCount,
          status: values.orderStatus || 'completed',
        },
      };
      onRunTest(context);
    } catch {
      // 表单校验失败
    }
  };

  /**
   * 运行 JSON 测试
   */
  const handleRunJson = () => {
    try {
      const context = JSON.parse(jsonInput) as TestContext;
      setJsonError(undefined);
      onRunTest(context);
    } catch (e) {
      setJsonError('JSON 格式错误: ' + (e as Error).message);
    }
  };

  /**
   * JSON 输入变更
   */
  const handleJsonChange = (value: string) => {
    setJsonInput(value);
    try {
      JSON.parse(value);
      setJsonError(undefined);
    } catch {
      // 实时校验时不显示错误
    }
  };

  /**
   * 预置场景选项
   */
  const scenarioOptions = useMemo(
    () =>
      presetScenarios.map((s) => ({
        value: s.key,
        label: s.name,
      })),
    []
  );

  /**
   * 事件类型选项
   */
  const eventTypeOptions = [
    { value: 'check_in', label: '签到' },
    { value: 'purchase', label: '消费' },
    { value: 'level_up', label: '升级' },
    { value: 'visit', label: '访问' },
    { value: 'share', label: '分享' },
    { value: 'invite', label: '邀请' },
  ];

  /**
   * 会员等级选项
   */
  const membershipOptions = [
    { value: 'bronze', label: '青铜' },
    { value: 'silver', label: '白银' },
    { value: 'gold', label: '黄金' },
    { value: 'platinum', label: '铂金' },
    { value: 'diamond', label: '钻石' },
  ];

  return (
    <Drawer
      title="规则测试"
      placement="right"
      width={480}
      open={open}
      onClose={onClose}
      extra={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={loading}
            onClick={() => {
              switch (activeTab) {
                case 'preset':
                  handleRunPreset();
                  break;
                case 'custom':
                  handleRunCustom();
                  break;
                case 'json':
                  handleRunJson();
                  break;
              }
            }}
          >
            运行测试
          </Button>
        </Space>
      }
    >
      <Tabs
        activeKey={activeTab}
        onChange={(key) => setActiveTab(key as 'preset' | 'custom' | 'json')}
        items={[
          {
            key: 'preset',
            label: '预置场景',
            children: (
              <div>
                <Alert
                  type="info"
                  message="选择一个预置的测试场景快速验证规则"
                  style={{ marginBottom: 16 }}
                  showIcon
                />
                <Select
                  placeholder="选择测试场景"
                  style={{ width: '100%', marginBottom: 16 }}
                  value={selectedPreset}
                  onChange={handleSelectPreset}
                  options={scenarioOptions}
                />
                {selectedPreset && (
                  <Card size="small">
                    <Text strong>
                      {presetScenarios.find((s) => s.key === selectedPreset)?.name}
                    </Text>
                    <br />
                    <Text type="secondary">
                      {presetScenarios.find((s) => s.key === selectedPreset)?.description}
                    </Text>
                    <Divider style={{ margin: '12px 0' }} />
                    <pre
                      style={{
                        fontSize: 12,
                        maxHeight: 300,
                        overflow: 'auto',
                        background: '#f5f5f5',
                        padding: 8,
                        borderRadius: 4,
                      }}
                    >
                      {JSON.stringify(
                        presetScenarios.find((s) => s.key === selectedPreset)?.context,
                        null,
                        2
                      )}
                    </pre>
                  </Card>
                )}
              </div>
            ),
          },
          {
            key: 'custom',
            label: '自定义表单',
            children: (
              <Form form={form} layout="vertical" size="small">
                <Card
                  size="small"
                  title={
                    <>
                      <ThunderboltOutlined /> 事件信息
                    </>
                  }
                  style={{ marginBottom: 12 }}
                >
                  <Form.Item
                    name="eventType"
                    label="事件类型"
                    rules={[{ required: true, message: '请选择事件类型' }]}
                  >
                    <Select options={eventTypeOptions} placeholder="选择事件类型" />
                  </Form.Item>
                  <Form.Item name="timestamp" label="事件时间">
                    <DatePicker showTime style={{ width: '100%' }} />
                  </Form.Item>
                </Card>

                <Card
                  size="small"
                  title={
                    <>
                      <UserOutlined /> 用户信息
                    </>
                  }
                  style={{ marginBottom: 12 }}
                >
                  <Form.Item
                    name="userId"
                    label="用户 ID"
                    rules={[{ required: true, message: '请输入用户 ID' }]}
                  >
                    <Input placeholder="test-user-001" />
                  </Form.Item>
                  <Form.Item name="membershipLevel" label="会员等级">
                    <Select options={membershipOptions} placeholder="选择会员等级" />
                  </Form.Item>
                  <Space style={{ width: '100%' }} size={8}>
                    <Form.Item name="userLevel" label="用户等级" style={{ flex: 1 }}>
                      <InputNumber min={1} max={100} style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item name="userPoints" label="用户积分" style={{ flex: 1 }}>
                      <InputNumber min={0} style={{ width: '100%' }} />
                    </Form.Item>
                  </Space>
                  <Form.Item name="registerDays" label="注册天数">
                    <InputNumber min={0} style={{ width: '100%' }} />
                  </Form.Item>
                  <Form.Item name="userTags" label="用户标签">
                    <Input placeholder="active, vip（逗号分隔）" />
                  </Form.Item>
                </Card>

                <Card
                  size="small"
                  title={
                    <>
                      <ShoppingOutlined /> 订单信息
                    </>
                  }
                  style={{ marginBottom: 12 }}
                >
                  <Space style={{ width: '100%' }} size={8}>
                    <Form.Item name="orderAmount" label="订单金额" style={{ flex: 1 }}>
                      <InputNumber min={0} prefix="¥" style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item name="orderCount" label="订单数量" style={{ flex: 1 }}>
                      <InputNumber min={0} style={{ width: '100%' }} />
                    </Form.Item>
                  </Space>
                </Card>

                <Card
                  size="small"
                  title={
                    <>
                      <CalendarOutlined /> 时间条件
                    </>
                  }
                >
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    时间条件将根据事件时间自动计算（小时、星期几等）
                  </Text>
                </Card>
              </Form>
            ),
          },
          {
            key: 'json',
            label: 'JSON 编辑',
            children: (
              <div>
                <Alert
                  type="info"
                  message="直接编辑测试上下文的 JSON 数据"
                  style={{ marginBottom: 16 }}
                  showIcon
                />
                {jsonError && (
                  <Alert
                    type="error"
                    message={jsonError}
                    style={{ marginBottom: 16 }}
                    showIcon
                  />
                )}
                <TextArea
                  value={jsonInput}
                  onChange={(e) => handleJsonChange(e.target.value)}
                  rows={20}
                  style={{ fontFamily: 'monospace', fontSize: 12 }}
                />
              </div>
            ),
          },
        ]}
      />
    </Drawer>
  );
};

export default TestPanel;

/**
 * 通知配置列表页面
 *
 * 提供通知配置的增删改查和测试发送功能
 */

import React, { useRef, useState } from 'react';
import {
  Button,
  Tag,
  Space,
  Popconfirm,
  Select,
  Switch,
  Modal,
  Form,
  Input,
  InputNumber,
} from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  ReloadOutlined,
  EditOutlined,
  DeleteOutlined,
  SendOutlined,
} from '@ant-design/icons';
import { formatDate } from '@/utils/format';
import {
  useNotificationConfigs,
  useCreateNotificationConfig,
  useUpdateNotificationConfig,
  useDeleteNotificationConfig,
  useTestNotification,
} from '@/hooks/useNotification';
import { useBadgeList } from '@/hooks';
import {
  type NotificationConfig,
  type NotificationConfigParams,
  type CreateNotificationConfigRequest,
  TRIGGER_TYPE_OPTIONS,
  CHANNEL_OPTIONS,
} from '@/services/notification';

/**
 * 通知配置列表页面
 */
const NotificationConfigsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [form] = Form.useForm();
  const [testForm] = Form.useForm();

  // 查询参数
  const [queryParams, setQueryParams] = useState<NotificationConfigParams>({
    page: 1,
    pageSize: 20,
  });

  // 模态框状态
  const [formOpen, setFormOpen] = useState(false);
  const [testOpen, setTestOpen] = useState(false);
  const [editingConfig, setEditingConfig] = useState<NotificationConfig | undefined>();
  const [testingConfigId, setTestingConfigId] = useState<number | undefined>();

  // 数据查询
  const { data, isLoading, refetch } = useNotificationConfigs(queryParams);
  const { data: badgesData } = useBadgeList({ page: 1, pageSize: 200 });

  // 变更操作
  const createMutation = useCreateNotificationConfig();
  const updateMutation = useUpdateNotificationConfig();
  const deleteMutation = useDeleteNotificationConfig();
  const testMutation = useTestNotification();

  const badges = badgesData?.items || [];

  /**
   * 打开创建/编辑表单
   */
  const handleOpenForm = (config?: NotificationConfig) => {
    setEditingConfig(config);
    if (config) {
      form.setFieldsValue({
        badgeId: config.badgeId,
        benefitId: config.benefitId,
        triggerType: config.triggerType,
        channels: config.channels,
        templateId: config.templateId,
        advanceDays: config.advanceDays,
        retryCount: config.retryCount,
        retryIntervalSeconds: config.retryIntervalSeconds,
        enabled: config.enabled,
      });
    } else {
      form.resetFields();
      form.setFieldsValue({
        channels: ['app_push'],
        retryCount: 3,
        retryIntervalSeconds: 60,
        enabled: true,
      });
    }
    setFormOpen(true);
  };

  /**
   * 提交表单
   */
  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();

      if (editingConfig) {
        await updateMutation.mutateAsync({
          id: editingConfig.id,
          data: {
            triggerType: values.triggerType,
            channels: values.channels,
            templateId: values.templateId,
            advanceDays: values.advanceDays,
            retryCount: values.retryCount,
            retryIntervalSeconds: values.retryIntervalSeconds,
            enabled: values.enabled,
          },
        });
      } else {
        await createMutation.mutateAsync(values as CreateNotificationConfigRequest);
      }

      setFormOpen(false);
      refetch();
    } catch (error) {
      // 表单验证失败，不需要额外处理
    }
  };

  /**
   * 删除配置
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
    refetch();
  };

  /**
   * 打开测试发送模态框
   */
  const handleOpenTest = (configId: number) => {
    setTestingConfigId(configId);
    testForm.resetFields();
    setTestOpen(true);
  };

  /**
   * 执行测试发送
   */
  const handleTest = async () => {
    try {
      const values = await testForm.validateFields();
      if (!testingConfigId) return;

      await testMutation.mutateAsync({
        configId: testingConfigId,
        testUserId: values.testUserId,
      });

      setTestOpen(false);
    } catch (error) {
      // 表单验证失败
    }
  };

  /**
   * 切换启用状态
   */
  const handleToggleEnabled = async (config: NotificationConfig, enabled: boolean) => {
    await updateMutation.mutateAsync({
      id: config.id,
      data: { enabled },
    });
    refetch();
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<NotificationConfig>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 60,
      search: false,
    },
    {
      title: '触发类型',
      dataIndex: 'triggerType',
      width: 140,
      renderFormItem: () => (
        <Select
          placeholder="选择触发类型"
          allowClear
          options={TRIGGER_TYPE_OPTIONS}
        />
      ),
      render: (_, record) => {
        const option = TRIGGER_TYPE_OPTIONS.find((o) => o.value === record.triggerType);
        return option?.label || record.triggerType;
      },
    },
    {
      title: '关联徽章',
      dataIndex: 'badgeName',
      width: 150,
      search: false,
      render: (_, record) => record.badgeName || '-',
    },
    {
      title: '关联权益',
      dataIndex: 'benefitName',
      width: 150,
      search: false,
      render: (_, record) => record.benefitName || '-',
    },
    {
      title: '通知渠道',
      dataIndex: 'channels',
      width: 200,
      search: false,
      render: (_, record) => (
        <Space size={[0, 4]} wrap>
          {record.channels.map((ch) => {
            const option = CHANNEL_OPTIONS.find((o) => o.value === ch);
            return (
              <Tag key={ch} color="blue">
                {option?.label || ch}
              </Tag>
            );
          })}
        </Space>
      ),
    },
    {
      title: '提前天数',
      dataIndex: 'advanceDays',
      width: 90,
      search: false,
      render: (_, record) =>
        record.advanceDays !== undefined ? `${record.advanceDays} 天` : '-',
    },
    {
      title: '重试配置',
      key: 'retry',
      width: 120,
      search: false,
      render: (_, record) =>
        `${record.retryCount} 次 / ${record.retryIntervalSeconds}s`,
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 80,
      renderFormItem: () => (
        <Select
          placeholder="状态"
          allowClear
          options={[
            { value: true, label: '启用' },
            { value: false, label: '禁用' },
          ]}
        />
      ),
      render: (_, record) => (
        <Switch
          checked={record.enabled}
          onChange={(checked) => handleToggleEnabled(record, checked)}
          size="small"
        />
      ),
    },
    {
      title: '更新时间',
      dataIndex: 'updatedAt',
      width: 160,
      search: false,
      render: (_, record) => formatDate(record.updatedAt),
    },
    {
      title: '操作',
      key: 'action',
      width: 160,
      fixed: 'right',
      search: false,
      render: (_, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<SendOutlined />}
            onClick={() => handleOpenTest(record.id)}
          >
            测试
          </Button>
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleOpenForm(record)}
          >
            编辑
          </Button>
          <Popconfirm
            title="确认删除此通知配置?"
            onConfirm={() => handleDelete(record.id)}
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer
      header={{
        title: '通知配置',
        subTitle: '配置徽章和权益相关的通知策略',
      }}
    >
      <ProTable<NotificationConfig>
        actionRef={actionRef}
        columns={columns}
        dataSource={data?.items}
        loading={isLoading}
        rowKey="id"
        search={{
          labelWidth: 'auto',
          defaultCollapsed: false,
        }}
        pagination={{
          total: data?.total,
          current: queryParams.page,
          pageSize: queryParams.pageSize,
          showSizeChanger: true,
          showQuickJumper: true,
          onChange: (page, pageSize) => {
            setQueryParams((prev) => ({ ...prev, page, pageSize }));
          },
        }}
        onSubmit={(params) => {
          setQueryParams((prev) => ({
            ...prev,
            ...params,
            page: 1,
          }));
        }}
        onReset={() => {
          setQueryParams({ page: 1, pageSize: 20 });
        }}
        toolBarRender={() => [
          <Button
            key="refresh"
            icon={<ReloadOutlined />}
            onClick={() => refetch()}
          >
            刷新
          </Button>,
          <Button
            key="create"
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => handleOpenForm()}
          >
            新建配置
          </Button>,
        ]}
        scroll={{ x: 1200 }}
      />

      {/* 创建/编辑表单 */}
      <Modal
        title={editingConfig ? '编辑通知配置' : '新建通知配置'}
        open={formOpen}
        onCancel={() => setFormOpen(false)}
        onOk={handleSubmit}
        confirmLoading={createMutation.isPending || updateMutation.isPending}
        width={600}
      >
        <Form form={form} layout="vertical">
          {!editingConfig && (
            <>
              <Form.Item
                name="badgeId"
                label="关联徽章"
                tooltip="与权益二选一"
              >
                <Select
                  placeholder="选择徽章（可选）"
                  allowClear
                  showSearch
                  optionFilterProp="children"
                >
                  {badges.map((badge) => (
                    <Select.Option key={badge.id} value={badge.id}>
                      {badge.name}
                    </Select.Option>
                  ))}
                </Select>
              </Form.Item>
              <Form.Item
                name="benefitId"
                label="关联权益"
                tooltip="与徽章二选一"
              >
                <InputNumber
                  placeholder="输入权益 ID（可选）"
                  style={{ width: '100%' }}
                  min={1}
                />
              </Form.Item>
            </>
          )}

          <Form.Item
            name="triggerType"
            label="触发类型"
            rules={[{ required: true, message: '请选择触发类型' }]}
          >
            <Select
              placeholder="选择触发类型"
              options={TRIGGER_TYPE_OPTIONS}
            />
          </Form.Item>

          <Form.Item
            name="channels"
            label="通知渠道"
            rules={[{ required: true, message: '请选择至少一个通知渠道' }]}
          >
            <Select
              mode="multiple"
              placeholder="选择通知渠道"
              options={CHANNEL_OPTIONS}
            />
          </Form.Item>

          <Form.Item name="templateId" label="消息模板 ID">
            <Input placeholder="输入消息模板 ID（可选）" />
          </Form.Item>

          <Form.Item name="advanceDays" label="提前提醒天数">
            <InputNumber
              placeholder="提前 N 天提醒"
              style={{ width: '100%' }}
              min={1}
              max={30}
            />
          </Form.Item>

          <Space>
            <Form.Item name="retryCount" label="重试次数">
              <InputNumber min={0} max={10} />
            </Form.Item>
            <Form.Item name="retryIntervalSeconds" label="重试间隔（秒）">
              <InputNumber min={10} max={3600} />
            </Form.Item>
          </Space>

          {editingConfig && (
            <Form.Item name="enabled" label="启用状态" valuePropName="checked">
              <Switch />
            </Form.Item>
          )}
        </Form>
      </Modal>

      {/* 测试发送模态框 */}
      <Modal
        title="测试发送"
        open={testOpen}
        onCancel={() => setTestOpen(false)}
        onOk={handleTest}
        confirmLoading={testMutation.isPending}
      >
        <Form form={testForm} layout="vertical">
          <Form.Item
            name="testUserId"
            label="测试用户 ID"
            rules={[{ required: true, message: '请输入测试用户 ID' }]}
          >
            <Input placeholder="输入测试用户 ID" />
          </Form.Item>
        </Form>
      </Modal>
    </PageContainer>
  );
};

export default NotificationConfigsPage;

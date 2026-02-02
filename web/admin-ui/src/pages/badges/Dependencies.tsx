/**
 * 徽章依赖配置页面
 *
 * 管理徽章之间的依赖关系，支持前置条件、消耗和互斥三种依赖类型
 */

import React, { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Card,
  Table,
  Button,
  Space,
  Modal,
  Form,
  Select,
  Input,
  InputNumber,
  Switch,
  Popconfirm,
  Tag,
  Typography,
  Tooltip,
  Drawer,
} from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  ReloadOutlined,
  ArrowLeftOutlined,
  QuestionCircleOutlined,
  ApartmentOutlined,
} from '@ant-design/icons';
import {
  useDependencyList,
  useCreateDependency,
  useDeleteDependency,
  useRefreshDependencyCache,
} from '@/hooks/useDependency';
import type { BadgeDependency, CreateDependencyRequest, DependencyType } from '@/services/dependency';
import DependencyGraph from './components/DependencyGraph';

const { Text } = Typography;

/**
 * 依赖类型选项
 */
const dependencyTypeOptions = [
  { label: '前置条件', value: 'prerequisite' },
  { label: '消耗', value: 'consume' },
  { label: '互斥', value: 'exclusive' },
];

/**
 * 依赖类型颜色映射
 */
const dependencyTypeColors: Record<DependencyType, string> = {
  prerequisite: 'blue',
  consume: 'orange',
  exclusive: 'red',
};

/**
 * 依赖类型说明
 */
const dependencyTypeDescriptions: Record<DependencyType, string> = {
  prerequisite: '必须持有指定数量的依赖徽章才能获取当前徽章',
  consume: '获取当前徽章时会消耗指定数量的依赖徽章',
  exclusive: '与互斥组内的其他徽章不能同时持有',
};

const DependenciesPage: React.FC = () => {
  const { badgeId } = useParams<{ badgeId: string }>();
  const navigate = useNavigate();
  const [modalVisible, setModalVisible] = useState(false);
  const [graphDrawerVisible, setGraphDrawerVisible] = useState(false);
  const [form] = Form.useForm<CreateDependencyRequest>();

  // React Query Hooks
  const { data: dependencies = [], isLoading } = useDependencyList(badgeId || '');
  const createMutation = useCreateDependency(badgeId || '');
  const deleteMutation = useDeleteDependency(badgeId || '');
  const refreshCacheMutation = useRefreshDependencyCache();

  /**
   * 监听依赖类型变化，动态控制互斥组字段
   */
  const dependencyType = Form.useWatch('dependencyType', form);

  /**
   * 打开新建弹窗
   */
  const handleOpenModal = () => {
    form.resetFields();
    setModalVisible(true);
  };

  /**
   * 关闭弹窗
   */
  const handleCloseModal = () => {
    setModalVisible(false);
    form.resetFields();
  };

  /**
   * 提交创建表单
   */
  const handleCreate = async (values: CreateDependencyRequest) => {
    try {
      await createMutation.mutateAsync(values);
      handleCloseModal();
    } catch {
      // 错误已在 hook 中处理
    }
  };

  /**
   * 删除依赖关系
   */
  const handleDelete = async (id: string) => {
    try {
      await deleteMutation.mutateAsync(id);
    } catch {
      // 错误已在 hook 中处理
    }
  };

  /**
   * 刷新依赖缓存
   */
  const handleRefreshCache = async () => {
    try {
      await refreshCacheMutation.mutateAsync();
    } catch {
      // 错误已在 hook 中处理
    }
  };

  /**
   * 表格列定义
   */
  const columns = [
    {
      title: '依赖徽章 ID',
      dataIndex: 'dependsOnBadgeId',
      key: 'dependsOnBadgeId',
      ellipsis: true,
      width: 280,
      render: (val: string) => (
        <Text copyable style={{ fontFamily: 'monospace', fontSize: 12 }}>
          {val}
        </Text>
      ),
    },
    {
      title: '依赖类型',
      dataIndex: 'dependencyType',
      key: 'dependencyType',
      width: 120,
      render: (type: DependencyType) => (
        <Tooltip title={dependencyTypeDescriptions[type]}>
          <Tag color={dependencyTypeColors[type]}>
            {dependencyTypeOptions.find((o) => o.value === type)?.label || type}
          </Tag>
        </Tooltip>
      ),
    },
    {
      title: '需求数量',
      dataIndex: 'requiredQuantity',
      key: 'requiredQuantity',
      width: 100,
      align: 'center' as const,
    },
    {
      title: '依赖组',
      dataIndex: 'dependencyGroupId',
      key: 'dependencyGroupId',
      width: 120,
      render: (val: string) => <Tag>{val}</Tag>,
    },
    {
      title: (
        <span>
          互斥组{' '}
          <Tooltip title="仅互斥类型需要设置，同一互斥组内的徽章不能同时持有">
            <QuestionCircleOutlined style={{ color: '#999' }} />
          </Tooltip>
        </span>
      ),
      dataIndex: 'exclusiveGroupId',
      key: 'exclusiveGroupId',
      width: 120,
      render: (val: string | null) => (val ? <Tag color="red">{val}</Tag> : '-'),
    },
    {
      title: (
        <span>
          自动触发{' '}
          <Tooltip title="满足条件时是否自动触发级联发放">
            <QuestionCircleOutlined style={{ color: '#999' }} />
          </Tooltip>
        </span>
      ),
      dataIndex: 'autoTrigger',
      key: 'autoTrigger',
      width: 100,
      align: 'center' as const,
      render: (val: boolean) => (val ? <Tag color="green">是</Tag> : <Tag>否</Tag>),
    },
    {
      title: '优先级',
      dataIndex: 'priority',
      key: 'priority',
      width: 80,
      align: 'center' as const,
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      key: 'enabled',
      width: 80,
      align: 'center' as const,
      render: (val: boolean) =>
        val ? <Tag color="success">启用</Tag> : <Tag color="default">禁用</Tag>,
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right' as const,
      render: (_: unknown, record: BadgeDependency) => (
        <Popconfirm
          title="确定要删除这条依赖关系吗？"
          description="删除后不可恢复"
          onConfirm={() => handleDelete(record.id)}
          okText="确认删除"
          cancelText="取消"
          okButtonProps={{ danger: true }}
        >
          <Button
            type="link"
            danger
            size="small"
            icon={<DeleteOutlined />}
            loading={deleteMutation.isPending}
          >
            删除
          </Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <Card
      title={
        <Space>
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => navigate('/badges/definitions')}
          />
          <span>徽章依赖配置</span>
          <Text type="secondary" style={{ fontSize: 12, fontFamily: 'monospace' }}>
            {badgeId}
          </Text>
        </Space>
      }
      extra={
        <Space>
          <Button
            icon={<ApartmentOutlined />}
            onClick={() => setGraphDrawerVisible(true)}
          >
            查看依赖图
          </Button>
          <Button
            icon={<ReloadOutlined />}
            onClick={handleRefreshCache}
            loading={refreshCacheMutation.isPending}
          >
            刷新缓存
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={handleOpenModal}>
            添加依赖
          </Button>
        </Space>
      }
    >
      <Table
        className="dependency-list"
        columns={columns}
        dataSource={dependencies}
        rowKey="id"
        loading={isLoading}
        pagination={false}
        scroll={{ x: 1200 }}
        locale={{ emptyText: '暂无依赖关系配置' }}
        rowClassName={() => 'dependency-item'}
      />

      <Modal
        title="添加依赖关系"
        open={modalVisible}
        onCancel={handleCloseModal}
        footer={null}
        destroyOnClose
        width={500}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleCreate}
          initialValues={{
            requiredQuantity: 1,
            priority: 0,
            autoTrigger: false,
          }}
        >
          <Form.Item
            name="dependsOnBadgeId"
            label="依赖徽章 ID"
            rules={[{ required: true, message: '请输入依赖徽章 ID' }]}
            tooltip="被依赖的徽章 UUID"
          >
            <Input
              id="depends_on_badge_id"
              placeholder="输入依赖徽章的 UUID"
              style={{ fontFamily: 'monospace' }}
            />
          </Form.Item>

          <Form.Item
            name="dependencyType"
            label="依赖类型"
            rules={[{ required: true, message: '请选择依赖类型' }]}
          >
            <Select
              id="dependency_type"
              options={dependencyTypeOptions}
              placeholder="选择依赖类型"
              onChange={() => {
                // 切换类型时清空互斥组
                form.setFieldValue('exclusiveGroupId', undefined);
              }}
            />
          </Form.Item>

          <Form.Item
            name="requiredQuantity"
            label="需求数量"
            tooltip="用户需要持有的依赖徽章数量"
          >
            <InputNumber id="required_quantity" min={1} max={9999} style={{ width: '100%' }} />
          </Form.Item>

          <Form.Item
            name="dependencyGroupId"
            label="依赖组 ID"
            rules={[{ required: true, message: '请输入依赖组 ID' }]}
            tooltip="同组条件是 AND 关系，不同组是 OR 关系"
          >
            <Input placeholder="如: group1" />
          </Form.Item>

          {dependencyType === 'exclusive' && (
            <Form.Item
              name="exclusiveGroupId"
              label="互斥组 ID"
              rules={[{ required: true, message: '互斥类型必须填写互斥组 ID' }]}
              tooltip="同一互斥组内的徽章不能同时持有"
            >
              <Input id="exclusive_group_id" placeholder="如: exclusive_group_1" />
            </Form.Item>
          )}

          <Form.Item
            name="autoTrigger"
            label="自动触发"
            valuePropName="checked"
            tooltip="满足依赖条件时是否自动触发级联发放"
          >
            <Switch id="auto_trigger" />
          </Form.Item>

          <Form.Item
            name="priority"
            label="优先级"
            tooltip="数值越小优先级越高，影响级联评估顺序"
          >
            <InputNumber min={0} max={9999} style={{ width: '100%' }} />
          </Form.Item>

          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button onClick={handleCloseModal}>取消</Button>
              <Button type="primary" htmlType="submit" loading={createMutation.isPending}>
                确定
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      <Drawer
        title="依赖关系图"
        placement="right"
        width={900}
        open={graphDrawerVisible}
        onClose={() => setGraphDrawerVisible(false)}
        destroyOnClose
      >
        <DependencyGraph badgeId={badgeId} height={600} />
      </Drawer>
    </Card>
  );
};

export default DependenciesPage;

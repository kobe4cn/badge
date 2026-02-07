/**
 * 权益列表页面
 *
 * 提供权益的列表展示和基础管理功能
 */

import React, { useRef, useState } from 'react';
import { Button, Tag, Space, Popconfirm, Input, Select, message } from 'antd';
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
  SearchOutlined,
} from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { formatDate } from '@/utils/format';
import {
  listBenefits,
  deleteBenefit,
  createBenefit,
  updateBenefit,
  type Benefit,
  type BenefitType,
  type BenefitStatus,
  type BenefitQueryParams,
  type CreateBenefitRequest,
  type UpdateBenefitRequest,
} from '@/services/benefit';
import BenefitForm from './components/BenefitForm';

/**
 * 权益类型显示配置
 */
const BENEFIT_TYPE_MAP: Record<BenefitType, { text: string; color: string }> = {
  POINTS: { text: '积分', color: 'blue' },
  COUPON: { text: '优惠券', color: 'orange' },
  PHYSICAL: { text: '实物', color: 'green' },
  VIRTUAL: { text: '虚拟物品', color: 'purple' },
  THIRD_PARTY: { text: '第三方', color: 'cyan' },
};

/**
 * 权益状态显示配置
 */
const BENEFIT_STATUS_MAP: Record<BenefitStatus, { text: string; status: string }> = {
  DRAFT: { text: '草稿', status: 'default' },
  ACTIVE: { text: '启用', status: 'success' },
  INACTIVE: { text: '禁用', status: 'warning' },
  EXPIRED: { text: '已过期', status: 'error' },
};

const BenefitsListPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const queryClient = useQueryClient();

  // 查询参数
  const [queryParams, setQueryParams] = useState<BenefitQueryParams>({
    page: 1,
    pageSize: 20,
  });

  const [formOpen, setFormOpen] = useState(false);
  const [editingBenefit, setEditingBenefit] = useState<Benefit | undefined>();

  // 获取权益列表
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['benefits', queryParams],
    queryFn: () => listBenefits(queryParams),
  });

  // 删除权益
  const deleteMutation = useMutation({
    mutationFn: deleteBenefit,
    onSuccess: () => {
      message.success('删除成功');
      queryClient.invalidateQueries({ queryKey: ['benefits'] });
    },
    onError: () => {
      message.error('删除失败');
    },
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateBenefitRequest) => createBenefit(data),
    onSuccess: () => {
      message.success('创建成功');
      queryClient.invalidateQueries({ queryKey: ['benefits'] });
    },
    onError: () => {
      message.error('创建失败');
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: UpdateBenefitRequest }) => updateBenefit(id, data),
    onSuccess: () => {
      message.success('更新成功');
      queryClient.invalidateQueries({ queryKey: ['benefits'] });
    },
    onError: () => {
      message.error('更新失败');
    },
  });

  const handleFormSubmit = async (values: CreateBenefitRequest) => {
    if (editingBenefit) {
      await updateMutation.mutateAsync({ id: editingBenefit.id, data: values });
    } else {
      await createMutation.mutateAsync(values);
    }
    return true;
  };

  /**
   * 删除处理
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<Benefit>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 80,
      search: false,
    },
    {
      title: '编码',
      dataIndex: 'code',
      width: 120,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '名称',
      dataIndex: 'name',
      width: 150,
      ellipsis: true,
      renderFormItem: () => (
        <Input placeholder="搜索权益名称" prefix={<SearchOutlined />} />
      ),
    },
    {
      title: '类型',
      dataIndex: 'benefitType',
      width: 100,
      render: (_, record) => {
        const config = BENEFIT_TYPE_MAP[record.benefitType];
        return <Tag color={config?.color}>{config?.text || record.benefitType}</Tag>;
      },
      renderFormItem: () => (
        <Select
          placeholder="选择类型"
          allowClear
          options={Object.entries(BENEFIT_TYPE_MAP).map(([value, { text }]) => ({
            value,
            label: text,
          }))}
        />
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      valueEnum: BENEFIT_STATUS_MAP,
      renderFormItem: () => (
        <Select
          placeholder="选择状态"
          allowClear
          options={Object.entries(BENEFIT_STATUS_MAP).map(([value, { text }]) => ({
            value,
            label: text,
          }))}
        />
      ),
    },
    {
      title: '库存',
      dataIndex: 'remainingStock',
      width: 100,
      search: false,
      render: (_, record) => {
        if (record.totalStock === null || record.totalStock === undefined) {
          return <span style={{ color: '#8c8c8c' }}>无限制</span>;
        }
        return (
          <span>
            {record.remainingStock} / {record.totalStock}
          </span>
        );
      },
    },
    {
      title: '已兑换',
      dataIndex: 'redeemedCount',
      width: 80,
      search: false,
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 170,
      search: false,
      sorter: true,
      render: (_, record) => formatDate(record.createdAt),
    },
    {
      title: '操作',
      valueType: 'option',
      width: 150,
      fixed: 'right',
      render: (_, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => {
              setEditingBenefit(record);
              setFormOpen(true);
            }}
          >
            编辑
          </Button>
          <Popconfirm
            title="确认删除"
            description="确定要删除该权益吗？"
            onConfirm={() => handleDelete(record.id)}
            okText="确认"
            cancelText="取消"
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
    <PageContainer title="权益列表">
      <ProTable<Benefit>
        actionRef={actionRef}
        rowKey="id"
        columns={columns}
        dataSource={data?.items}
        loading={isLoading}
        pagination={{
          current: queryParams.page,
          pageSize: queryParams.pageSize,
          total: data?.total || 0,
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total) => `共 ${total} 条`,
        }}
        search={{
          labelWidth: 'auto',
          defaultCollapsed: false,
        }}
        options={{
          density: true,
          fullScreen: true,
          reload: () => refetch(),
          setting: true,
        }}
        toolBarRender={() => [
          <Button
            key="create"
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => {
              setEditingBenefit(undefined);
              setFormOpen(true);
            }}
          >
            新建权益
          </Button>,
          <Button key="reload" icon={<ReloadOutlined />} onClick={() => refetch()}>
            刷新
          </Button>,
        ]}
        request={async (params) => {
          setQueryParams({
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            keyword: params.name,
            benefitType: params.benefitType,
            status: params.status,
          });
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1100 }}
      />
      <BenefitForm
        open={formOpen}
        onOpenChange={setFormOpen}
        initialValues={editingBenefit}
        onSubmit={handleFormSubmit}
        loading={createMutation.isPending || updateMutation.isPending}
      />
    </PageContainer>
  );
};

export default BenefitsListPage;

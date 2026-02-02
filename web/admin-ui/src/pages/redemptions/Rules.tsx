/**
 * 兑换规则管理页面
 *
 * 管理徽章兑换权益的规则配置
 */

import React, { useRef, useState } from 'react';
import { Button, Tag, Space, Popconfirm, Switch, message, Input } from 'antd';
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
  listRedemptionRules,
  deleteRedemptionRule,
  toggleRedemptionRule,
  type RedemptionRule,
  type RedemptionRuleQueryParams,
} from '@/services/redemption';

const RedemptionRulesPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const queryClient = useQueryClient();

  // 查询参数
  const [queryParams, setQueryParams] = useState<RedemptionRuleQueryParams>({
    page: 1,
    pageSize: 20,
  });

  // 获取规则列表
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['redemptionRules', queryParams],
    queryFn: () => listRedemptionRules(queryParams),
  });

  // 删除规则
  const deleteMutation = useMutation({
    mutationFn: deleteRedemptionRule,
    onSuccess: () => {
      message.success('删除成功');
      queryClient.invalidateQueries({ queryKey: ['redemptionRules'] });
    },
    onError: () => {
      message.error('删除失败');
    },
  });

  // 切换状态
  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) =>
      toggleRedemptionRule(id, enabled),
    onSuccess: () => {
      message.success('状态更新成功');
      queryClient.invalidateQueries({ queryKey: ['redemptionRules'] });
    },
    onError: () => {
      message.error('状态更新失败');
    },
  });

  /**
   * 删除处理
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
  };

  /**
   * 切换启用状态
   */
  const handleToggle = async (record: RedemptionRule, checked: boolean) => {
    await toggleMutation.mutateAsync({ id: record.id, enabled: checked });
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<RedemptionRule>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 80,
      search: false,
    },
    {
      title: '规则名称',
      dataIndex: 'name',
      width: 150,
      ellipsis: true,
      renderFormItem: () => (
        <Input placeholder="搜索规则名称" prefix={<SearchOutlined />} />
      ),
    },
    {
      title: '兑换权益',
      dataIndex: 'benefitName',
      width: 120,
      search: false,
      render: (_, record) => <Tag color="blue">{record.benefitName}</Tag>,
    },
    {
      title: '所需徽章',
      dataIndex: 'requiredBadges',
      width: 200,
      search: false,
      render: (_, record) => (
        <Space wrap size={4}>
          {record.requiredBadges.map((badge) => (
            <Tag key={badge.badgeId}>
              {badge.badgeName} x{badge.quantity}
            </Tag>
          ))}
        </Space>
      ),
    },
    {
      title: '自动兑换',
      dataIndex: 'autoRedeem',
      width: 90,
      search: false,
      render: (_, record) => (
        <Tag color={record.autoRedeem ? 'green' : 'default'}>
          {record.autoRedeem ? '是' : '否'}
        </Tag>
      ),
    },
    {
      title: '频率限制',
      dataIndex: 'frequencyConfig',
      width: 150,
      search: false,
      render: (_, record) => {
        const config = record.frequencyConfig;
        const limits: string[] = [];
        if (config.maxPerUser) limits.push(`用户${config.maxPerUser}次`);
        if (config.maxPerDay) limits.push(`每日${config.maxPerDay}次`);
        if (config.maxPerWeek) limits.push(`每周${config.maxPerWeek}次`);
        if (config.maxPerMonth) limits.push(`每月${config.maxPerMonth}次`);
        return limits.length > 0 ? limits.join('、') : '无限制';
      },
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 100,
      render: (_, record) => (
        <Switch
          checked={record.enabled}
          checkedChildren="启用"
          unCheckedChildren="禁用"
          loading={toggleMutation.isPending}
          onChange={(checked) => handleToggle(record, checked)}
        />
      ),
      valueEnum: {
        true: { text: '启用', status: 'Success' },
        false: { text: '禁用', status: 'Default' },
      },
    },
    {
      title: '有效期',
      key: 'validity',
      width: 200,
      search: false,
      render: (_, record) => {
        if (!record.startTime && !record.endTime) {
          return '永久有效';
        }
        const start = record.startTime ? formatDate(record.startTime, 'YYYY-MM-DD') : '-';
        const end = record.endTime ? formatDate(record.endTime, 'YYYY-MM-DD') : '-';
        return `${start} 至 ${end}`;
      },
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
              message.info('编辑功能开发中');
            }}
          >
            编辑
          </Button>
          <Popconfirm
            title="确认删除"
            description="确定要删除该兑换规则吗？"
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
    <PageContainer title="兑换规则">
      <ProTable<RedemptionRule>
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
              message.info('新建功能开发中');
            }}
          >
            新建规则
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
            enabled: params.enabled === 'true' ? true : params.enabled === 'false' ? false : undefined,
          });
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1400 }}
      />
    </PageContainer>
  );
};

export default RedemptionRulesPage;

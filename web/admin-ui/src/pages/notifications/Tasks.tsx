/**
 * 通知任务列表页面
 *
 * 展示通知发送记录，支持状态筛选和查看详情
 */

import React, { useRef, useState } from 'react';
import { Tag, Select, Input, Tooltip } from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  CheckCircleOutlined,
  ClockCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { formatDate } from '@/utils/format';
import { useNotificationTasks } from '@/hooks/useNotification';
import {
  type NotificationTask,
  type NotificationTaskParams,
  TRIGGER_TYPE_OPTIONS,
  CHANNEL_OPTIONS,
  TASK_STATUS_OPTIONS,
} from '@/services/notification';

/**
 * 状态图标映射
 */
const STATUS_ICONS: Record<string, React.ReactNode> = {
  pending: <ClockCircleOutlined />,
  sending: <SyncOutlined spin />,
  success: <CheckCircleOutlined />,
  failed: <CloseCircleOutlined />,
};

/**
 * 通知任务列表页面
 */
const NotificationTasksPage: React.FC = () => {
  const actionRef = useRef<ActionType>();

  // 查询参数
  const [queryParams, setQueryParams] = useState<NotificationTaskParams>({
    page: 1,
    pageSize: 20,
  });

  // 数据查询
  const { data, isLoading } = useNotificationTasks(queryParams);

  /**
   * 表格列定义
   */
  const columns: ProColumns<NotificationTask>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
    },
    {
      title: '配置 ID',
      dataIndex: 'configId',
      width: 80,
      renderFormItem: () => (
        <Input placeholder="配置 ID" type="number" />
      ),
    },
    {
      title: '用户 ID',
      dataIndex: 'userId',
      width: 140,
      ellipsis: true,
      copyable: true,
      renderFormItem: () => (
        <Input placeholder="用户 ID" />
      ),
    },
    {
      title: '触发类型',
      dataIndex: 'triggerType',
      width: 130,
      renderFormItem: () => (
        <Select
          placeholder="触发类型"
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
      title: '通知渠道',
      dataIndex: 'channel',
      width: 100,
      search: false,
      render: (_, record) => {
        const option = CHANNEL_OPTIONS.find((o) => o.value === record.channel);
        return (
          <Tag color="blue">{option?.label || record.channel}</Tag>
        );
      },
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      renderFormItem: () => (
        <Select
          placeholder="状态"
          allowClear
          options={TASK_STATUS_OPTIONS.map((o) => ({
            value: o.value,
            label: o.label,
          }))}
        />
      ),
      render: (_, record) => {
        const option = TASK_STATUS_OPTIONS.find((o) => o.value === record.status);
        return (
          <Tag
            color={option?.color || 'default'}
            icon={STATUS_ICONS[record.status]}
          >
            {option?.label || record.status}
          </Tag>
        );
      },
    },
    {
      title: '重试次数',
      dataIndex: 'retryCount',
      width: 90,
      search: false,
      render: (_, record) => record.retryCount || 0,
    },
    {
      title: '错误信息',
      dataIndex: 'errorMessage',
      width: 200,
      search: false,
      ellipsis: true,
      render: (_, record) =>
        record.errorMessage ? (
          <Tooltip title={record.errorMessage}>
            <span style={{ color: '#ff4d4f' }}>{record.errorMessage}</span>
          </Tooltip>
        ) : (
          '-'
        ),
    },
    {
      title: '发送时间',
      dataIndex: 'sentAt',
      width: 160,
      search: false,
      render: (_, record) => (record.sentAt ? formatDate(record.sentAt) : '-'),
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 160,
      search: false,
      render: (_, record) => formatDate(record.createdAt),
    },
  ];

  return (
    <PageContainer
      header={{
        title: '发送记录',
        subTitle: '查看通知发送任务的执行记录',
      }}
    >
      <ProTable<NotificationTask>
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
        scroll={{ x: 1200 }}
      />
    </PageContainer>
  );
};

export default NotificationTasksPage;

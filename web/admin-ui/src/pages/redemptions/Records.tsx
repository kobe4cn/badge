/**
 * 兑换记录页面
 *
 * 展示用户兑换权益的历史记录
 */

import React, { useRef, useState } from 'react';
import { Tag, Input, Select, DatePicker } from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import { SearchOutlined } from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { formatDate } from '@/utils/format';
import {
  listRedemptionOrders,
  type RedemptionOrder,
  type RedemptionOrderStatus,
  type RedemptionOrderQueryParams,
} from '@/services/redemption';

const { RangePicker } = DatePicker;

/**
 * 订单状态显示配置
 */
const ORDER_STATUS_MAP: Record<RedemptionOrderStatus, { text: string; color: string }> = {
  PENDING: { text: '待处理', color: 'processing' },
  PROCESSING: { text: '处理中', color: 'warning' },
  COMPLETED: { text: '已完成', color: 'success' },
  FAILED: { text: '失败', color: 'error' },
  CANCELLED: { text: '已取消', color: 'default' },
};

const RedemptionRecordsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();

  // 查询参数
  const [queryParams, setQueryParams] = useState<RedemptionOrderQueryParams>({
    page: 1,
    pageSize: 20,
  });

  // 获取兑换记录列表
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['redemptionOrders', queryParams],
    queryFn: () => listRedemptionOrders(queryParams),
  });

  /**
   * 表格列定义
   */
  const columns: ProColumns<RedemptionOrder>[] = [
    {
      title: '订单号',
      dataIndex: 'orderNo',
      width: 180,
      ellipsis: true,
      copyable: true,
      search: false,
    },
    {
      title: '用户 ID',
      dataIndex: 'userId',
      width: 150,
      ellipsis: true,
      renderFormItem: () => (
        <Input placeholder="输入用户ID" prefix={<SearchOutlined />} />
      ),
    },
    {
      title: '兑换规则',
      dataIndex: 'ruleName',
      width: 150,
      ellipsis: true,
      search: false,
    },
    {
      title: '兑换权益',
      dataIndex: 'benefitName',
      width: 120,
      search: false,
      render: (_, record) => <Tag color="blue">{record.benefitName}</Tag>,
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (_, record) => {
        const config = ORDER_STATUS_MAP[record.status];
        return <Tag color={config?.color}>{config?.text || record.status}</Tag>;
      },
      renderFormItem: () => (
        <Select
          placeholder="选择状态"
          allowClear
          options={Object.entries(ORDER_STATUS_MAP).map(([value, { text }]) => ({
            value,
            label: text,
          }))}
        />
      ),
    },
    {
      title: '失败原因',
      dataIndex: 'failureReason',
      width: 200,
      ellipsis: true,
      search: false,
      render: (_, record) => record.failureReason || '-',
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 170,
      sorter: true,
      render: (_, record) => formatDate(record.createdAt),
      renderFormItem: () => (
        <RangePicker
          placeholder={['开始日期', '结束日期']}
          style={{ width: '100%' }}
        />
      ),
    },
    {
      title: '完成时间',
      dataIndex: 'completedAt',
      width: 170,
      search: false,
      render: (_, record) => (record.completedAt ? formatDate(record.completedAt) : '-'),
    },
  ];

  return (
    <PageContainer title="兑换记录">
      <ProTable<RedemptionOrder>
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
        request={async (params) => {
          const dateRange = params.createdAt as [dayjs.Dayjs, dayjs.Dayjs] | undefined;
          setQueryParams({
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            userId: params.userId,
            status: params.status,
            startDate: dateRange?.[0]?.format('YYYY-MM-DD'),
            endDate: dateRange?.[1]?.format('YYYY-MM-DD'),
          });
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1300 }}
      />
    </PageContainer>
  );
};

export default RedemptionRecordsPage;

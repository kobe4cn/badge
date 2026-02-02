/**
 * 权益发放记录页面
 *
 * 展示权益发放历史记录
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
  listBenefitGrants,
  type BenefitGrant,
  type BenefitGrantQueryParams,
} from '@/services/benefit';

const { RangePicker } = DatePicker;

/**
 * 发放状态显示配置
 */
const GRANT_STATUS_MAP: Record<string, { text: string; color: string }> = {
  PENDING: { text: '待发放', color: 'processing' },
  GRANTED: { text: '已发放', color: 'success' },
  FAILED: { text: '发放失败', color: 'error' },
  EXPIRED: { text: '已过期', color: 'default' },
};

/**
 * 来源类型显示
 */
const SOURCE_TYPE_MAP: Record<string, string> = {
  BADGE_REDEMPTION: '徽章兑换',
  AUTO_GRANT: '自动发放',
  MANUAL: '手动发放',
  CAMPAIGN: '活动赠送',
};

const BenefitGrantsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();

  // 查询参数
  const [queryParams, setQueryParams] = useState<BenefitGrantQueryParams>({
    page: 1,
    pageSize: 20,
  });

  // 获取发放记录列表
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['benefitGrants', queryParams],
    queryFn: () => listBenefitGrants(queryParams),
  });

  /**
   * 表格列定义
   */
  const columns: ProColumns<BenefitGrant>[] = [
    {
      title: '发放单号',
      dataIndex: 'grantNo',
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
      title: '权益名称',
      dataIndex: 'benefitName',
      width: 150,
      ellipsis: true,
      search: false,
    },
    {
      title: '权益类型',
      dataIndex: 'benefitType',
      width: 100,
      search: false,
      render: (_, record) => {
        const typeMap: Record<string, { text: string; color: string }> = {
          POINTS: { text: '积分', color: 'blue' },
          COUPON: { text: '优惠券', color: 'orange' },
          PHYSICAL: { text: '实物', color: 'green' },
          VIRTUAL: { text: '虚拟物品', color: 'purple' },
          THIRD_PARTY: { text: '第三方', color: 'cyan' },
        };
        const config = typeMap[record.benefitType];
        return <Tag color={config?.color}>{config?.text || record.benefitType}</Tag>;
      },
    },
    {
      title: '来源',
      dataIndex: 'sourceType',
      width: 100,
      search: false,
      render: (_, record) => SOURCE_TYPE_MAP[record.sourceType] || record.sourceType,
    },
    {
      title: '数量',
      dataIndex: 'quantity',
      width: 80,
      search: false,
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (_, record) => {
        const config = GRANT_STATUS_MAP[record.status];
        return <Tag color={config?.color}>{config?.text || record.status}</Tag>;
      },
      renderFormItem: () => (
        <Select
          placeholder="选择状态"
          allowClear
          options={Object.entries(GRANT_STATUS_MAP).map(([value, { text }]) => ({
            value,
            label: text,
          }))}
        />
      ),
    },
    {
      title: '发放时间',
      dataIndex: 'grantedAt',
      width: 170,
      search: false,
      render: (_, record) => (record.grantedAt ? formatDate(record.grantedAt) : '-'),
    },
    {
      title: '过期时间',
      dataIndex: 'expiresAt',
      width: 170,
      search: false,
      render: (_, record) => (record.expiresAt ? formatDate(record.expiresAt) : '永久有效'),
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
  ];

  return (
    <PageContainer title="权益发放记录">
      <ProTable<BenefitGrant>
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
        scroll={{ x: 1400 }}
      />
    </PageContainer>
  );
};

export default BenefitGrantsPage;

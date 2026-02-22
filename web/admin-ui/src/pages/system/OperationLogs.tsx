/**
 * 操作日志页面
 *
 * 展示系统审计日志，支持按操作模块、动作、操作人、时间范围和目标资源筛选。
 * 所有管理后台的变更操作（CRUD/发放/取消等）均被记录。
 */

import React, { useRef, useState } from 'react';
import { Tag, Typography } from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import { useQuery } from '@tanstack/react-query';
import { listOperationLogs, type OperationLog, type OperationLogQueryParams } from '@/services/operationLog';
import { formatDate } from '@/utils/format';

const { Text } = Typography;

/**
 * 操作模块颜色和文本映射
 *
 * 用于 Tag 组件展示，颜色区分便于快速识别操作范围
 */
const MODULE_CONFIG: Record<string, { color: string; text: string }> = {
  badge: { color: 'blue', text: '徽章' },
  category: { color: 'cyan', text: '分类' },
  series: { color: 'geekblue', text: '系列' },
  rule: { color: 'purple', text: '规则' },
  grant: { color: 'green', text: '发放' },
  revoke: { color: 'red', text: '撤销' },
  benefit: { color: 'orange', text: '权益' },
  redemption: { color: 'gold', text: '兑换' },
  notification: { color: 'lime', text: '通知' },
  system: { color: 'default', text: '系统' },
  asset: { color: 'magenta', text: '素材' },
};

/**
 * 操作动作文本映射
 */
const ACTION_TEXT: Record<string, string> = {
  create: '创建',
  update: '更新',
  delete: '删除',
  publish: '发布',
  offline: '下线',
  archive: '归档',
  grant: '发放',
  revoke: '撤销',
  redeem: '兑换',
  enable: '启用',
  disable: '禁用',
};

const OperationLogsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [searchParams, setSearchParams] = useState<OperationLogQueryParams>({
    page: 1,
    pageSize: 20,
  });

  const { data, isLoading, refetch } = useQuery({
    queryKey: ['operationLogs', searchParams],
    queryFn: () => listOperationLogs(searchParams),
  });

  const columns: ProColumns<OperationLog>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
    },
    {
      title: '操作模块',
      dataIndex: 'module',
      width: 100,
      valueType: 'select',
      valueEnum: Object.fromEntries(
        Object.entries(MODULE_CONFIG).map(([key, cfg]) => [key, { text: cfg.text }])
      ),
      render: (_, record) => {
        const cfg = MODULE_CONFIG[record.module];
        return cfg ? (
          <Tag color={cfg.color}>{cfg.text}</Tag>
        ) : (
          <Tag>{record.module}</Tag>
        );
      },
    },
    {
      title: '操作动作',
      dataIndex: 'action',
      width: 100,
      valueType: 'select',
      valueEnum: Object.fromEntries(
        Object.entries(ACTION_TEXT).map(([key, text]) => [key, { text }])
      ),
      render: (_, record) => ACTION_TEXT[record.action] || record.action,
    },
    {
      title: '操作人',
      dataIndex: 'operatorId',
      width: 140,
      render: (_, record) => (
        <span>
          {record.operatorName || record.operatorId}
          {record.operatorName && (
            <Text type="secondary" style={{ fontSize: 12, marginLeft: 4 }}>
              ({record.operatorId})
            </Text>
          )}
        </span>
      ),
    },
    {
      title: '目标类型',
      dataIndex: 'targetType',
      width: 100,
      search: false,
      render: (_, record) => record.targetType || '-',
    },
    {
      title: '目标 ID',
      dataIndex: 'targetId',
      width: 100,
      search: false,
      render: (_, record) => record.targetId || '-',
    },
    {
      title: 'IP 地址',
      dataIndex: 'ipAddress',
      width: 140,
      search: false,
      render: (_, record) => record.ipAddress || '-',
    },
    {
      title: '操作时间',
      dataIndex: 'createdAt',
      width: 170,
      valueType: 'dateTimeRange',
      search: {
        transform: (value) => ({
          startTime: value[0],
          endTime: value[1],
        }),
      },
      render: (_, record) => formatDate(record.createdAt),
    },
    {
      title: '变更详情',
      key: 'detail',
      width: 120,
      search: false,
      render: (_, record) => {
        const hasChange = record.beforeData || record.afterData;
        if (!hasChange) return <Text type="secondary">-</Text>;
        return (
          <Text
            type="secondary"
            style={{ fontSize: 12 }}
            ellipsis={{ tooltip: JSON.stringify(record.afterData, null, 2) }}
          >
            {record.afterData ? JSON.stringify(record.afterData).slice(0, 50) : '有变更'}
          </Text>
        );
      },
    },
  ];

  return (
    <PageContainer title="操作日志">
      <ProTable<OperationLog>
        actionRef={actionRef}
        rowKey="id"
        columns={columns}
        dataSource={data?.items}
        loading={isLoading}
        pagination={{
          current: searchParams.page,
          pageSize: searchParams.pageSize,
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
          const newParams: OperationLogQueryParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            module: params.module as string | undefined,
            action: params.action as string | undefined,
            operatorId: params.operatorId as string | undefined,
            startTime: params.startTime as string | undefined,
            endTime: params.endTime as string | undefined,
          };
          setSearchParams(newParams);
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1200 }}
      />
    </PageContainer>
  );
};

export default OperationLogsPage;

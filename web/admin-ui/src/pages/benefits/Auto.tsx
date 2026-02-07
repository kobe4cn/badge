/**
 * 自动权益管理页面
 *
 * 展示自动权益发放记录和评估日志
 * 支持查询、筛选和失败记录重试
 */

import React, { useState, useRef } from 'react';
import { Card, Tabs, Button, Tag, Space, message, Popconfirm, Typography } from 'antd';
import {
  ReloadOutlined,
  RedoOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  SyncOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import { PageContainer, ProTable, type ActionType } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  listAutoBenefitGrants,
  listEvaluationLogs,
  retryAutoGrant,
  type AutoBenefitGrant,
  type EvaluationLog,
  type AutoBenefitGrantParams,
  type EvaluationLogParams,
} from '@/services/auto-benefit';
import { formatDateTime } from '@/utils/format';

const { Text } = Typography;

/**
 * 状态标签配置
 */
const statusConfig: Record<string, { color: string; icon: React.ReactNode; text: string }> = {
  PENDING: {
    color: 'orange',
    icon: <ClockCircleOutlined />,
    text: '待处理',
  },
  PROCESSING: {
    color: 'processing',
    icon: <SyncOutlined spin />,
    text: '处理中',
  },
  SUCCESS: {
    color: 'success',
    icon: <CheckCircleOutlined />,
    text: '成功',
  },
  FAILED: {
    color: 'error',
    icon: <CloseCircleOutlined />,
    text: '失败',
  },
  SKIPPED: {
    color: 'default',
    icon: <ExclamationCircleOutlined />,
    text: '已跳过',
  },
};

/**
 * 自动权益管理页面组件
 */
const AutoBenefitsPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState('grants');
  const grantsActionRef = useRef<ActionType>();
  const logsActionRef = useRef<ActionType>();
  const queryClient = useQueryClient();

  // 重试 mutation
  const retryMutation = useMutation({
    mutationFn: retryAutoGrant,
    onSuccess: () => {
      message.success('已触发重试');
      queryClient.invalidateQueries({ queryKey: ['autoBenefitGrants'] });
      grantsActionRef.current?.reload();
    },
    onError: () => {
      message.error('重试失败');
    },
  });

  /**
   * 发放记录表格列
   */
  const grantColumns: ProColumns<AutoBenefitGrant>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 80,
      search: false,
    },
    {
      title: '用户 ID',
      dataIndex: 'userId',
      width: 150,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '规则',
      key: 'rule',
      width: 150,
      search: false,
      render: (_, record) => (
        <Space direction="vertical" size={0}>
          <Text>{record.ruleName || '-'}</Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            ID: {record.ruleId}
          </Text>
        </Space>
      ),
    },
    {
      title: '规则 ID',
      dataIndex: 'ruleId',
      hideInTable: true,
    },
    {
      title: '触发徽章',
      key: 'triggerBadge',
      width: 150,
      search: false,
      render: (_, record) => (
        <Tag color="blue">{record.triggerBadgeName || `ID: ${record.triggerBadgeId}`}</Tag>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 120,
      valueType: 'select',
      valueEnum: {
        PENDING: { text: '待处理', status: 'Warning' },
        PROCESSING: { text: '处理中', status: 'Processing' },
        SUCCESS: { text: '成功', status: 'Success' },
        FAILED: { text: '失败', status: 'Error' },
        SKIPPED: { text: '已跳过', status: 'Default' },
      },
      render: (_, record) => {
        const config = statusConfig[record.status] || statusConfig.PENDING;
        return (
          <Tag color={config.color} icon={config.icon}>
            {config.text}
          </Tag>
        );
      },
    },
    {
      title: '错误信息',
      dataIndex: 'errorMessage',
      width: 200,
      ellipsis: true,
      search: false,
      render: (_, record) =>
        record.errorMessage ? (
          <Text type="danger">{record.errorMessage}</Text>
        ) : (
          '-'
        ),
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      valueType: 'dateTimeRange',
      width: 180,
      render: (_, record) => formatDateTime(record.createdAt),
      search: {
        transform: (value) => {
          if (value && value.length === 2) {
            return { startTime: value[0], endTime: value[1] };
          }
          return {};
        },
      },
    },
    {
      title: '完成时间',
      dataIndex: 'completedAt',
      width: 180,
      search: false,
      render: (_, record) => (record.completedAt ? formatDateTime(record.completedAt) : '-'),
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      search: false,
      render: (_, record) => {
        if (record.status === 'FAILED') {
          return (
            <Popconfirm
              title="确认重试"
              description="将重置为待处理状态，后台会自动重新处理"
              onConfirm={() => retryMutation.mutate(record.id)}
              okText="确认"
              cancelText="取消"
            >
              <Button
                type="link"
                size="small"
                icon={<RedoOutlined />}
                loading={retryMutation.isPending}
              >
                重试
              </Button>
            </Popconfirm>
          );
        }
        return '-';
      },
    },
  ];

  /**
   * 评估日志表格列
   */
  const logColumns: ProColumns<EvaluationLog>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 80,
      search: false,
    },
    {
      title: '用户 ID',
      dataIndex: 'userId',
      width: 150,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '触发徽章',
      key: 'triggerBadge',
      width: 150,
      search: false,
      render: (_, record) => (
        <Tag color="blue">{record.triggerBadgeName || `ID: ${record.triggerBadgeId}`}</Tag>
      ),
    },
    {
      title: '触发徽章 ID',
      dataIndex: 'triggerBadgeId',
      hideInTable: true,
    },
    {
      title: '评估规则数',
      dataIndex: 'rulesEvaluated',
      width: 100,
      search: false,
    },
    {
      title: '匹配规则数',
      dataIndex: 'rulesMatched',
      width: 100,
      search: false,
      render: (_, record) => (
        <Text type={record.rulesMatched > 0 ? 'success' : undefined}>
          {record.rulesMatched}
        </Text>
      ),
    },
    {
      title: '创建发放数',
      dataIndex: 'grantsCreated',
      width: 100,
      search: false,
      render: (_, record) => (
        <Text type={record.grantsCreated > 0 ? 'success' : undefined}>
          {record.grantsCreated}
        </Text>
      ),
    },
    {
      title: '耗时',
      dataIndex: 'durationMs',
      width: 100,
      search: false,
      render: (_, record) => `${record.durationMs} ms`,
    },
    {
      title: '评估时间',
      dataIndex: 'createdAt',
      valueType: 'dateTimeRange',
      width: 180,
      render: (_, record) => formatDateTime(record.createdAt),
      search: {
        transform: (value) => {
          if (value && value.length === 2) {
            return { startTime: value[0], endTime: value[1] };
          }
          return {};
        },
      },
    },
  ];

  /**
   * Tab 内容渲染
   */
  const tabItems = [
    {
      key: 'grants',
      label: '发放记录',
      children: (
        <ProTable<AutoBenefitGrant, AutoBenefitGrantParams>
          actionRef={grantsActionRef}
          columns={grantColumns}
          rowKey="id"
          request={async (params) => {
            const { current, pageSize, userId, ruleId, status, ...rest } = params;
            try {
              const response = await listAutoBenefitGrants({
                page: current || 1,
                pageSize: pageSize || 20,
                userId,
                ruleId: ruleId ? Number(ruleId) : undefined,
                status,
                ...rest,
              });
              return {
                data: response.items,
                total: response.total,
                success: true,
              };
            } catch {
              return { data: [], total: 0, success: false };
            }
          }}
          pagination={{
            defaultPageSize: 20,
            showSizeChanger: true,
            showQuickJumper: true,
          }}
          search={{
            labelWidth: 'auto',
            defaultCollapsed: false,
          }}
          toolBarRender={() => [
            <Button
              key="refresh"
              icon={<ReloadOutlined />}
              onClick={() => grantsActionRef.current?.reload()}
            >
              刷新
            </Button>,
          ]}
          scroll={{ x: 1300 }}
          dateFormatter="string"
        />
      ),
    },
    {
      key: 'logs',
      label: '评估日志',
      children: (
        <ProTable<EvaluationLog, EvaluationLogParams>
          actionRef={logsActionRef}
          columns={logColumns}
          rowKey="id"
          request={async (params) => {
            const { current, pageSize, userId, triggerBadgeId, ...rest } = params;
            try {
              const response = await listEvaluationLogs({
                page: current || 1,
                pageSize: pageSize || 20,
                userId,
                triggerBadgeId: triggerBadgeId ? Number(triggerBadgeId) : undefined,
                ...rest,
              });
              return {
                data: response.items,
                total: response.total,
                success: true,
              };
            } catch {
              return { data: [], total: 0, success: false };
            }
          }}
          pagination={{
            defaultPageSize: 20,
            showSizeChanger: true,
            showQuickJumper: true,
          }}
          search={{
            labelWidth: 'auto',
            defaultCollapsed: false,
          }}
          toolBarRender={() => [
            <Button
              key="refresh"
              icon={<ReloadOutlined />}
              onClick={() => logsActionRef.current?.reload()}
            >
              刷新
            </Button>,
          ]}
          scroll={{ x: 1100 }}
          dateFormatter="string"
        />
      ),
    },
  ];

  return (
    <PageContainer title="自动权益">
      <Card>
        <Tabs activeKey={activeTab} onChange={setActiveTab} items={tabItems} />
      </Card>
    </PageContainer>
  );
};

export default AutoBenefitsPage;

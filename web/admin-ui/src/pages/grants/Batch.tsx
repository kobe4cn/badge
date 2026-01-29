/**
 * 批量任务页面
 *
 * 管理批量徽章发放任务，支持：
 * - 任务列表展示与筛选
 * - 新建批量任务
 * - 查看任务详情
 * - 取消进行中的任务
 * - 下载已完成任务的结果
 */

import React, { useState, useCallback, useRef } from 'react';
import { Button, Space, Tag, Popconfirm, message, Tooltip } from 'antd';
import {
  PlusOutlined,
  ReloadOutlined,
  EyeOutlined,
  StopOutlined,
  DownloadOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { PageContainer, ProTable, type ActionType } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { getBatchTasks, downloadBatchResult } from '@/services/grant';
import { useCancelBatchTask } from '@/hooks/useGrant';
import { formatDateTime } from '@/utils/format';
import CreateBatchTaskModal from './components/CreateBatchTaskModal';
import BatchTaskDetail from './components/BatchTaskDetail';
import type { BatchTask, BatchTaskStatus, BatchTaskQueryParams } from '@/types';

/**
 * 获取任务状态标签配置
 */
const getStatusConfig = (status: BatchTaskStatus) => {
  const configs: Record<
    BatchTaskStatus,
    { color: string; icon: React.ReactNode; text: string }
  > = {
    pending: {
      color: 'default',
      icon: <ClockCircleOutlined />,
      text: '等待中',
    },
    processing: {
      color: 'processing',
      icon: <SyncOutlined spin />,
      text: '处理中',
    },
    completed: {
      color: 'success',
      icon: <CheckCircleOutlined />,
      text: '已完成',
    },
    failed: {
      color: 'error',
      icon: <CloseCircleOutlined />,
      text: '失败',
    },
    cancelled: {
      color: 'warning',
      icon: <StopOutlined />,
      text: '已取消',
    },
  };
  return configs[status] || configs.pending;
};

/**
 * 批量任务页面组件
 */
const BatchGrantPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null);

  const { mutateAsync: cancelTask, isPending: isCancelling } = useCancelBatchTask();

  /**
   * 打开详情弹窗
   */
  const handleViewDetail = useCallback((taskId: number) => {
    setSelectedTaskId(taskId);
    setDetailModalOpen(true);
  }, []);

  /**
   * 取消任务
   */
  const handleCancelTask = useCallback(
    async (taskId: number) => {
      try {
        await cancelTask(taskId);
        actionRef.current?.reload();
      } catch {
        // 错误已在 hook 中处理
      }
    },
    [cancelTask]
  );

  /**
   * 下载结果
   */
  const handleDownloadResult = useCallback(async (task: BatchTask) => {
    try {
      const blob = await downloadBatchResult(task.id);
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `batch-task-${task.id}-result.csv`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      message.success('下载成功');
    } catch {
      message.error('下载失败');
    }
  }, []);

  /**
   * 表格列定义
   */
  const columns: ProColumns<BatchTask>[] = [
    {
      title: '任务 ID',
      dataIndex: 'id',
      width: 80,
      search: false,
    },
    {
      title: '任务名称',
      dataIndex: 'name',
      ellipsis: true,
      render: (_, record) => record.name || '-',
    },
    {
      title: '徽章',
      dataIndex: 'badgeName',
      ellipsis: true,
      search: false,
      render: (_, record) => record.badgeName || '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      valueType: 'select',
      valueEnum: {
        pending: { text: '等待中', status: 'Default' },
        processing: { text: '处理中', status: 'Processing' },
        completed: { text: '已完成', status: 'Success' },
        failed: { text: '失败', status: 'Error' },
        cancelled: { text: '已取消', status: 'Warning' },
      },
      render: (_, record) => {
        const config = getStatusConfig(record.status);
        return (
          <Tag color={config.color} icon={config.icon}>
            {config.text}
          </Tag>
        );
      },
    },
    {
      title: '进度',
      dataIndex: 'progress',
      width: 180,
      search: false,
      render: (_, record) => {
        const { totalCount, successCount, failureCount, progress } = record;
        const processed = successCount + failureCount;
        return (
          <Space direction="vertical" size={0} style={{ lineHeight: 1.2 }}>
            <span>
              <span style={{ color: '#52c41a' }}>{successCount}</span>
              {failureCount > 0 && (
                <>
                  {' / '}
                  <span style={{ color: '#ff4d4f' }}>{failureCount}</span>
                </>
              )}
              {' / '}
              <span>{totalCount}</span>
            </span>
            <span style={{ fontSize: 12, color: '#8c8c8c' }}>
              {progress}% ({processed}/{totalCount})
            </span>
          </Space>
        );
      },
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
            return {
              startTime: value[0],
              endTime: value[1],
            };
          }
          return {};
        },
      },
    },
    {
      title: '创建人',
      dataIndex: 'createdBy',
      width: 100,
      ellipsis: true,
      search: false,
    },
    {
      title: '操作',
      key: 'action',
      width: 150,
      fixed: 'right',
      search: false,
      render: (_, record) => {
        const isRunning = record.status === 'pending' || record.status === 'processing';
        const isCompleted = record.status === 'completed';
        const hasResult = !!record.resultFileUrl;

        return (
          <Space size={0}>
            <Tooltip title="查看详情">
              <Button
                type="link"
                size="small"
                icon={<EyeOutlined />}
                onClick={() => handleViewDetail(record.id)}
              />
            </Tooltip>

            {isRunning && (
              <Popconfirm
                title="确认取消"
                description="确定要取消此任务吗？已处理的记录不会回滚。"
                onConfirm={() => handleCancelTask(record.id)}
                okText="确认"
                cancelText="取消"
              >
                <Tooltip title="取消任务">
                  <Button
                    type="link"
                    size="small"
                    danger
                    icon={<StopOutlined />}
                    loading={isCancelling}
                  />
                </Tooltip>
              </Popconfirm>
            )}

            {isCompleted && hasResult && (
              <Tooltip title="下载结果">
                <Button
                  type="link"
                  size="small"
                  icon={<DownloadOutlined />}
                  onClick={() => handleDownloadResult(record)}
                />
              </Tooltip>
            )}
          </Space>
        );
      },
    },
  ];

  return (
    <PageContainer
      title="批量任务"
      extra={
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setCreateModalOpen(true)}
        >
          新建任务
        </Button>
      }
    >
      <ProTable<BatchTask, BatchTaskQueryParams>
        actionRef={actionRef}
        columns={columns}
        rowKey="id"
        request={async (params) => {
          const { current, pageSize, status, ...rest } = params;
          const response = await getBatchTasks({
            page: current || 1,
            pageSize: pageSize || 20,
            status: status as BatchTaskStatus,
            ...rest,
          });
          return {
            data: response.items,
            total: response.total,
            success: true,
          };
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
            onClick={() => actionRef.current?.reload()}
          >
            刷新
          </Button>,
        ]}
        scroll={{ x: 1100 }}
        polling={5000}
        dateFormatter="string"
      />

      {/* 新建任务弹窗 */}
      <CreateBatchTaskModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onSuccess={() => actionRef.current?.reload()}
      />

      {/* 任务详情弹窗 */}
      <BatchTaskDetail
        open={detailModalOpen}
        taskId={selectedTaskId}
        onClose={() => {
          setDetailModalOpen(false);
          setSelectedTaskId(null);
        }}
      />
    </PageContainer>
  );
};

export default BatchGrantPage;

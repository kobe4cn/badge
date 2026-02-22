/**
 * 异常处置管理页面
 *
 * 展示系统异常列表（如发放失败、批量任务错误等），
 * 支持手动重试、忽略和标记已处理操作。
 *
 * 基于现有的 BatchTask 和 BatchTaskFailure API 构建，
 * 对应需求 §11（系统日志和告警 — 异常请求处置）
 */

import React, { useState, useCallback } from 'react';
import {
  Card,
  Table,
  Tag,
  Button,
  Space,
  Tooltip,
  Tabs,
  Modal,
  Statistic,
  Row,
  Col,
  message,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  ReloadOutlined,
  ExclamationCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
  DeleteOutlined,
  EyeOutlined,
  DownloadOutlined,
} from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import type { ColumnsType } from 'antd/es/table';
import { getList, get, post } from '@/services/api';
import type {
  BatchTask,
  BatchTaskFailure,
  BatchTaskStatus,
  RetryResult,
} from '@/types/grant';
import type { PaginatedResponse } from '@/types';
import { formatDate } from '@/utils/format';

/**
 * 任务状态配色与文本
 */
const TASK_STATUS_CONFIG: Record<BatchTaskStatus, { color: string; text: string }> = {
  pending: { color: 'default', text: '等待中' },
  processing: { color: 'processing', text: '处理中' },
  completed: { color: 'success', text: '已完成' },
  failed: { color: 'error', text: '失败' },
  cancelled: { color: 'warning', text: '已取消' },
};

/**
 * 重试状态配色
 */
const RETRY_STATUS_CONFIG: Record<string, { color: string; text: string }> = {
  PENDING: { color: 'default', text: '待重试' },
  RETRYING: { color: 'processing', text: '重试中' },
  SUCCESS: { color: 'success', text: '已成功' },
  EXHAUSTED: { color: 'error', text: '已耗尽' },
};

const ExceptionsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState('failed');
  const [failureDetailTask, setFailureDetailTask] = useState<BatchTask | null>(null);

  // 查询失败/异常任务列表
  const {
    data: failedTasksData,
    isLoading: failedLoading,
  } = useQuery({
    queryKey: ['exceptions', 'failedTasks', activeTab],
    queryFn: () =>
      getList<BatchTask>('/admin/tasks', {
        status: activeTab === 'all' ? undefined : 'failed',
        page: 1,
        pageSize: 50,
      }),
    refetchInterval: 30 * 1000,
  });

  // 查询异常统计
  const {
    data: statsData,
  } = useQuery({
    queryKey: ['exceptions', 'stats'],
    queryFn: async () => {
      const [failed, processing] = await Promise.all([
        getList<BatchTask>('/admin/tasks', { status: 'failed', page: 1, pageSize: 1 }),
        getList<BatchTask>('/admin/tasks', { status: 'processing', page: 1, pageSize: 1 }),
      ]);
      return {
        failedCount: failed.total,
        processingCount: processing.total,
      };
    },
  });

  // 查询任务失败明细
  const {
    data: failureDetails,
    isLoading: detailsLoading,
  } = useQuery({
    queryKey: ['exceptions', 'failures', failureDetailTask?.id],
    queryFn: () =>
      get<PaginatedResponse<BatchTaskFailure>>(
        `/admin/tasks/${failureDetailTask!.id}/failures`
      ),
    enabled: !!failureDetailTask,
  });

  // 重试失败条目
  const retryMutation = useMutation({
    mutationFn: (taskId: number) =>
      post<RetryResult>(`/admin/tasks/${taskId}/retry`),
    onSuccess: (result) => {
      message.success(result?.message || '重试任务已提交');
      queryClient.invalidateQueries({ queryKey: ['exceptions'] });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '重试失败');
    },
  });

  // 取消任务
  const cancelMutation = useMutation({
    mutationFn: (taskId: number) =>
      post(`/admin/tasks/${taskId}/cancel`),
    onSuccess: () => {
      message.success('任务已取消');
      queryClient.invalidateQueries({ queryKey: ['exceptions'] });
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '取消失败');
    },
  });

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['exceptions'] });
  }, [queryClient]);

  const handleRetry = useCallback((taskId: number) => {
    Modal.confirm({
      title: '确认重试',
      icon: <ExclamationCircleOutlined />,
      content: '将重试该任务中所有失败的条目，确认继续？',
      onOk: () => retryMutation.mutate(taskId),
    });
  }, [retryMutation]);

  const handleCancel = useCallback((taskId: number) => {
    Modal.confirm({
      title: '确认取消',
      icon: <ExclamationCircleOutlined />,
      content: '取消后将不再处理该任务中剩余条目，确认继续？',
      onOk: () => cancelMutation.mutate(taskId),
    });
  }, [cancelMutation]);

  const handleDownloadFailures = useCallback((taskId: number) => {
    // 直接打开下载链接
    const baseUrl = import.meta.env.VITE_API_BASE_URL || '/api';
    window.open(`${baseUrl}/admin/tasks/${taskId}/failures/download`, '_blank');
  }, []);

  // 任务列表表格列
  const taskColumns: ColumnsType<BatchTask> = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 60,
    },
    {
      title: '任务名称',
      dataIndex: 'name',
      key: 'name',
      width: 200,
      render: (name: string, record: BatchTask) => name || `${record.taskType}-${record.id}`,
    },
    {
      title: '任务类型',
      dataIndex: 'taskType',
      key: 'taskType',
      width: 110,
      render: (type: string) => {
        const map: Record<string, string> = {
          batch_grant: '批量发放',
          batch_revoke: '批量撤销',
          data_export: '数据导出',
        };
        return map[type] || type;
      },
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: BatchTaskStatus) => {
        const config = TASK_STATUS_CONFIG[status];
        return <Tag color={config.color}>{config.text}</Tag>;
      },
    },
    {
      title: '进度',
      key: 'progress',
      width: 160,
      render: (_: unknown, record: BatchTask) => (
        <Space direction="vertical" size={0}>
          <span>
            总计: {record.totalCount} | 成功: <span style={{ color: '#52c41a' }}>{record.successCount}</span>
          </span>
          <span>
            失败: <span style={{ color: '#ff4d4f' }}>{record.failureCount}</span>
          </span>
        </Space>
      ),
    },
    {
      title: '错误信息',
      dataIndex: 'errorMessage',
      key: 'errorMessage',
      ellipsis: true,
      render: (msg: string) => msg || '-',
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      key: 'createdAt',
      width: 170,
      render: (v: string) => formatDate(v),
    },
    {
      title: '操作',
      key: 'action',
      width: 220,
      render: (_: unknown, record: BatchTask) => (
        <Space size={4}>
          {record.failureCount > 0 && (
            <Tooltip title="查看失败明细">
              <Button
                type="link"
                size="small"
                icon={<EyeOutlined />}
                onClick={() => setFailureDetailTask(record)}
              >
                明细
              </Button>
            </Tooltip>
          )}
          {record.status === 'failed' && (
            <Tooltip title="重试失败条目">
              <Button
                type="link"
                size="small"
                icon={<SyncOutlined />}
                onClick={() => handleRetry(record.id)}
              >
                重试
              </Button>
            </Tooltip>
          )}
          {(record.status === 'pending' || record.status === 'processing') && (
            <Tooltip title="取消任务">
              <Button
                type="link"
                size="small"
                danger
                icon={<DeleteOutlined />}
                onClick={() => handleCancel(record.id)}
              >
                取消
              </Button>
            </Tooltip>
          )}
          {record.failureCount > 0 && (
            <Tooltip title="下载失败报告">
              <Button
                type="link"
                size="small"
                icon={<DownloadOutlined />}
                onClick={() => handleDownloadFailures(record.id)}
              >
                下载
              </Button>
            </Tooltip>
          )}
        </Space>
      ),
    },
  ];

  // 失败明细表格列
  const failureColumns: ColumnsType<BatchTaskFailure> = [
    {
      title: '行号',
      dataIndex: 'rowNumber',
      key: 'rowNumber',
      width: 70,
    },
    {
      title: '用户 ID',
      dataIndex: 'userId',
      key: 'userId',
      width: 160,
      render: (id: string) => id || '-',
    },
    {
      title: '错误码',
      dataIndex: 'errorCode',
      key: 'errorCode',
      width: 140,
      render: (code: string) => <Tag color="error">{code}</Tag>,
    },
    {
      title: '错误信息',
      dataIndex: 'errorMessage',
      key: 'errorMessage',
      ellipsis: true,
    },
    {
      title: '重试次数',
      dataIndex: 'retryCount',
      key: 'retryCount',
      width: 90,
    },
    {
      title: '重试状态',
      dataIndex: 'retryStatus',
      key: 'retryStatus',
      width: 100,
      render: (status: string) => {
        const config = RETRY_STATUS_CONFIG[status] || { color: 'default', text: status };
        return <Tag color={config.color}>{config.text}</Tag>;
      },
    },
    {
      title: '上次重试',
      dataIndex: 'lastRetryAt',
      key: 'lastRetryAt',
      width: 170,
      render: (v: string) => v ? formatDate(v) : '-',
    },
  ];

  return (
    <PageContainer
      title="异常处置"
      extra={
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
          刷新
        </Button>
      }
    >
      {/* 异常概览统计 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card hoverable>
            <Statistic
              title={
                <span>
                  <CloseCircleOutlined style={{ color: '#ff4d4f', marginRight: 8 }} />
                  失败任务
                </span>
              }
              value={statsData?.failedCount ?? 0}
              valueStyle={{ color: '#ff4d4f' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card hoverable>
            <Statistic
              title={
                <span>
                  <SyncOutlined style={{ color: '#1677ff', marginRight: 8 }} />
                  处理中任务
                </span>
              }
              value={statsData?.processingCount ?? 0}
              valueStyle={{ color: '#1677ff' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card hoverable>
            <Statistic
              title={
                <span>
                  <CheckCircleOutlined style={{ color: '#52c41a', marginRight: 8 }} />
                  系统状态
                </span>
              }
              value={(statsData?.failedCount ?? 0) === 0 ? '正常' : '有异常'}
              valueStyle={{
                color: (statsData?.failedCount ?? 0) === 0 ? '#52c41a' : '#ff4d4f',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 异常任务列表 */}
      <Card>
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            { key: 'failed', label: '失败任务' },
            { key: 'all', label: '全部任务' },
          ]}
        />
        <Table<BatchTask>
          columns={taskColumns}
          dataSource={failedTasksData?.items || []}
          rowKey="id"
          loading={failedLoading}
          pagination={{
            total: failedTasksData?.total || 0,
            pageSize: 20,
            showTotal: (total) => `共 ${total} 条`,
          }}
          size="middle"
          locale={{ emptyText: '暂无异常任务' }}
        />
      </Card>

      {/* 失败明细弹窗 */}
      <Modal
        title={`任务 #${failureDetailTask?.id} — 失败明细`}
        open={!!failureDetailTask}
        onCancel={() => setFailureDetailTask(null)}
        footer={null}
        width={900}
      >
        <Table<BatchTaskFailure>
          columns={failureColumns}
          dataSource={(failureDetails as PaginatedResponse<BatchTaskFailure>)?.items || []}
          rowKey="id"
          loading={detailsLoading}
          pagination={{
            pageSize: 10,
            showTotal: (total) => `共 ${total} 条`,
          }}
          size="small"
        />
      </Modal>
    </PageContainer>
  );
};

export default ExceptionsPage;

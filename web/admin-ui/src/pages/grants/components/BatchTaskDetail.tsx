/**
 * 批量任务详情弹窗
 *
 * 显示任务的详细信息、执行进度和失败明细
 * 支持进行中任务的实时进度轮询
 */

import React from 'react';
import {
  Modal,
  Descriptions,
  Progress,
  Table,
  Tag,
  Space,
  Typography,
  Spin,
  Empty,
  Card,
  Row,
  Col,
  Statistic,
  Avatar,
  Divider,
} from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  SyncOutlined,
  StopOutlined,
  ExclamationCircleOutlined,
  UserOutlined,
} from '@ant-design/icons';
import { useBatchTaskDetail, useBatchTaskFailures } from '@/hooks/useGrant';
import { formatDateTime } from '@/utils/format';
import type { BatchTaskStatus, BatchTaskFailure } from '@/types';
import type { ColumnsType } from 'antd/es/table';

const { Text } = Typography;

interface BatchTaskDetailProps {
  open: boolean;
  taskId: number | null;
  onClose: () => void;
}

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
 * 批量任务详情弹窗组件
 */
const BatchTaskDetail: React.FC<BatchTaskDetailProps> = ({
  open,
  taskId,
  onClose,
}) => {
  // 查询任务详情，进行中的任务自动轮询
  const {
    data: task,
    isLoading: taskLoading,
    isError: taskError,
  } = useBatchTaskDetail(taskId || 0, open && !!taskId);

  // 查询失败明细
  const {
    data: failuresData,
    isLoading: failuresLoading,
  } = useBatchTaskFailures(
    taskId || 0,
    { page: 1, pageSize: 50 },
    open && !!taskId && !!task && task.failureCount > 0
  );

  /**
   * 渲染加载状态
   */
  if (taskLoading) {
    return (
      <Modal
        title="任务详情"
        open={open}
        onCancel={onClose}
        footer={null}
        width={720}
      >
        <div style={{ textAlign: 'center', padding: 48 }}>
          <Spin size="large" />
          <p style={{ marginTop: 16 }}>加载中...</p>
        </div>
      </Modal>
    );
  }

  /**
   * 渲染错误状态
   */
  if (taskError || !task) {
    return (
      <Modal
        title="任务详情"
        open={open}
        onCancel={onClose}
        footer={null}
        width={720}
      >
        <Empty description="任务不存在或加载失败" />
      </Modal>
    );
  }

  const statusConfig = getStatusConfig(task.status);
  const isRunning = task.status === 'pending' || task.status === 'processing';

  /**
   * 根据重试状态返回标签配置
   */
  const getRetryStatusTag = (status: string) => {
    const configs: Record<string, { color: string; text: string }> = {
      PENDING: { color: 'orange', text: '待重试' },
      RETRYING: { color: 'processing', text: '重试中' },
      SUCCESS: { color: 'success', text: '重试成功' },
      EXHAUSTED: { color: 'error', text: '已耗尽' },
    };
    return configs[status] || { color: 'default', text: status };
  };

  // 失败明细表格列定义
  const failureColumns: ColumnsType<BatchTaskFailure> = [
    {
      title: '行号',
      dataIndex: 'rowNumber',
      key: 'rowNumber',
      width: 70,
    },
    {
      title: '用户',
      key: 'user',
      width: 150,
      render: (_, record) => (
        <Space>
          <Avatar size="small" icon={<UserOutlined />} />
          <Text type="secondary">{record.userId || '-'}</Text>
        </Space>
      ),
    },
    {
      title: '错误信息',
      dataIndex: 'errorMessage',
      key: 'errorMessage',
      render: (msg: string) => (
        <Text type="danger">
          <ExclamationCircleOutlined style={{ marginRight: 4 }} />
          {msg}
        </Text>
      ),
    },
    {
      title: '重试状态',
      key: 'retryStatus',
      width: 100,
      render: (_, record) => {
        const config = getRetryStatusTag(record.retryStatus);
        return (
          <Tag color={config.color}>
            {config.text}
            {record.retryCount > 0 && ` (${record.retryCount}次)`}
          </Tag>
        );
      },
    },
  ];

  return (
    <Modal
      title={
        <Space>
          <span>任务详情</span>
          <Tag color={statusConfig.color} icon={statusConfig.icon}>
            {statusConfig.text}
          </Tag>
          {isRunning && <Text type="secondary">(自动刷新中)</Text>}
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
    >
      {/* 基本信息 */}
      <Card size="small" style={{ marginBottom: 16 }}>
        <Descriptions column={2} size="small">
          <Descriptions.Item label="任务 ID">{task.id}</Descriptions.Item>
          <Descriptions.Item label="任务名称">
            {task.name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="徽章">
            {task.badgeName || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="每人数量">
            {task.quantity || 1} 个
          </Descriptions.Item>
          <Descriptions.Item label="创建人">{task.createdBy}</Descriptions.Item>
          <Descriptions.Item label="创建时间">
            {formatDateTime(task.createdAt)}
          </Descriptions.Item>
          {task.completedAt && (
            <Descriptions.Item label="完成时间" span={2}>
              {formatDateTime(task.completedAt)}
            </Descriptions.Item>
          )}
        </Descriptions>
      </Card>

      {/* 执行进度 */}
      <Card size="small" title="执行进度" style={{ marginBottom: 16 }}>
        <Progress
          percent={task.progress}
          status={
            task.status === 'failed'
              ? 'exception'
              : task.status === 'completed'
              ? 'success'
              : 'active'
          }
          strokeColor={
            task.status === 'processing'
              ? { from: '#108ee9', to: '#87d068' }
              : undefined
          }
          style={{ marginBottom: 16 }}
        />

        <Row gutter={24}>
          <Col span={8}>
            <Statistic
              title="总数"
              value={task.totalCount}
              suffix="人"
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="成功"
              value={task.successCount}
              suffix="人"
              valueStyle={{ color: '#52c41a' }}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="失败"
              value={task.failureCount}
              suffix="人"
              valueStyle={{
                color: task.failureCount > 0 ? '#ff4d4f' : undefined,
              }}
            />
          </Col>
        </Row>

        {task.errorMessage && (
          <>
            <Divider style={{ margin: '16px 0' }} />
            <Text type="danger">
              <ExclamationCircleOutlined style={{ marginRight: 4 }} />
              错误信息：{task.errorMessage}
            </Text>
          </>
        )}
      </Card>

      {/* 失败明细 */}
      {task.failureCount > 0 && (
        <Card
          size="small"
          title={
            <Space>
              <CloseCircleOutlined style={{ color: '#ff4d4f' }} />
              失败明细 ({task.failureCount})
            </Space>
          }
        >
          <Table
            dataSource={failuresData?.items || []}
            columns={failureColumns}
            rowKey="id"
            loading={failuresLoading}
            pagination={
              (failuresData?.total || 0) > 50
                ? { pageSize: 50, showTotal: (total) => `共 ${total} 条` }
                : false
            }
            size="small"
            locale={{ emptyText: <Empty description="暂无失败记录" /> }}
          />
        </Card>
      )}
    </Modal>
  );
};

export default BatchTaskDetail;

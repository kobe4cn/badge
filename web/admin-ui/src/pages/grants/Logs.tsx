/**
 * 发放日志页面
 *
 * 查看徽章发放的历史记录，支持多维度筛选和导出功能
 * 包含日志列表、详情查看和 CSV 导出
 */

import React, { useState, useCallback, useRef, useMemo } from 'react';
import { Button, Space, Tag, Avatar, Tooltip, Typography } from 'antd';
import {
  ExportOutlined,
  ReloadOutlined,
  EyeOutlined,
  UserOutlined,
  TrophyOutlined,
} from '@ant-design/icons';
import { PageContainer, ProTable, type ActionType } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { getGrantLogs } from '@/services/grant';
import { useExportGrantLogs } from '@/hooks/useGrant';
import { formatDateTime } from '@/utils/format';
import LogDetailModal from './components/LogDetailModal';
import type { GrantLog, SourceType, LogAction, GrantLogQueryParams } from '@/types';

const { Text } = Typography;

/**
 * 来源类型配置
 */
const SOURCE_TYPE_CONFIG: Record<SourceType, { color: string; text: string }> = {
  MANUAL: { color: 'blue', text: '手动发放' },
  EVENT: { color: 'purple', text: '事件触发' },
  SCHEDULED: { color: 'cyan', text: '定时任务' },
  REDEMPTION: { color: 'orange', text: '兑换' },
  SYSTEM: { color: 'default', text: '系统' },
};

/**
 * 操作动作配置
 */
const ACTION_CONFIG: Record<LogAction, { color: string; text: string }> = {
  GRANT: { color: 'success', text: '发放' },
  REVOKE: { color: 'error', text: '撤回' },
  REDEEM: { color: 'warning', text: '兑换' },
  EXPIRE: { color: 'default', text: '过期' },
};

/**
 * 发放日志列表页面组件
 */
const GrantLogsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  // 存储当前筛选参数用于导出
  const [currentParams, setCurrentParams] = useState<GrantLogQueryParams>({});

  const { mutateAsync: exportLogs, isPending: isExporting } = useExportGrantLogs();

  /**
   * 打开详情弹窗
   */
  const handleViewDetail = useCallback((logId: number) => {
    setSelectedLogId(logId);
    setDetailModalOpen(true);
  }, []);

  /**
   * 导出日志
   */
  const handleExport = useCallback(async () => {
    try {
      await exportLogs(currentParams);
    } catch {
      // 错误已在 hook 中处理
    }
  }, [exportLogs, currentParams]);

  /**
   * 来源类型筛选选项
   */
  const sourceTypeValueEnum = useMemo(() => {
    return Object.entries(SOURCE_TYPE_CONFIG).reduce(
      (acc, [key, value]) => {
        acc[key] = { text: value.text };
        return acc;
      },
      {} as Record<string, { text: string }>
    );
  }, []);

  /**
   * 操作动作筛选选项
   */
  const actionValueEnum = useMemo(() => {
    return Object.entries(ACTION_CONFIG).reduce(
      (acc, [key, value]) => {
        acc[key] = { text: value.text };
        return acc;
      },
      {} as Record<string, { text: string }>
    );
  }, []);

  /**
   * 表格列定义
   */
  const columns: ProColumns<GrantLog>[] = [
    {
      title: '日志 ID',
      dataIndex: 'id',
      width: 80,
      search: false,
      fixed: 'left',
    },
    {
      title: '用户信息',
      key: 'user',
      width: 200,
      render: (_, record) => (
        <Space>
          <Avatar
            size="small"
            src={record.userAvatar}
            icon={<UserOutlined />}
          />
          <div style={{ lineHeight: 1.3 }}>
            <div>
              <Text strong>{record.userName || '-'}</Text>
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {record.userId}
              </Text>
            </div>
          </div>
        </Space>
      ),
      // 支持用户 ID 或昵称搜索
      fieldProps: {
        placeholder: '用户 ID / 昵称',
      },
      search: {
        transform: (value) => {
          // 简单判断：如果全是数字或字母则作为 userId，否则作为 userName
          if (/^[a-zA-Z0-9_-]+$/.test(value)) {
            return { userId: value };
          }
          return { userName: value };
        },
      },
    },
    {
      title: '徽章',
      key: 'badge',
      width: 180,
      render: (_, record) => (
        <Space>
          <Avatar
            size="small"
            src={record.badgeIcon}
            icon={<TrophyOutlined />}
            style={{ backgroundColor: '#f0f0f0' }}
          />
          <div style={{ lineHeight: 1.3 }}>
            <div>
              <Text>{record.badgeName || '-'}</Text>
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                ID: {record.badgeId}
              </Text>
            </div>
          </div>
        </Space>
      ),
      fieldProps: {
        placeholder: '徽章名称',
      },
      search: {
        transform: (value) => ({ badgeName: value }),
      },
    },
    {
      title: '操作',
      dataIndex: 'action',
      width: 80,
      valueType: 'select',
      valueEnum: actionValueEnum,
      render: (_, record) => {
        const config = ACTION_CONFIG[record.action];
        return <Tag color={config?.color}>{config?.text || record.action}</Tag>;
      },
    },
    {
      title: '数量',
      dataIndex: 'quantity',
      width: 80,
      search: false,
      render: (_, record) => {
        const isNegative = record.action === 'REVOKE';
        return (
          <Text style={{ color: isNegative ? '#ff4d4f' : '#52c41a' }}>
            {isNegative ? '-' : '+'}{record.quantity}
          </Text>
        );
      },
    },
    {
      title: '发放来源',
      dataIndex: 'sourceType',
      width: 100,
      valueType: 'select',
      valueEnum: sourceTypeValueEnum,
      render: (_, record) => {
        const config = SOURCE_TYPE_CONFIG[record.sourceType];
        return <Tag color={config?.color}>{config?.text || record.sourceType}</Tag>;
      },
    },
    {
      title: '操作时间',
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
      title: '操作人',
      dataIndex: 'operatorName',
      width: 100,
      ellipsis: true,
      render: (_, record) => record.operatorName || record.operatorId || '-',
      fieldProps: {
        placeholder: '操作人 ID',
      },
      search: {
        transform: (value) => ({ operatorId: value }),
      },
    },
    {
      title: '操作',
      key: 'actions',
      width: 80,
      fixed: 'right',
      search: false,
      render: (_, record) => (
        <Tooltip title="查看详情">
          <Button
            type="link"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => handleViewDetail(record.id)}
          />
        </Tooltip>
      ),
    },
  ];

  return (
    <PageContainer
      title="发放日志"
      extra={
        <Button
          icon={<ExportOutlined />}
          loading={isExporting}
          onClick={handleExport}
        >
          导出日志
        </Button>
      }
    >
      <ProTable<GrantLog, GrantLogQueryParams>
        actionRef={actionRef}
        columns={columns}
        rowKey="id"
        request={async (params) => {
          const {
            current,
            pageSize,
            action,
            sourceType,
            ...rest
          } = params;

          // 构建查询参数
          const queryParams: GrantLogQueryParams & { page?: number; pageSize?: number } = {
            page: current || 1,
            pageSize: pageSize || 20,
            ...rest,
          };

          // 处理枚举类型参数
          if (action) {
            queryParams.action = action as LogAction;
          }
          if (sourceType) {
            queryParams.sourceType = sourceType as SourceType;
          }

          // 保存当前筛选参数用于导出
          setCurrentParams(queryParams);

          const response = await getGrantLogs(queryParams);
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
          showTotal: (total) => `共 ${total} 条记录`,
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
        scroll={{ x: 1200 }}
        dateFormatter="string"
      />

      {/* 日志详情弹窗 */}
      <LogDetailModal
        open={detailModalOpen}
        logId={selectedLogId}
        onClose={() => {
          setDetailModalOpen(false);
          setSelectedLogId(null);
        }}
      />
    </PageContainer>
  );
};

export default GrantLogsPage;

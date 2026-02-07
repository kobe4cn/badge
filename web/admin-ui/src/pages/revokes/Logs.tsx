/**
 * 撤销记录页面
 *
 * 展示徽章撤销历史记录，支持筛选和导出
 */

import React, { useRef, useState } from 'react';
import { Button, Tag, message } from 'antd';
import {
  ReloadOutlined,
  DownloadOutlined,
} from '@ant-design/icons';
import { PageContainer, ProTable, type ActionType } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { getRevokeRecords, exportRevokeRecords, type RevokeLogParams } from '@/services/revoke';
import { formatDateTime } from '@/utils/format';
import type { GrantLog } from '@/types';

/**
 * 来源类型标签配置
 */
const sourceTypeConfig: Record<string, { color: string; text: string }> = {
  MANUAL: { color: 'blue', text: '手动撤销' },
  BATCH: { color: 'purple', text: '批量撤销' },
  RULE: { color: 'orange', text: '规则撤销' },
  SYSTEM: { color: 'default', text: '系统撤销' },
  EXPIRE: { color: 'red', text: '到期过期' },
  AUTO: { color: 'cyan', text: '自动取消' },
};

/**
 * 撤销记录页面组件
 */
const RevokeLogsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [exporting, setExporting] = useState(false);

  /**
   * 导出记录
   */
  const handleExport = async (params: Omit<RevokeLogParams, 'page' | 'pageSize'>) => {
    setExporting(true);
    try {
      const blob = await exportRevokeRecords(params);
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `revoke_records_${new Date().toISOString().slice(0, 10)}.csv`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      message.success('导出成功');
    } catch {
      message.error('导出失败');
    } finally {
      setExporting(false);
    }
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<GrantLog>[] = [
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
      title: '徽章',
      dataIndex: 'badgeName',
      width: 150,
      ellipsis: true,
      search: false,
    },
    {
      title: '徽章 ID',
      dataIndex: 'badgeId',
      width: 100,
      hideInTable: true,
    },
    {
      title: '数量',
      dataIndex: 'quantity',
      width: 80,
      search: false,
      render: (_, record) => (
        <Tag color="red">-{Math.abs(record.quantity)}</Tag>
      ),
    },
    {
      title: '来源类型',
      dataIndex: 'sourceType',
      width: 120,
      valueType: 'select',
      valueEnum: {
        MANUAL: { text: '手动撤销' },
        BATCH: { text: '批量撤销' },
        RULE: { text: '规则撤销' },
        SYSTEM: { text: '系统撤销' },
        EXPIRE: { text: '到期过期' },
        AUTO: { text: '自动取消' },
      },
      render: (_, record) => {
        const config = sourceTypeConfig[record.sourceType?.toUpperCase()] || {
          color: 'default',
          text: record.sourceType,
        };
        return <Tag color={config.color}>{config.text}</Tag>;
      },
    },
    {
      title: '撤销原因',
      dataIndex: 'reason',
      width: 200,
      ellipsis: true,
      search: false,
    },
    {
      title: '操作人',
      dataIndex: 'operatorName',
      width: 100,
      search: false,
      render: (_, record) => record.operatorName || '-',
    },
    {
      title: '撤销时间',
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
  ];

  return (
    <PageContainer title="撤销记录">
      <ProTable<GrantLog, RevokeLogParams>
        actionRef={actionRef}
        columns={columns}
        rowKey="id"
        request={async (params) => {
          const { current, pageSize, userId, badgeId, sourceType, ...rest } = params;
          try {
            const response = await getRevokeRecords({
              page: current || 1,
              pageSize: pageSize || 20,
              userId,
              badgeId: badgeId ? Number(badgeId) : undefined,
              sourceType,
              ...rest,
            });
            return {
              data: response.items,
              total: response.total,
              success: true,
            };
          } catch {
            return {
              data: [],
              total: 0,
              success: false,
            };
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
            key="export"
            icon={<DownloadOutlined />}
            loading={exporting}
            onClick={() => handleExport({})}
          >
            导出
          </Button>,
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
    </PageContainer>
  );
};

export default RevokeLogsPage;

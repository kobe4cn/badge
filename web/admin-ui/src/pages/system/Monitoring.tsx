/**
 * 监控告警管理页面
 *
 * 展示系统服务健康状态、告警规则和告警历史，
 * 支持告警规则的启用/禁用、告警事件的确认和静默操作。
 *
 * 对应需求 §11（系统日志和告警）中的监测和告警模块
 */

import React, { useState, useCallback, useMemo } from 'react';
import {
  Card,
  Table,
  Tag,
  Switch,
  Button,
  Space,
  Row,
  Col,
  Badge,
  Modal,
  InputNumber,
  message,
  Tooltip,
  Alert,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  BellOutlined,
  StopOutlined,
} from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import type { ColumnsType } from 'antd/es/table';
import {
  getServiceHealth,
  listAlertRules,
  toggleAlertRule,
  listAlertEvents,
  acknowledgeAlert,
  silenceAlert,
} from '@/services/monitoring';
import type {
  ServiceHealthStatus,
  AlertRule,
  AlertEvent,
  AlertSeverity,
  AlertStatus,
} from '@/types/monitoring';
import { formatDate } from '@/utils/format';

/**
 * 健康状态配色映射
 */
const HEALTH_STATUS_CONFIG: Record<ServiceHealthStatus, { color: string; icon: React.ReactNode; text: string }> = {
  healthy: { color: '#52c41a', icon: <CheckCircleOutlined />, text: '正常' },
  degraded: { color: '#faad14', icon: <ExclamationCircleOutlined />, text: '降级' },
  down: { color: '#ff4d4f', icon: <CloseCircleOutlined />, text: '宕机' },
  unknown: { color: '#d9d9d9', icon: <QuestionCircleOutlined />, text: '未知' },
};

/**
 * 告警严重程度配色
 */
const SEVERITY_CONFIG: Record<AlertSeverity, { color: string; text: string }> = {
  critical: { color: 'red', text: '严重' },
  warning: { color: 'orange', text: '警告' },
  info: { color: 'blue', text: '信息' },
};

/**
 * 告警状态配色
 */
const ALERT_STATUS_CONFIG: Record<AlertStatus, { color: string; text: string }> = {
  firing: { color: 'red', text: '触发中' },
  resolved: { color: 'green', text: '已恢复' },
  silenced: { color: 'default', text: '已静默' },
  acknowledged: { color: 'blue', text: '已确认' },
};

const MonitoringPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [silenceModalVisible, setSilenceModalVisible] = useState(false);
  const [silenceEventId, setSilenceEventId] = useState<number | null>(null);
  const [silenceDuration, setSilenceDuration] = useState<number>(30);

  // 获取服务健康状态
  const {
    data: healthData,
    isLoading: healthLoading,
  } = useQuery({
    queryKey: ['monitoring', 'health'],
    queryFn: getServiceHealth,
    refetchInterval: 60 * 1000, // 每分钟刷新
  });

  // 获取告警规则
  const {
    data: alertRules,
    isLoading: rulesLoading,
  } = useQuery({
    queryKey: ['monitoring', 'rules'],
    queryFn: listAlertRules,
  });

  // 获取告警事件
  const {
    data: alertEventsData,
    isLoading: eventsLoading,
  } = useQuery({
    queryKey: ['monitoring', 'events'],
    queryFn: () => listAlertEvents(),
    refetchInterval: 30 * 1000,
  });

  // 切换告警规则启用状态
  const toggleRuleMutation = useMutation({
    mutationFn: ({ ruleId, enabled }: { ruleId: number; enabled: boolean }) =>
      toggleAlertRule(ruleId, enabled),
    onSuccess: () => {
      message.success('告警规则状态已更新');
      queryClient.invalidateQueries({ queryKey: ['monitoring', 'rules'] });
    },
  });

  // 确认告警
  const acknowledgeMutation = useMutation({
    mutationFn: acknowledgeAlert,
    onSuccess: () => {
      message.success('告警已确认');
      queryClient.invalidateQueries({ queryKey: ['monitoring', 'events'] });
    },
  });

  // 静默告警
  const silenceMutation = useMutation({
    mutationFn: ({ eventId, duration }: { eventId: number; duration: number }) =>
      silenceAlert(eventId, duration),
    onSuccess: () => {
      message.success('告警已静默');
      setSilenceModalVisible(false);
      queryClient.invalidateQueries({ queryKey: ['monitoring', 'events'] });
    },
  });

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['monitoring'] });
  }, [queryClient]);

  const handleSilence = useCallback((eventId: number) => {
    setSilenceEventId(eventId);
    setSilenceDuration(30);
    setSilenceModalVisible(true);
  }, []);

  const confirmSilence = useCallback(() => {
    if (silenceEventId !== null) {
      silenceMutation.mutate({ eventId: silenceEventId, duration: silenceDuration });
    }
  }, [silenceEventId, silenceDuration, silenceMutation]);

  // 统计健康服务数
  const healthSummary = useMemo(() => {
    if (!healthData) return { healthy: 0, degraded: 0, down: 0, unknown: 0 };
    return healthData.reduce(
      (acc, s) => {
        acc[s.status] = (acc[s.status] || 0) + 1;
        return acc;
      },
      { healthy: 0, degraded: 0, down: 0, unknown: 0 } as Record<ServiceHealthStatus, number>
    );
  }, [healthData]);

  // 告警规则表格列
  const ruleColumns: ColumnsType<AlertRule> = [
    {
      title: '规则名称',
      dataIndex: 'name',
      key: 'name',
      width: 200,
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
    },
    {
      title: '严重程度',
      dataIndex: 'severity',
      key: 'severity',
      width: 100,
      render: (severity: AlertSeverity) => (
        <Tag color={SEVERITY_CONFIG[severity].color}>
          {SEVERITY_CONFIG[severity].text}
        </Tag>
      ),
    },
    {
      title: '持续时间',
      dataIndex: 'durationSeconds',
      key: 'duration',
      width: 100,
      render: (seconds: number) => {
        if (seconds >= 60) return `${Math.floor(seconds / 60)} 分钟`;
        return `${seconds} 秒`;
      },
    },
    {
      title: '通知渠道',
      dataIndex: 'notifyChannels',
      key: 'channels',
      width: 160,
      render: (channels: string[]) => (
        <Space size={4} wrap>
          {channels.map((c) => (
            <Tag key={c}>{c}</Tag>
          ))}
        </Space>
      ),
    },
    {
      title: '启用',
      dataIndex: 'enabled',
      key: 'enabled',
      width: 80,
      render: (enabled: boolean, record: AlertRule) => (
        <Switch
          checked={enabled}
          size="small"
          onChange={(checked) =>
            toggleRuleMutation.mutate({ ruleId: record.id, enabled: checked })
          }
        />
      ),
    },
  ];

  // 告警事件表格列
  const eventColumns: ColumnsType<AlertEvent> = [
    {
      title: '规则名称',
      dataIndex: 'ruleName',
      key: 'ruleName',
      width: 180,
    },
    {
      title: '严重程度',
      dataIndex: 'severity',
      key: 'severity',
      width: 100,
      render: (severity: AlertSeverity) => (
        <Tag color={SEVERITY_CONFIG[severity].color}>
          {SEVERITY_CONFIG[severity].text}
        </Tag>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: AlertStatus) => (
        <Tag color={ALERT_STATUS_CONFIG[status].color}>
          {ALERT_STATUS_CONFIG[status].text}
        </Tag>
      ),
    },
    {
      title: '告警详情',
      dataIndex: 'message',
      key: 'message',
      ellipsis: true,
    },
    {
      title: '触发时间',
      dataIndex: 'firedAt',
      key: 'firedAt',
      width: 170,
      render: (v: string) => formatDate(v),
    },
    {
      title: '恢复时间',
      dataIndex: 'resolvedAt',
      key: 'resolvedAt',
      width: 170,
      render: (v: string) => v ? formatDate(v) : '-',
    },
    {
      title: '操作',
      key: 'action',
      width: 160,
      render: (_: unknown, record: AlertEvent) => (
        <Space size={4}>
          {record.status === 'firing' && (
            <>
              <Tooltip title="确认告警">
                <Button
                  type="link"
                  size="small"
                  icon={<BellOutlined />}
                  onClick={() => acknowledgeMutation.mutate(record.id)}
                >
                  确认
                </Button>
              </Tooltip>
              <Tooltip title="静默告警">
                <Button
                  type="link"
                  size="small"
                  icon={<StopOutlined />}
                  onClick={() => handleSilence(record.id)}
                >
                  静默
                </Button>
              </Tooltip>
            </>
          )}
        </Space>
      ),
    },
  ];

  return (
    <PageContainer
      title="监控告警"
      extra={
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
          刷新
        </Button>
      }
    >
      <Alert
        message="告警系统正在建设中，当前展示的是预置告警规则，后续将接入 Prometheus AlertManager"
        type="info"
        showIcon
        closable
        style={{ marginBottom: 24 }}
      />

      {/* 服务健康状态 */}
      <Card title="服务健康状态" style={{ marginBottom: 24 }} loading={healthLoading}>
        <Row gutter={[16, 16]}>
          {healthData?.map((service) => {
            const config = HEALTH_STATUS_CONFIG[service.status];
            return (
              <Col xs={24} sm={12} md={8} lg={4} key={service.serviceId}>
                <Card
                  size="small"
                  hoverable
                  style={{ textAlign: 'center' }}
                >
                  <Badge
                    status={
                      service.status === 'healthy' ? 'success' :
                      service.status === 'degraded' ? 'warning' :
                      service.status === 'down' ? 'error' : 'default'
                    }
                  />
                  <div style={{ fontSize: 28, color: config.color, margin: '8px 0' }}>
                    {config.icon}
                  </div>
                  <div style={{ fontWeight: 500 }}>{service.name}</div>
                  <Tag color={config.color} style={{ marginTop: 8 }}>
                    {config.text}
                  </Tag>
                </Card>
              </Col>
            );
          })}
        </Row>

        {/* 健康概览数字 */}
        <div style={{ marginTop: 16, display: 'flex', gap: 24 }}>
          <span>
            <Badge status="success" /> 正常: {healthSummary.healthy}
          </span>
          <span>
            <Badge status="warning" /> 降级: {healthSummary.degraded}
          </span>
          <span>
            <Badge status="error" /> 宕机: {healthSummary.down}
          </span>
          <span>
            <Badge status="default" /> 未知: {healthSummary.unknown}
          </span>
        </div>
      </Card>

      {/* 告警规则 */}
      <Card title="告警规则" style={{ marginBottom: 24 }}>
        <Table<AlertRule>
          columns={ruleColumns}
          dataSource={alertRules || []}
          rowKey="id"
          loading={rulesLoading}
          pagination={false}
          size="middle"
          locale={{ emptyText: '暂无告警规则' }}
        />
      </Card>

      {/* 告警历史 */}
      <Card title="告警历史">
        <Table<AlertEvent>
          columns={eventColumns}
          dataSource={alertEventsData?.items || []}
          rowKey="id"
          loading={eventsLoading}
          pagination={{
            total: alertEventsData?.total || 0,
            pageSize: 20,
            showTotal: (total) => `共 ${total} 条`,
          }}
          size="middle"
          locale={{ emptyText: '暂无告警记录' }}
        />
      </Card>

      {/* 静默时长输入弹窗 */}
      <Modal
        title="设置静默时长"
        open={silenceModalVisible}
        onOk={confirmSilence}
        onCancel={() => setSilenceModalVisible(false)}
        confirmLoading={silenceMutation.isPending}
      >
        <div style={{ marginBottom: 16 }}>
          在指定时间内不再触发此告警的通知：
        </div>
        <InputNumber
          min={5}
          max={1440}
          value={silenceDuration}
          onChange={(v) => setSilenceDuration(v || 30)}
          addonAfter="分钟"
          style={{ width: '100%' }}
        />
      </Modal>
    </PageContainer>
  );
};

export default MonitoringPage;

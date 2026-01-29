/**
 * 规则测试结果组件
 *
 * 展示规则评估的详细结果，包括匹配状态、条件评估明细、
 * 触发的动作以及性能指标
 */

import React from 'react';
import {
  Drawer,
  Result,
  Descriptions,
  Table,
  Tag,
  Timeline,
  Typography,
  Space,
  Statistic,
  Card,
  Row,
  Col,
  Empty,
} from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  TrophyOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import type { RuleTestResult, ConditionEvaluation } from '@/services/rule';

const { Text, Paragraph } = Typography;

/**
 * 测试结果组件属性
 */
export interface TestResultProps {
  /** 是否可见 */
  open: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 测试结果数据 */
  result?: RuleTestResult;
  /** 是否正在加载 */
  loading?: boolean;
}

/**
 * 条件结果表格列配置
 */
const conditionColumns = [
  {
    title: '条件字段',
    dataIndex: 'field',
    key: 'field',
    render: (field: string) => <Text code>{field}</Text>,
  },
  {
    title: '操作符',
    dataIndex: 'operator',
    key: 'operator',
    width: 100,
    render: (op: string) => {
      const opLabels: Record<string, string> = {
        eq: '等于',
        neq: '不等于',
        gt: '大于',
        gte: '大于等于',
        lt: '小于',
        lte: '小于等于',
        contains: '包含',
        in: '属于',
        not_in: '不属于',
      };
      return <Tag>{opLabels[op] || op}</Tag>;
    },
  },
  {
    title: '期望值',
    dataIndex: 'expectedValue',
    key: 'expectedValue',
    render: (value: unknown) => (
      <Text type="secondary">{JSON.stringify(value)}</Text>
    ),
  },
  {
    title: '实际值',
    dataIndex: 'actualValue',
    key: 'actualValue',
    render: (value: unknown, record: ConditionEvaluation) => (
      <Text type={record.matched ? 'success' : 'danger'}>
        {JSON.stringify(value)}
      </Text>
    ),
  },
  {
    title: '结果',
    dataIndex: 'matched',
    key: 'matched',
    width: 80,
    render: (matched: boolean) =>
      matched ? (
        <Tag color="success" icon={<CheckCircleOutlined />}>
          匹配
        </Tag>
      ) : (
        <Tag color="error" icon={<CloseCircleOutlined />}>
          不匹配
        </Tag>
      ),
  },
];

const TestResult: React.FC<TestResultProps> = ({
  open,
  onClose,
  result,
  loading = false,
}) => {
  if (!result && !loading) {
    return (
      <Drawer
        title="测试结果"
        placement="right"
        width={600}
        open={open}
        onClose={onClose}
      >
        <Empty description="暂无测试结果" />
      </Drawer>
    );
  }

  const matchedCount = result?.conditionResults?.filter((c) => c.matched).length || 0;
  const totalCount = result?.conditionResults?.length || 0;

  return (
    <Drawer
      title="测试结果"
      placement="right"
      width={720}
      open={open}
      onClose={onClose}
      loading={loading}
    >
      {result && (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          {/* 整体结果 */}
          <Result
            status={result.matched ? 'success' : 'warning'}
            title={result.matched ? '规则匹配成功' : '规则未匹配'}
            subTitle={
              result.error ||
              (result.matched
                ? '所有条件都已满足，将触发配置的动作'
                : '部分条件不满足，规则未触发')
            }
            icon={
              result.matched ? (
                <CheckCircleOutlined style={{ color: '#52c41a' }} />
              ) : (
                <CloseCircleOutlined style={{ color: '#faad14' }} />
              )
            }
          />

          {/* 统计卡片 */}
          <Row gutter={16}>
            <Col span={8}>
              <Card size="small">
                <Statistic
                  title="条件匹配"
                  value={matchedCount}
                  suffix={`/ ${totalCount}`}
                  valueStyle={{
                    color: matchedCount === totalCount ? '#52c41a' : '#faad14',
                  }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small">
                <Statistic
                  title="触发动作"
                  value={result.triggeredActions?.length || 0}
                  prefix={<TrophyOutlined />}
                  valueStyle={{
                    color: result.matched ? '#1890ff' : '#999',
                  }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small">
                <Statistic
                  title="评估耗时"
                  value={result.evaluationTimeMs}
                  suffix="ms"
                  prefix={<ClockCircleOutlined />}
                  valueStyle={{
                    color: result.evaluationTimeMs < 100 ? '#52c41a' : '#faad14',
                  }}
                />
              </Card>
            </Col>
          </Row>

          {/* 条件评估详情 */}
          {result.conditionResults && result.conditionResults.length > 0 && (
            <Card
              title={
                <>
                  <ThunderboltOutlined /> 条件评估详情
                </>
              }
              size="small"
            >
              <Table
                dataSource={result.conditionResults}
                columns={conditionColumns}
                rowKey="nodeId"
                pagination={false}
                size="small"
                rowClassName={(record) =>
                  record.matched ? 'ant-table-row-success' : ''
                }
              />
            </Card>
          )}

          {/* 触发的动作 */}
          {result.matched && result.triggeredActions && result.triggeredActions.length > 0 && (
            <Card
              title={
                <>
                  <TrophyOutlined /> 触发的动作
                </>
              }
              size="small"
            >
              <Timeline
                items={result.triggeredActions.map((action, index) => ({
                  key: index,
                  color: 'green',
                  dot: <TrophyOutlined />,
                  children: (
                    <Descriptions size="small" column={1}>
                      <Descriptions.Item label="动作类型">
                        <Tag color="blue">{action.type}</Tag>
                      </Descriptions.Item>
                      {action.badgeName && (
                        <Descriptions.Item label="徽章名称">
                          {action.badgeName}
                        </Descriptions.Item>
                      )}
                      {action.badgeId && (
                        <Descriptions.Item label="徽章 ID">
                          <Text code>{action.badgeId}</Text>
                        </Descriptions.Item>
                      )}
                      {action.quantity && (
                        <Descriptions.Item label="发放数量">
                          <Text strong>{action.quantity}</Text>
                        </Descriptions.Item>
                      )}
                    </Descriptions>
                  ),
                }))}
              />
            </Card>
          )}

          {/* 匹配路径提示 */}
          {result.matchedNodeIds && result.matchedNodeIds.length > 0 && (
            <Card size="small" title="匹配的节点">
              <Paragraph type="secondary" style={{ marginBottom: 8 }}>
                以下节点在画布中会高亮显示：
              </Paragraph>
              <Space wrap>
                {result.matchedNodeIds.map((nodeId) => (
                  <Tag key={nodeId} color="green">
                    {nodeId}
                  </Tag>
                ))}
              </Space>
            </Card>
          )}
        </Space>
      )}

      {/* 自定义表格行样式 */}
      <style>{`
        .ant-table-row-success {
          background-color: #f6ffed;
        }
        .ant-table-row-success:hover > td {
          background-color: #d9f7be !important;
        }
      `}</style>
    </Drawer>
  );
};

export default TestResult;

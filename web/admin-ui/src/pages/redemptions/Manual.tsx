/**
 * 手动兑换页面
 *
 * 允许管理员为用户手动执行兑换操作
 */

import React, { useState } from 'react';
import {
  Card,
  Form,
  Input,
  Button,
  Select,
  Space,
  Descriptions,
  Tag,
  Alert,
  Result,
  Typography,
  message,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import { GiftOutlined, CheckCircleOutlined, UserOutlined } from '@ant-design/icons';
import { useQuery, useMutation } from '@tanstack/react-query';
import {
  listRedemptionRules,
  getRedemptionRule,
  executeRedemption,
  type RedemptionRule,
  type ExecuteRedemptionResponse,
} from '@/services/redemption';

const { Text } = Typography;

const ManualRedemptionPage: React.FC = () => {
  const [form] = Form.useForm();
  const [selectedRuleId, setSelectedRuleId] = useState<number | null>(null);
  const [redeemResult, setRedeemResult] = useState<ExecuteRedemptionResponse | null>(null);

  // 获取已启用的兑换规则列表
  const { data: rulesData, isLoading: rulesLoading } = useQuery({
    queryKey: ['redemptionRules', 'enabled'],
    queryFn: () => listRedemptionRules({ enabled: true, pageSize: 100 }),
  });

  // 获取选中规则的详情
  const { data: ruleDetail, isLoading: ruleDetailLoading } = useQuery({
    queryKey: ['redemptionRule', selectedRuleId],
    queryFn: () => getRedemptionRule(selectedRuleId!),
    enabled: !!selectedRuleId,
  });

  // 执行兑换
  const redeemMutation = useMutation({
    mutationFn: executeRedemption,
    onSuccess: (result) => {
      setRedeemResult(result);
      if (result.success) {
        message.success(`兑换成功！订单号: ${result.orderNo}`);
      } else {
        message.warning(result.message);
      }
    },
    onError: (error: { message?: string }) => {
      message.error(error.message || '兑换失败');
    },
  });

  const handleRuleChange = (ruleId: number) => {
    setSelectedRuleId(ruleId);
    setRedeemResult(null);
  };

  const handleSubmit = async (values: { userId: string; ruleId: number }) => {
    setRedeemResult(null);
    redeemMutation.mutate({
      userId: values.userId,
      ruleId: values.ruleId,
    });
  };

  const handleReset = () => {
    form.resetFields();
    setSelectedRuleId(null);
    setRedeemResult(null);
  };

  const rules = rulesData?.items || [];

  // 渲染频率限制信息
  const renderFrequencyInfo = (rule: RedemptionRule) => {
    const { frequencyConfig } = rule;
    if (!frequencyConfig) return <Tag color="green">无限制</Tag>;

    const limits = [];
    if (frequencyConfig.maxPerUser) limits.push(`总计 ${frequencyConfig.maxPerUser} 次`);
    if (frequencyConfig.maxPerDay) limits.push(`每日 ${frequencyConfig.maxPerDay} 次`);
    if (frequencyConfig.maxPerWeek) limits.push(`每周 ${frequencyConfig.maxPerWeek} 次`);
    if (frequencyConfig.maxPerMonth) limits.push(`每月 ${frequencyConfig.maxPerMonth} 次`);
    if (frequencyConfig.maxPerYear) limits.push(`每年 ${frequencyConfig.maxPerYear} 次`);

    if (limits.length === 0) return <Tag color="green">无限制</Tag>;
    return limits.map((limit, index) => (
      <Tag key={index} color="blue">
        {limit}
      </Tag>
    ));
  };

  return (
    <PageContainer
      header={{
        title: '手动兑换',
        subTitle: '为用户手动执行兑换操作',
      }}
    >
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          style={{ maxWidth: 600 }}
        >
          <Form.Item
            name="ruleId"
            label="选择兑换规则"
            rules={[{ required: true, message: '请选择兑换规则' }]}
          >
            <Select
              placeholder="选择要执行的兑换规则"
              loading={rulesLoading}
              onChange={handleRuleChange}
              optionFilterProp="label"
              showSearch
              options={rules.map((rule) => ({
                value: rule.id,
                label: `${rule.name} → ${rule.benefitName}`,
              }))}
            />
          </Form.Item>

          <Form.Item
            name="userId"
            label="用户 ID"
            rules={[
              { required: true, message: '请输入用户 ID' },
              { min: 1, message: '用户 ID 不能为空' },
            ]}
          >
            <Input
              prefix={<UserOutlined />}
              placeholder="输入要执行兑换的用户 ID"
            />
          </Form.Item>

          <Form.Item>
            <Space>
              <Button
                type="primary"
                htmlType="submit"
                icon={<GiftOutlined />}
                loading={redeemMutation.isPending}
              >
                执行兑换
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      {/* 规则详情展示 */}
      {selectedRuleId && ruleDetail && (
        <Card title="规则详情" style={{ marginTop: 16 }} loading={ruleDetailLoading}>
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="规则名称">{ruleDetail.name}</Descriptions.Item>
            <Descriptions.Item label="关联权益">
              <Tag color="purple">{ruleDetail.benefitName}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="描述" span={2}>
              {ruleDetail.description || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="所需徽章" span={2}>
              {ruleDetail.requiredBadges.length > 0 ? (
                <Space wrap>
                  {ruleDetail.requiredBadges.map((badge, index) => (
                    <Tag key={index} color="orange">
                      {badge.badgeName} × {badge.quantity}
                    </Tag>
                  ))}
                </Space>
              ) : (
                <Text type="secondary">无需徽章</Text>
              )}
            </Descriptions.Item>
            <Descriptions.Item label="频率限制" span={2}>
              {renderFrequencyInfo(ruleDetail)}
            </Descriptions.Item>
            <Descriptions.Item label="有效期">
              {ruleDetail.startTime || ruleDetail.endTime ? (
                <>
                  {ruleDetail.startTime || '不限'} ~ {ruleDetail.endTime || '不限'}
                </>
              ) : (
                '长期有效'
              )}
            </Descriptions.Item>
            <Descriptions.Item label="自动兑换">
              {ruleDetail.autoRedeem ? (
                <Tag color="green">是</Tag>
              ) : (
                <Tag color="default">否</Tag>
              )}
            </Descriptions.Item>
          </Descriptions>

          <Alert
            style={{ marginTop: 16 }}
            message="操作提示"
            description={
              <ul style={{ margin: 0, paddingLeft: 20 }}>
                <li>手动兑换会立即为用户发放对应权益</li>
                <li>系统会自动检查用户是否满足兑换条件（所需徽章、频率限制等）</li>
                <li>兑换记录将记录在系统日志中</li>
              </ul>
            }
            type="info"
            showIcon
          />
        </Card>
      )}

      {/* 兑换结果展示 */}
      {redeemResult && (
        <Card style={{ marginTop: 16 }}>
          {redeemResult.success ? (
            <Result
              status="success"
              icon={<CheckCircleOutlined />}
              title="兑换成功"
              subTitle={redeemResult.message}
              extra={[
                <Descriptions key="details" column={1} bordered size="small">
                  <Descriptions.Item label="订单号">
                    <Text copyable>{redeemResult.orderNo}</Text>
                  </Descriptions.Item>
                  <Descriptions.Item label="发放权益">
                    {redeemResult.benefitName}
                  </Descriptions.Item>
                </Descriptions>,
              ]}
            />
          ) : (
            <Result
              status="warning"
              title="兑换未完成"
              subTitle={redeemResult.message}
            />
          )}
        </Card>
      )}
    </PageContainer>
  );
};

export default ManualRedemptionPage;

/**
 * 手动发放页面
 *
 * 支持管理员手动为用户发放徽章，使用步骤条引导完成发放流程：
 * 1. 选择用户 - 搜索并选择发放目标用户
 * 2. 选择徽章 - 选择徽章并设置发放数量和原因
 * 3. 确认发放 - 预览发放信息并确认
 * 4. 发放完成 - 展示发放结果
 */

import React, { useState, useCallback } from 'react';
import {
  Card,
  Steps,
  Button,
  Space,
  Typography,
  Table,
  Avatar,
  Tag,
  InputNumber,
  Input,
  Result,
  Alert,
  Divider,
  Descriptions,
  List,
  Statistic,
  Row,
  Col,
} from 'antd';
import { PageContainer } from '@ant-design/pro-components';
import {
  UserOutlined,
  GiftOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  ArrowLeftOutlined,
  ArrowRightOutlined,
  SendOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import UserSelect, { type UserSelectValue } from './components/UserSelect';
import BadgeSelect, { type BadgeSelectValue } from './components/BadgeSelect';
import { useManualGrant } from '@/hooks/useGrant';
import { getMembershipConfig } from '@/types/user';
import type { GrantResult, UserGrantResult } from '@/services/grant';
import type { ColumnsType } from 'antd/es/table';

const { Title, Text } = Typography;
const { TextArea } = Input;

/**
 * 发放表单数据
 */
interface GrantFormData {
  users: UserSelectValue[];
  badge: BadgeSelectValue | undefined;
  quantity: number;
  reason: string;
}

/**
 * 初始表单数据
 */
const initialFormData: GrantFormData = {
  users: [],
  badge: undefined,
  quantity: 1,
  reason: '',
};

/**
 * 手动发放页面组件
 */
const ManualGrantPage: React.FC = () => {
  const [currentStep, setCurrentStep] = useState(0);
  const [formData, setFormData] = useState<GrantFormData>(initialFormData);
  const [grantResult, setGrantResult] = useState<GrantResult | null>(null);

  // 发放 mutation
  const { mutateAsync: doGrant, isPending: isGranting } = useManualGrant();

  /**
   * 更新表单数据
   */
  const updateFormData = useCallback(
    <K extends keyof GrantFormData>(key: K, value: GrantFormData[K]) => {
      setFormData((prev) => ({ ...prev, [key]: value }));
    },
    []
  );

  /**
   * 重置流程
   */
  const handleReset = useCallback(() => {
    setCurrentStep(0);
    setFormData(initialFormData);
    setGrantResult(null);
  }, []);

  /**
   * 下一步
   */
  const handleNext = useCallback(() => {
    setCurrentStep((prev) => prev + 1);
  }, []);

  /**
   * 上一步
   */
  const handlePrev = useCallback(() => {
    setCurrentStep((prev) => prev - 1);
  }, []);

  /**
   * 执行发放
   */
  const handleGrant = useCallback(async () => {
    if (!formData.badge) return;

    try {
      const result = await doGrant({
        userIds: formData.users.map((u) => u.value),
        badgeId: formData.badge.value,
        quantity: formData.quantity,
        reason: formData.reason || undefined,
      });
      setGrantResult(result);
      setCurrentStep(3);
    } catch {
      // 错误已在 hook 中处理
    }
  }, [formData, doGrant]);

  /**
   * 验证当前步骤是否可以继续
   */
  const canProceed = useCallback(() => {
    switch (currentStep) {
      case 0:
        return formData.users.length > 0;
      case 1:
        return formData.badge !== undefined && formData.quantity > 0;
      case 2:
        return true;
      default:
        return false;
    }
  }, [currentStep, formData]);

  /**
   * 渲染步骤 1：选择用户
   */
  const renderSelectUsers = () => {
    // 用户表格列定义
    const columns: ColumnsType<UserSelectValue> = [
      {
        title: '用户',
        key: 'user',
        render: (_, record) => (
          <Space>
            <Avatar size="small" icon={<UserOutlined />} />
            <div>
              <div>
                <Text strong>{record.user.username}</Text>
              </div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {record.user.userId}
              </Text>
            </div>
          </Space>
        ),
      },
      {
        title: '手机号',
        dataIndex: ['user', 'phone'],
        key: 'phone',
        render: (phone: string) => phone || '-',
      },
      {
        title: '会员等级',
        key: 'level',
        render: (_, record) => {
          const config = getMembershipConfig(record.user.membershipLevel);
          return <Tag color={config.color}>{config.name}</Tag>;
        },
      },
      {
        title: '操作',
        key: 'action',
        width: 80,
        render: (_, record) => (
          <Button
            type="link"
            danger
            size="small"
            onClick={() => {
              updateFormData(
                'users',
                formData.users.filter((u) => u.value !== record.value)
              );
            }}
          >
            移除
          </Button>
        ),
      },
    ];

    return (
      <div>
        <div style={{ marginBottom: 24 }}>
          <Title level={5}>搜索用户</Title>
          <Text type="secondary">
            输入用户 ID、手机号或昵称搜索用户，支持选择多个用户
          </Text>
        </div>

        <UserSelect
          value={formData.users}
          onChange={(users) => updateFormData('users', users)}
          placeholder="请输入用户 ID、手机号或昵称搜索"
          style={{ marginBottom: 24 }}
        />

        {formData.users.length > 0 && (
          <>
            <Divider orientation="left">
              已选择 {formData.users.length} 位用户
            </Divider>
            <Table
              dataSource={formData.users}
              columns={columns}
              rowKey="value"
              pagination={false}
              size="small"
            />
          </>
        )}
      </div>
    );
  };

  /**
   * 渲染步骤 2：选择徽章
   */
  const renderSelectBadge = () => (
    <div>
      <div style={{ marginBottom: 24 }}>
        <Title level={5}>选择徽章</Title>
        <Text type="secondary">
          选择要发放的徽章，可按分类和系列筛选，只能选择已上架且有库存的徽章
        </Text>
      </div>

      <BadgeSelect
        value={formData.badge}
        onChange={(badge) => updateFormData('badge', badge)}
        placeholder="请选择要发放的徽章"
        style={{ marginBottom: 24 }}
      />

      {formData.badge && (
        <Card size="small" style={{ marginBottom: 24 }}>
          <Descriptions column={2} size="small">
            <Descriptions.Item label="徽章名称">
              <Space>
                <Avatar
                  size={24}
                  src={formData.badge.badge.assets.iconUrl}
                  shape="square"
                />
                {formData.badge.badge.name}
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="所属系列">
              {formData.badge.badge.seriesName || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="徽章类型">
              <Tag>
                {formData.badge.badge.badgeType === 'NORMAL'
                  ? '普通徽章'
                  : formData.badge.badge.badgeType === 'LIMITED'
                  ? '限定徽章'
                  : formData.badge.badge.badgeType === 'ACHIEVEMENT'
                  ? '成就徽章'
                  : '活动徽章'}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="库存">
              {formData.badge.badge.maxSupply
                ? `${formData.badge.badge.maxSupply - formData.badge.badge.issuedCount} / ${formData.badge.badge.maxSupply}`
                : '不限量'}
            </Descriptions.Item>
          </Descriptions>
        </Card>
      )}

      <div style={{ marginBottom: 16 }}>
        <Title level={5}>发放数量</Title>
        <InputNumber
          min={1}
          max={99}
          value={formData.quantity}
          onChange={(val) => updateFormData('quantity', val || 1)}
          style={{ width: 200 }}
          addonAfter="个/人"
        />
      </div>

      <div>
        <Title level={5}>发放原因（可选）</Title>
        <TextArea
          value={formData.reason}
          onChange={(e) => updateFormData('reason', e.target.value)}
          placeholder="请输入发放原因，将记录在发放日志中"
          rows={3}
          maxLength={200}
          showCount
        />
      </div>
    </div>
  );

  /**
   * 渲染步骤 3：确认发放
   */
  const renderConfirm = () => {
    const totalQuantity = formData.users.length * formData.quantity;

    return (
      <div>
        <Alert
          message="请确认发放信息"
          description="发放后不可撤销，请仔细核对以下信息"
          type="warning"
          showIcon
          icon={<ExclamationCircleOutlined />}
          style={{ marginBottom: 24 }}
        />

        <Card title="发放汇总" style={{ marginBottom: 24 }}>
          <Row gutter={[24, 16]}>
            <Col xs={24} sm={8}>
              <Statistic
                title="发放用户数"
                value={formData.users.length}
                suffix="人"
              />
            </Col>
            <Col xs={12} sm={8}>
              <Statistic
                title="每人发放数量"
                value={formData.quantity}
                suffix="个"
              />
            </Col>
            <Col xs={12} sm={8}>
              <Statistic
                title="发放总数"
                value={totalQuantity}
                suffix="个"
                valueStyle={{ color: '#1890ff' }}
              />
            </Col>
          </Row>
        </Card>

        <Card
          title="徽章信息"
          style={{ marginBottom: 24 }}
          extra={
            formData.badge && (
              <Avatar
                size={48}
                src={formData.badge.badge.assets.iconUrl}
                shape="square"
              />
            )
          }
        >
          {formData.badge && (
            <Descriptions column={2} size="small">
              <Descriptions.Item label="徽章名称">
                {formData.badge.badge.name}
              </Descriptions.Item>
              <Descriptions.Item label="所属系列">
                {formData.badge.badge.seriesName || '-'}
              </Descriptions.Item>
              <Descriptions.Item label="发放原因" span={2}>
                {formData.reason || '未填写'}
              </Descriptions.Item>
            </Descriptions>
          )}
        </Card>

        <Card title="用户列表">
          <List
            size="small"
            dataSource={formData.users}
            renderItem={(user) => (
              <List.Item>
                <List.Item.Meta
                  avatar={<Avatar size="small" icon={<UserOutlined />} />}
                  title={user.user.username}
                  description={user.user.userId}
                />
                <Tag color={getMembershipConfig(user.user.membershipLevel).color}>
                  {getMembershipConfig(user.user.membershipLevel).name}
                </Tag>
              </List.Item>
            )}
          />
        </Card>
      </div>
    );
  };

  /**
   * 渲染步骤 4：发放结果
   */
  const renderResult = () => {
    if (!grantResult) return null;

    const isAllSuccess = grantResult.failedCount === 0;
    const isAllFailed = grantResult.successCount === 0;

    // 成功/失败用户列表
    const successResults = grantResult.results.filter((r) => r.success);
    const failedResults = grantResult.results.filter((r) => !r.success);

    // 获取用户信息
    const getUserInfo = (userId: string) => {
      return formData.users.find((u) => u.value === userId);
    };

    // 结果表格列
    const resultColumns: ColumnsType<UserGrantResult> = [
      {
        title: '用户',
        key: 'user',
        render: (_, record) => {
          const userInfo = getUserInfo(record.userId);
          return (
            <Space>
              <Avatar size="small" icon={<UserOutlined />} />
              <div>
                <div>
                  <Text strong>{userInfo?.user.username || record.userId}</Text>
                </div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {record.userId}
                </Text>
              </div>
            </Space>
          );
        },
      },
      {
        title: '状态',
        key: 'status',
        width: 100,
        render: (_, record) =>
          record.success ? (
            <Tag color="success" icon={<CheckCircleOutlined />}>
              成功
            </Tag>
          ) : (
            <Tag color="error" icon={<CloseCircleOutlined />}>
              失败
            </Tag>
          ),
      },
      {
        title: '信息',
        key: 'message',
        render: (_, record) =>
          record.success
            ? `徽章 ID: ${record.userBadgeId}`
            : record.message || '发放失败',
      },
    ];

    return (
      <div>
        <Result
          status={isAllSuccess ? 'success' : isAllFailed ? 'error' : 'warning'}
          icon={
            isAllSuccess ? (
              <CheckCircleOutlined />
            ) : isAllFailed ? (
              <CloseCircleOutlined />
            ) : (
              <ExclamationCircleOutlined />
            )
          }
          title={
            isAllSuccess
              ? '发放成功'
              : isAllFailed
              ? '发放失败'
              : '部分发放成功'
          }
          subTitle={
            <Space>
              <span>成功: {grantResult.successCount} 人</span>
              {grantResult.failedCount > 0 && (
                <span style={{ color: '#ff4d4f' }}>
                  失败: {grantResult.failedCount} 人
                </span>
              )}
            </Space>
          }
          extra={[
            <Button
              key="reset"
              type="primary"
              icon={<ReloadOutlined />}
              onClick={handleReset}
            >
              继续发放
            </Button>,
          ]}
        />

        {successResults.length > 0 && (
          <Card
            title={
              <Space>
                <CheckCircleOutlined style={{ color: '#52c41a' }} />
                成功用户 ({successResults.length})
              </Space>
            }
            style={{ marginTop: 24 }}
          >
            <Table
              dataSource={successResults}
              columns={resultColumns}
              rowKey="userId"
              pagination={false}
              size="small"
            />
          </Card>
        )}

        {failedResults.length > 0 && (
          <Card
            title={
              <Space>
                <CloseCircleOutlined style={{ color: '#ff4d4f' }} />
                失败用户 ({failedResults.length})
              </Space>
            }
            style={{ marginTop: 24 }}
          >
            <Table
              dataSource={failedResults}
              columns={resultColumns}
              rowKey="userId"
              pagination={false}
              size="small"
            />
          </Card>
        )}
      </div>
    );
  };

  /**
   * 渲染当前步骤内容
   */
  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return renderSelectUsers();
      case 1:
        return renderSelectBadge();
      case 2:
        return renderConfirm();
      case 3:
        return renderResult();
      default:
        return null;
    }
  };

  /**
   * 渲染底部操作按钮
   */
  const renderActions = () => {
    if (currentStep === 3) return null;

    return (
      <div
        style={{
          marginTop: 24,
          paddingTop: 24,
          borderTop: '1px solid #f0f0f0',
          display: 'flex',
          justifyContent: 'space-between',
        }}
      >
        <div>
          {currentStep > 0 && (
            <Button icon={<ArrowLeftOutlined />} onClick={handlePrev}>
              上一步
            </Button>
          )}
        </div>
        <div>
          {currentStep < 2 && (
            <Button
              type="primary"
              icon={<ArrowRightOutlined />}
              onClick={handleNext}
              disabled={!canProceed()}
            >
              下一步
            </Button>
          )}
          {currentStep === 2 && (
            <Button
              type="primary"
              icon={<SendOutlined />}
              onClick={handleGrant}
              loading={isGranting}
              disabled={!canProceed()}
            >
              确认发放
            </Button>
          )}
        </div>
      </div>
    );
  };

  return (
    <PageContainer title="手动发放">
      <Card>
        <Steps
          current={currentStep}
          items={[
            {
              title: '选择用户',
              icon: <UserOutlined />,
            },
            {
              title: '选择徽章',
              icon: <GiftOutlined />,
            },
            {
              title: '确认发放',
              icon: <ExclamationCircleOutlined />,
            },
            {
              title: '发放完成',
              icon: <CheckCircleOutlined />,
            },
          ]}
          style={{ marginBottom: 32 }}
        />

        {renderStepContent()}
        {renderActions()}
      </Card>
    </PageContainer>
  );
};

export default ManualGrantPage;

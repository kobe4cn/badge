/**
 * 批量撤销页面
 *
 * 管理批量徽章撤销任务，支持：
 * - 选择徽章
 * - 上传用户列表 CSV
 * - 填写撤销原因
 * - 查看任务进度
 */

import React, { useState, useCallback } from 'react';
import {
  Button,
  Card,
  Form,
  Input,
  Space,
  Upload,
  message,
  Alert,
  Steps,
  Result,
  Typography,
  Descriptions,
  Progress,
  Table,
  Tag,
} from 'antd';
import {
  UploadOutlined,
  CheckCircleOutlined,
  LoadingOutlined,
  FileExcelOutlined,
  RollbackOutlined,
} from '@ant-design/icons';
import { PageContainer } from '@ant-design/pro-components';
import { useQuery, useMutation } from '@tanstack/react-query';
import { batchRevoke } from '@/services/revoke';
import { uploadUserCsv, getBatchTask, getBatchTaskFailures } from '@/services/grant';
import BadgeSelect from '@/pages/grants/components/BadgeSelect';
import type { CsvParseResult, BatchTaskFailure } from '@/types';
import type { UploadFile } from 'antd/es/upload/interface';

const { TextArea } = Input;
const { Text, Title } = Typography;

/**
 * 步骤配置
 */
const steps = [
  { title: '配置', description: '选择徽章和用户' },
  { title: '确认', description: '预览撤销信息' },
  { title: '执行', description: '查看任务进度' },
];

/**
 * 批量撤销页面组件
 */
const BatchRevokePage: React.FC = () => {
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [csvResult, setCsvResult] = useState<CsvParseResult | null>(null);
  const [taskId, setTaskId] = useState<number | null>(null);
  const [fileList, setFileList] = useState<UploadFile[]>([]);

  // 查询任务状态
  const { data: task } = useQuery({
    queryKey: ['batchTask', taskId],
    queryFn: () => getBatchTask(taskId!),
    enabled: !!taskId && currentStep === 2,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (data && (data.status === 'pending' || data.status === 'processing')) {
        return 2000;
      }
      return false;
    },
  });

  // 查询失败明细
  const { data: failuresData } = useQuery({
    queryKey: ['batchTaskFailures', taskId],
    queryFn: () => getBatchTaskFailures(taskId!, { page: 1, pageSize: 10 }),
    enabled: !!taskId && (task?.failureCount ?? 0) > 0,
  });

  // 批量撤销 mutation
  const revokeMutation = useMutation({
    mutationFn: batchRevoke,
    onSuccess: (data) => {
      setTaskId(data.id);
      setCurrentStep(2);
      message.success('任务已提交');
    },
    onError: () => {
      message.error('提交失败');
    },
  });

  /**
   * 处理 CSV 上传
   *
   * 上传后后端解析返回 userIds，直接用于提交任务，不再构造 uploaded:// 伪 URL
   */
  const handleCsvUpload = useCallback(async (file: File) => {
    try {
      const result = await uploadUserCsv(file);
      setCsvResult(result);
      return false;
    } catch {
      message.error('CSV 解析失败');
      return false;
    }
  }, []);

  /**
   * 进入下一步
   */
  const handleNext = async () => {
    if (currentStep === 0) {
      try {
        await form.validateFields();
        if (!csvResult || csvResult.validCount === 0) {
          message.error('请先上传有效的用户列表');
          return;
        }
        setCurrentStep(1);
      } catch {
        // 表单验证失败
      }
    } else if (currentStep === 1) {
      const values = form.getFieldsValue();
      revokeMutation.mutate({
        badgeId: values.badgeId,
        csvRefKey: csvResult?.csvRefKey,
        reason: values.reason,
      });
    }
  };

  /**
   * 返回上一步
   */
  const handlePrev = () => {
    setCurrentStep(currentStep - 1);
  };

  /**
   * 重新开始
   */
  const handleReset = () => {
    setCurrentStep(0);
    setTaskId(null);
    setCsvResult(null);
    setFileList([]);
    form.resetFields();
  };

  /**
   * 渲染配置步骤
   */
  const renderConfigStep = () => (
    <Card>
      <Form form={form} layout="vertical">
        <Form.Item
          name="badgeId"
          label="选择徽章"
          rules={[{ required: true, message: '请选择要撤销的徽章' }]}
        >
          <BadgeSelect placeholder="请选择要撤销的徽章" />
        </Form.Item>

        <Form.Item
          label="用户列表"
          required
          extra="支持 CSV 格式，需包含 user_id 列"
        >
          <Upload
            accept=".csv"
            maxCount={1}
            fileList={fileList}
            beforeUpload={(file) => {
              handleCsvUpload(file);
              setFileList([file]);
              return false;
            }}
            onRemove={() => {
              setCsvResult(null);
              setFileList([]);
            }}
          >
            <Button icon={<UploadOutlined />}>上传 CSV 文件</Button>
          </Upload>
          {csvResult && (
            <Alert
              style={{ marginTop: 12 }}
              type={csvResult.validCount > 0 ? 'success' : 'warning'}
              message={
                <Space direction="vertical" size={0}>
                  <Text>
                    共 {csvResult.totalRows} 行，有效 {csvResult.validCount} 条
                    {csvResult.invalidRows.length > 0 && (
                      <Text type="warning">，无效 {csvResult.invalidRows.length} 条</Text>
                    )}
                  </Text>
                </Space>
              }
            />
          )}
        </Form.Item>

        <Form.Item
          name="reason"
          label="撤销原因"
          rules={[
            { required: true, message: '请输入撤销原因' },
            { min: 2, message: '原因至少 2 个字符' },
          ]}
        >
          <TextArea
            placeholder="请输入撤销原因"
            maxLength={500}
            showCount
            rows={3}
          />
        </Form.Item>
      </Form>

      <div style={{ textAlign: 'right', marginTop: 24 }}>
        <Button type="primary" onClick={handleNext}>
          下一步
        </Button>
      </div>
    </Card>
  );

  /**
   * 渲染确认步骤
   */
  const renderConfirmStep = () => {
    const values = form.getFieldsValue();
    return (
      <Card>
        <Alert
          type="warning"
          showIcon
          message="确认撤销信息"
          description="批量撤销操作不可逆，请仔细核对以下信息"
          style={{ marginBottom: 24 }}
        />

        <Descriptions bordered column={1}>
          <Descriptions.Item label="徽章 ID">{values.badgeId}</Descriptions.Item>
          <Descriptions.Item label="影响用户数">
            <Text strong style={{ color: '#ff4d4f' }}>
              {csvResult?.validCount || 0} 人
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label="撤销原因">{values.reason}</Descriptions.Item>
        </Descriptions>

        <div style={{ textAlign: 'right', marginTop: 24 }}>
          <Space>
            <Button onClick={handlePrev}>上一步</Button>
            <Button
              type="primary"
              danger
              loading={revokeMutation.isPending}
              onClick={handleNext}
              icon={<RollbackOutlined />}
            >
              确认撤销
            </Button>
          </Space>
        </div>
      </Card>
    );
  };

  /**
   * 渲染执行步骤
   */
  const renderExecuteStep = () => {
    if (!task) {
      return (
        <Card>
          <div style={{ textAlign: 'center', padding: 48 }}>
            <LoadingOutlined style={{ fontSize: 32 }} />
            <p style={{ marginTop: 16 }}>加载中...</p>
          </div>
        </Card>
      );
    }

    const isRunning = task.status === 'pending' || task.status === 'processing';
    const isCompleted = task.status === 'completed';
    const isFailed = task.status === 'failed';

    return (
      <Card>
        {isCompleted && task.failureCount === 0 && (
          <Result
            status="success"
            title="撤销完成"
            subTitle={`共撤销 ${task.successCount} 个用户的徽章`}
            extra={
              <Button type="primary" onClick={handleReset}>
                继续撤销
              </Button>
            }
          />
        )}

        {isCompleted && task.failureCount > 0 && (
          <Result
            status="warning"
            title="部分撤销成功"
            subTitle={`成功 ${task.successCount}，失败 ${task.failureCount}`}
            extra={
              <Button type="primary" onClick={handleReset}>
                继续撤销
              </Button>
            }
          />
        )}

        {isFailed && (
          <Result
            status="error"
            title="撤销失败"
            subTitle={task.errorMessage || '任务执行出错'}
            extra={
              <Button type="primary" onClick={handleReset}>
                重新开始
              </Button>
            }
          />
        )}

        {isRunning && (
          <>
            <div style={{ textAlign: 'center', marginBottom: 24 }}>
              <Title level={4}>
                <LoadingOutlined style={{ marginRight: 8 }} />
                正在执行撤销任务
              </Title>
            </div>

            <Progress
              percent={task.progress}
              status="active"
              strokeColor={{ from: '#ff4d4f', to: '#ff7875' }}
              style={{ marginBottom: 24 }}
            />

            <Descriptions bordered size="small">
              <Descriptions.Item label="任务 ID">{task.id}</Descriptions.Item>
              <Descriptions.Item label="总数">{task.totalCount}</Descriptions.Item>
              <Descriptions.Item label="已处理">
                {task.successCount + task.failureCount}
              </Descriptions.Item>
              <Descriptions.Item label="成功">
                <Text type="success">{task.successCount}</Text>
              </Descriptions.Item>
              <Descriptions.Item label="失败">
                <Text type="danger">{task.failureCount}</Text>
              </Descriptions.Item>
              <Descriptions.Item label="进度">{task.progress}%</Descriptions.Item>
            </Descriptions>
          </>
        )}

        {/* 失败明细 */}
        {failuresData && failuresData.items && failuresData.items.length > 0 && (
          <Card
            size="small"
            title="失败明细"
            style={{ marginTop: 24 }}
          >
            <Table<BatchTaskFailure>
              dataSource={failuresData.items}
              rowKey="id"
              size="small"
              pagination={false}
              columns={[
                { title: '行号', dataIndex: 'rowNumber', width: 70 },
                { title: '用户 ID', dataIndex: 'userId' },
                { title: '错误信息', dataIndex: 'errorMessage' },
                {
                  title: '重试状态',
                  dataIndex: 'retryStatus',
                  render: (status: string) => (
                    <Tag color={status === 'SUCCESS' ? 'success' : status === 'PENDING' ? 'orange' : 'error'}>
                      {status}
                    </Tag>
                  ),
                },
              ]}
            />
          </Card>
        )}
      </Card>
    );
  };

  return (
    <PageContainer
      title="批量撤销"
      extra={
        currentStep === 2 && task && (task.status === 'completed' || task.status === 'failed') ? (
          <Button onClick={handleReset} icon={<FileExcelOutlined />}>
            新建任务
          </Button>
        ) : null
      }
    >
      <Card style={{ marginBottom: 24 }}>
        <Steps
          current={currentStep}
          items={steps.map((step, index) => ({
            ...step,
            icon: index === currentStep && currentStep === 2 && task?.status === 'processing' ? (
              <LoadingOutlined />
            ) : index < currentStep || (index === 2 && task?.status === 'completed') ? (
              <CheckCircleOutlined />
            ) : undefined,
          }))}
        />
      </Card>

      {currentStep === 0 && renderConfigStep()}
      {currentStep === 1 && renderConfirmStep()}
      {currentStep === 2 && renderExecuteStep()}
    </PageContainer>
  );
};

export default BatchRevokePage;

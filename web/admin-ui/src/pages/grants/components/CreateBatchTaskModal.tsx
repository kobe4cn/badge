/**
 * 新建批量任务弹窗
 *
 * 支持两种创建模式：
 * 1. CSV 上传模式：上传包含用户 ID 的 CSV 文件
 * 2. 条件筛选模式：设置筛选条件选择目标用户
 */

import React, { useState, useCallback } from 'react';
import {
  Modal,
  Form,
  Input,
  InputNumber,
  Select,
  Upload,
  Radio,
  Space,
  Alert,
  Typography,
  DatePicker,
  Spin,
  Divider,
  List,
  Avatar,
  Button,
  message,
} from 'antd';
import {
  InboxOutlined,
  UserOutlined,
  FilterOutlined,
  UploadOutlined,
  CheckCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { UploadFile, UploadProps } from 'antd/es/upload';
import BadgeSelect, { type BadgeSelectValue } from './BadgeSelect';
import {
  useCreateBatchTask,
  useUploadUserCsv,
  usePreviewUserFilter,
} from '@/hooks/useGrant';
import { MEMBERSHIP_LEVELS } from '@/types/user';
import type { CsvParseResult, UserFilterCondition, UserFilterPreview } from '@/types';

const { Text } = Typography;
const { TextArea } = Input;
const { Dragger } = Upload;
const { RangePicker } = DatePicker;

type CreateMode = 'csv' | 'filter';

interface CreateBatchTaskModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

interface FormValues {
  name: string;
  badge: BadgeSelectValue;
  quantity: number;
  reason?: string;
}

/**
 * 新建批量任务弹窗组件
 */
const CreateBatchTaskModal: React.FC<CreateBatchTaskModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [form] = Form.useForm<FormValues>();
  const [mode, setMode] = useState<CreateMode>('csv');
  const [fileList, setFileList] = useState<UploadFile[]>([]);
  const [csvResult, setCsvResult] = useState<CsvParseResult | null>(null);
  const [filterCondition, setFilterCondition] = useState<UserFilterCondition>({});
  const [filterPreview, setFilterPreview] = useState<UserFilterPreview | null>(null);

  const { mutateAsync: createTask, isPending: isCreating } = useCreateBatchTask();
  const { mutateAsync: uploadCsv, isPending: isUploading } = useUploadUserCsv();
  const { mutateAsync: previewFilter, isPending: isPreviewing } = usePreviewUserFilter();

  /**
   * 重置弹窗状态
   */
  const resetState = useCallback(() => {
    form.resetFields();
    setMode('csv');
    setFileList([]);
    setCsvResult(null);
    setFilterCondition({});
    setFilterPreview(null);
  }, [form]);

  /**
   * 关闭弹窗
   */
  const handleClose = useCallback(() => {
    resetState();
    onClose();
  }, [resetState, onClose]);

  /**
   * 处理 CSV 文件上传
   */
  const handleCsvUpload: UploadProps['customRequest'] = async (options) => {
    const { file, onSuccess: uploadSuccess, onError } = options;
    try {
      const result = await uploadCsv(file as File);
      setCsvResult(result);
      uploadSuccess?.({});
    } catch (error) {
      onError?.(error as Error);
    }
  };

  /**
   * 处理文件列表变更
   */
  const handleFileChange: UploadProps['onChange'] = ({ fileList: newFileList }) => {
    setFileList(newFileList.slice(-1));
    if (newFileList.length === 0) {
      setCsvResult(null);
    }
  };

  /**
   * 处理筛选条件变更并预览
   */
  const handleFilterChange = async (newCondition: UserFilterCondition) => {
    setFilterCondition(newCondition);

    // 只有当有筛选条件时才预览
    const hasCondition = Object.values(newCondition).some(
      (v) => v !== undefined && (Array.isArray(v) ? v.length > 0 : true)
    );

    if (hasCondition) {
      try {
        const preview = await previewFilter(newCondition);
        setFilterPreview(preview);
      } catch {
        setFilterPreview(null);
      }
    } else {
      setFilterPreview(null);
    }
  };

  /**
   * 提交创建任务
   */
  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();

      // 验证用户数据
      if (mode === 'csv') {
        if (!csvResult || csvResult.userIds.length === 0) {
          message.error('请上传有效的 CSV 文件');
          return;
        }
      } else {
        if (!filterPreview || filterPreview.count === 0) {
          message.error('请设置筛选条件，且至少匹配一个用户');
          return;
        }
      }

      await createTask({
        name: values.name,
        badgeId: values.badge.value,
        quantity: values.quantity,
        reason: values.reason,
        userIds: mode === 'csv' ? csvResult?.userIds : undefined,
        userFilter: mode === 'filter' ? filterCondition : undefined,
      });

      handleClose();
      onSuccess?.();
    } catch {
      // 表单验证失败或创建失败，错误已在 hook 中处理
    }
  };

  /**
   * 渲染 CSV 上传模式内容
   */
  const renderCsvMode = () => (
    <div>
      <Dragger
        accept=".csv"
        fileList={fileList}
        customRequest={handleCsvUpload}
        onChange={handleFileChange}
        maxCount={1}
        disabled={isUploading}
      >
        <p className="ant-upload-drag-icon">
          {isUploading ? <Spin /> : <InboxOutlined />}
        </p>
        <p className="ant-upload-text">点击或拖拽 CSV 文件到此区域</p>
        <p className="ant-upload-hint">
          CSV 文件应包含用户 ID 列，支持第一行作为表头
        </p>
      </Dragger>

      {csvResult && (
        <Alert
          style={{ marginTop: 16 }}
          type={csvResult.invalidRows.length > 0 ? 'warning' : 'success'}
          icon={
            csvResult.invalidRows.length > 0 ? (
              <WarningOutlined />
            ) : (
              <CheckCircleOutlined />
            )
          }
          message={
            <Space direction="vertical" size={4}>
              <Text>
                解析完成：共 {csvResult.totalRows} 行，有效用户{' '}
                <Text strong type="success">
                  {csvResult.userIds.length}
                </Text>{' '}
                个
              </Text>
              {csvResult.invalidRows.length > 0 && (
                <Text type="warning">
                  无效行：{csvResult.invalidRows.slice(0, 5).join(', ')}
                  {csvResult.invalidRows.length > 5 &&
                    ` 等 ${csvResult.invalidRows.length} 行`}
                </Text>
              )}
            </Space>
          }
        />
      )}
    </div>
  );

  /**
   * 渲染条件筛选模式内容
   */
  const renderFilterMode = () => (
    <div>
      <Form layout="vertical" size="small">
        <Form.Item label="会员等级">
          <Select
            mode="multiple"
            placeholder="选择会员等级（可多选）"
            options={MEMBERSHIP_LEVELS.map((level) => ({
              label: level.name,
              value: level.level,
            }))}
            value={filterCondition.membershipLevels}
            onChange={(levels) =>
              handleFilterChange({
                ...filterCondition,
                membershipLevels: levels.length > 0 ? levels : undefined,
              })
            }
            allowClear
          />
        </Form.Item>

        <Form.Item label="注册时间">
          <RangePicker
            style={{ width: '100%' }}
            placeholder={['注册起始日期', '注册结束日期']}
            onChange={(dates) =>
              handleFilterChange({
                ...filterCondition,
                registeredAfter: dates?.[0]?.toISOString(),
                registeredBefore: dates?.[1]?.toISOString(),
              })
            }
          />
        </Form.Item>

        <Form.Item label="最低消费金额">
          <InputNumber
            style={{ width: '100%' }}
            min={0}
            placeholder="输入最低消费金额"
            value={filterCondition.minTotalSpent}
            onChange={(value) =>
              handleFilterChange({
                ...filterCondition,
                minTotalSpent: value ?? undefined,
              })
            }
            addonAfter="元"
          />
        </Form.Item>

        <Form.Item label="最低订单数">
          <InputNumber
            style={{ width: '100%' }}
            min={0}
            placeholder="输入最低订单数"
            value={filterCondition.minOrderCount}
            onChange={(value) =>
              handleFilterChange({
                ...filterCondition,
                minOrderCount: value ?? undefined,
              })
            }
            addonAfter="单"
          />
        </Form.Item>
      </Form>

      {isPreviewing && (
        <div style={{ textAlign: 'center', padding: 16 }}>
          <Spin tip="正在筛选用户..." />
        </div>
      )}

      {filterPreview && !isPreviewing && (
        <Alert
          type={filterPreview.count > 0 ? 'info' : 'warning'}
          message={
            <Text>
              符合条件的用户：
              <Text strong type={filterPreview.count > 0 ? 'success' : 'warning'}>
                {filterPreview.count}
              </Text>{' '}
              人
            </Text>
          }
          description={
            filterPreview.count > 0 && (
              <List
                size="small"
                dataSource={filterPreview.users}
                renderItem={(user) => (
                  <List.Item style={{ padding: '4px 0' }}>
                    <Space>
                      <Avatar size="small" icon={<UserOutlined />} />
                      <Text>{user.username}</Text>
                      <Text type="secondary">({user.userId})</Text>
                    </Space>
                  </List.Item>
                )}
                footer={
                  filterPreview.count > filterPreview.users.length && (
                    <Text type="secondary">
                      ... 还有 {filterPreview.count - filterPreview.users.length} 位用户
                    </Text>
                  )
                }
              />
            )
          }
        />
      )}
    </div>
  );

  /**
   * 获取用户数量用于确认按钮显示
   */
  const getUserCount = (): number => {
    if (mode === 'csv') {
      return csvResult?.userIds.length || 0;
    }
    return filterPreview?.count || 0;
  };

  const userCount = getUserCount();

  return (
    <Modal
      title="新建批量任务"
      open={open}
      onCancel={handleClose}
      width={640}
      footer={[
        <Button key="cancel" onClick={handleClose}>
          取消
        </Button>,
        <Button
          key="submit"
          type="primary"
          loading={isCreating}
          onClick={handleSubmit}
          disabled={userCount === 0}
        >
          创建任务 {userCount > 0 && `(${userCount} 人)`}
        </Button>,
      ]}
      destroyOnClose
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{ quantity: 1 }}
      >
        <Form.Item
          name="name"
          label="任务名称"
          rules={[{ required: true, message: '请输入任务名称' }]}
        >
          <Input placeholder="请输入任务名称，便于后续查找" maxLength={50} />
        </Form.Item>

        <Form.Item
          name="badge"
          label="选择徽章"
          rules={[{ required: true, message: '请选择要发放的徽章' }]}
        >
          <BadgeSelect placeholder="请选择要发放的徽章" />
        </Form.Item>

        <Form.Item
          name="quantity"
          label="每人发放数量"
          rules={[{ required: true, message: '请输入发放数量' }]}
        >
          <InputNumber min={1} max={99} style={{ width: 200 }} addonAfter="个/人" />
        </Form.Item>

        <Form.Item name="reason" label="发放原因（可选）">
          <TextArea
            placeholder="请输入发放原因，将记录在发放日志中"
            rows={2}
            maxLength={200}
            showCount
          />
        </Form.Item>

        <Divider />

        <Form.Item label="选择用户">
          <Radio.Group
            value={mode}
            onChange={(e) => {
              setMode(e.target.value);
              setCsvResult(null);
              setFileList([]);
              setFilterPreview(null);
            }}
            style={{ marginBottom: 16 }}
          >
            <Radio.Button value="csv">
              <Space>
                <UploadOutlined />
                CSV 上传
              </Space>
            </Radio.Button>
            <Radio.Button value="filter">
              <Space>
                <FilterOutlined />
                条件筛选
              </Space>
            </Radio.Button>
          </Radio.Group>

          {mode === 'csv' ? renderCsvMode() : renderFilterMode()}
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default CreateBatchTaskModal;

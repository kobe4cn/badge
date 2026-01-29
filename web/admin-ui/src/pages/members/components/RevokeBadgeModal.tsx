/**
 * 撤销徽章弹窗
 *
 * 确认撤销信息并填写撤销原因
 */

import React from 'react';
import {
  Modal,
  Form,
  Input,
  Alert,
  Space,
  Avatar,
  Typography,
  Descriptions,
} from 'antd';
import {
  ExclamationCircleOutlined,
  TrophyOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { UserBadgeDetail } from '@/types';
import dayjs from 'dayjs';

const { TextArea } = Input;
const { Text } = Typography;

interface RevokeBadgeModalProps {
  /** 是否显示 */
  open: boolean;
  /** 要撤销的徽章 */
  badge: UserBadgeDetail | null;
  /** 是否正在提交 */
  loading?: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 确认撤销回调 */
  onConfirm: (userBadgeId: number, reason: string) => void;
}

interface FormValues {
  reason: string;
}

/**
 * 撤销徽章弹窗组件
 */
const RevokeBadgeModal: React.FC<RevokeBadgeModalProps> = ({
  open,
  badge,
  loading = false,
  onClose,
  onConfirm,
}) => {
  const [form] = Form.useForm<FormValues>();

  /**
   * 处理提交
   */
  const handleSubmit = async () => {
    if (!badge) return;

    try {
      const values = await form.validateFields();
      onConfirm(badge.id, values.reason);
    } catch {
      // 校验失败，不处理
    }
  };

  /**
   * 处理关闭
   */
  const handleClose = () => {
    form.resetFields();
    onClose();
  };

  if (!badge) return null;

  return (
    <Modal
      title={
        <Space>
          <ExclamationCircleOutlined style={{ color: '#faad14' }} />
          撤销徽章
        </Space>
      }
      open={open}
      onCancel={handleClose}
      onOk={handleSubmit}
      okText="确认撤销"
      okButtonProps={{ danger: true, loading }}
      cancelText="取消"
      width={500}
    >
      <Alert
        message="撤销操作不可恢复"
        description="撤销后，该用户将无法再使用此徽章。如需重新发放，需要进行新的发放操作。"
        type="warning"
        showIcon
        icon={<WarningOutlined />}
        style={{ marginBottom: 24 }}
      />

      {/* 徽章信息 */}
      <div
        style={{
          padding: 16,
          backgroundColor: '#fafafa',
          borderRadius: 8,
          marginBottom: 24,
        }}
      >
        <Space align="start">
          <Avatar
            src={badge.badgeIcon}
            size={48}
            shape="square"
            icon={<TrophyOutlined />}
          />
          <div>
            <Text strong style={{ fontSize: 16, display: 'block' }}>
              {badge.badgeName}
            </Text>
            <Descriptions size="small" column={1} style={{ marginTop: 8 }}>
              <Descriptions.Item label="持有数量">
                {badge.quantity} 个
              </Descriptions.Item>
              <Descriptions.Item label="获取时间">
                {dayjs(badge.grantedAt).format('YYYY-MM-DD HH:mm')}
              </Descriptions.Item>
            </Descriptions>
          </div>
        </Space>
      </div>

      {/* 撤销原因表单 */}
      <Form form={form} layout="vertical">
        <Form.Item
          name="reason"
          label="撤销原因"
          rules={[
            { required: true, message: '请填写撤销原因' },
            { min: 2, message: '撤销原因至少 2 个字符' },
            { max: 200, message: '撤销原因最多 200 个字符' },
          ]}
        >
          <TextArea
            placeholder="请填写撤销原因，将记录在操作日志中"
            rows={3}
            maxLength={200}
            showCount
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default RevokeBadgeModal;

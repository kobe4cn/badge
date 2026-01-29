/**
 * 徽章节点配置弹窗
 *
 * 用于配置发放的徽章和数量。
 * 支持从徽章列表中选择和设置发放数量。
 */

import React, { useEffect, useState } from 'react';
import { Modal, Form, Select, InputNumber, Spin } from 'antd';
import type { BadgeNodeData } from '../../../../types/rule-canvas';
import { badgeService } from '../../../../services/badge';
import type { Badge } from '../../../../types/badge';

export interface BadgeNodeConfigProps {
  open: boolean;
  data: BadgeNodeData;
  onSave: (data: BadgeNodeData) => void;
  onCancel: () => void;
}

const BadgeNodeConfig: React.FC<BadgeNodeConfigProps> = ({
  open,
  data,
  onSave,
  onCancel,
}) => {
  const [form] = Form.useForm();
  const [badges, setBadges] = useState<Badge[]>([]);
  const [loading, setLoading] = useState(false);

  // 加载徽章列表
  useEffect(() => {
    if (open && badges.length === 0) {
      setLoading(true);
      badgeService
        .getAll()
        .then((response) => {
          // API 返回 PaginatedResponse，数据在 items 字段中
          setBadges(response.items || []);
        })
        .catch(() => {
          // 加载失败时使用空列表
          setBadges([]);
        })
        .finally(() => {
          setLoading(false);
        });
    }
  }, [open, badges.length]);

  // 弹窗打开时初始化表单
  useEffect(() => {
    if (open) {
      form.setFieldsValue({
        badgeId: data.badgeId,
        quantity: data.quantity || 1,
      });
    }
  }, [open, data, form]);

  const handleOk = () => {
    form.validateFields().then((values) => {
      const selectedBadge = badges.find((b) => String(b.id) === values.badgeId);
      onSave({
        badgeId: values.badgeId,
        badgeName: selectedBadge?.name || '未知徽章',
        quantity: values.quantity,
      });
    });
  };

  // 徽章选项
  const badgeOptions = badges.map((badge) => ({
    label: badge.name,
    value: String(badge.id),
  }));

  return (
    <Modal
      title="配置发放徽章"
      open={open}
      onOk={handleOk}
      onCancel={onCancel}
      destroyOnClose
      width={400}
    >
      <Spin spinning={loading}>
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          <Form.Item
            name="badgeId"
            label="选择徽章"
            rules={[{ required: true, message: '请选择要发放的徽章' }]}
          >
            <Select
              placeholder="请选择徽章"
              options={badgeOptions}
              showSearch
              optionFilterProp="label"
              loading={loading}
              notFoundContent={loading ? <Spin size="small" /> : '暂无徽章'}
            />
          </Form.Item>

          <Form.Item
            name="quantity"
            label="发放数量"
            rules={[{ required: true, message: '请输入发放数量' }]}
            extra="用户满足条件时发放的徽章数量"
          >
            <InputNumber min={1} max={100} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Spin>
    </Modal>
  );
};

export default BadgeNodeConfig;

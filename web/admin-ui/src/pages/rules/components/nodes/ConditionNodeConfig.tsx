/**
 * 条件节点配置弹窗
 *
 * 用于配置条件节点的字段、操作符和比较值。
 * 根据选择的操作符动态调整值输入组件的类型。
 */

import React, { useEffect, useMemo } from 'react';
import { Modal, Form, Select, Input, InputNumber, Space } from 'antd';
import type { ConditionNodeData, ConditionOperator } from '../../../../types/rule-canvas';
import { OPERATOR_CONFIG, PRESET_FIELDS } from '../../../../types/rule-canvas';

export interface ConditionNodeConfigProps {
  open: boolean;
  data: ConditionNodeData;
  onSave: (data: ConditionNodeData) => void;
  onCancel: () => void;
}

const ConditionNodeConfig: React.FC<ConditionNodeConfigProps> = ({
  open,
  data,
  onSave,
  onCancel,
}) => {
  const [form] = Form.useForm();

  // 按分类分组的字段选项
  const fieldOptions = useMemo(() => {
    const categories = {
      event: { label: '事件属性', options: [] as { label: string; value: string }[] },
      user: { label: '用户属性', options: [] as { label: string; value: string }[] },
      order: { label: '订单属性', options: [] as { label: string; value: string }[] },
      time: { label: '时间条件', options: [] as { label: string; value: string }[] },
    };

    PRESET_FIELDS.forEach((field) => {
      categories[field.category].options.push({
        label: field.label,
        value: field.field,
      });
    });

    return Object.values(categories).filter((cat) => cat.options.length > 0);
  }, []);

  // 操作符选项
  const operatorOptions = useMemo(() => {
    return Object.entries(OPERATOR_CONFIG).map(([value, config]) => ({
      label: config.label,
      value,
    }));
  }, []);

  // 监听表单值变化，获取当前选中的操作符
  const watchedOperator = Form.useWatch('operator', form);

  // 根据操作符确定值输入类型
  const valueType = useMemo(() => {
    if (!watchedOperator) return 'single';
    return OPERATOR_CONFIG[watchedOperator as ConditionOperator]?.valueType || 'single';
  }, [watchedOperator]);

  // 弹窗打开时初始化表单
  useEffect(() => {
    if (open) {
      form.setFieldsValue({
        field: data.field,
        operator: data.operator,
        value: data.value,
      });
    }
  }, [open, data, form]);

  const handleOk = () => {
    form.validateFields().then((values) => {
      // 查找字段的显示名称
      const fieldConfig = PRESET_FIELDS.find((f) => f.field === values.field);
      onSave({
        ...values,
        fieldLabel: fieldConfig?.label,
      });
    });
  };

  /**
   * 渲染值输入组件
   *
   * 根据操作符类型返回对应的输入控件
   */
  const renderValueInput = () => {
    switch (valueType) {
      case 'none':
        // is_null / is_not_null 不需要值输入
        return null;

      case 'range':
        // between 需要两个值
        return (
          <Space>
            <Form.Item
              name={['value', 0]}
              noStyle
              rules={[{ required: true, message: '请输入起始值' }]}
            >
              <InputNumber placeholder="起始值" style={{ width: 100 }} />
            </Form.Item>
            <span style={{ color: '#8c8c8c' }}>至</span>
            <Form.Item
              name={['value', 1]}
              noStyle
              rules={[{ required: true, message: '请输入结束值' }]}
            >
              <InputNumber placeholder="结束值" style={{ width: 100 }} />
            </Form.Item>
          </Space>
        );

      case 'list':
        // in / not_in 需要多个值
        return (
          <Form.Item
            name="value"
            rules={[{ required: true, message: '请输入值列表' }]}
          >
            <Select
              mode="tags"
              placeholder="输入值并按回车添加"
              style={{ width: '100%' }}
              tokenSeparators={[',']}
            />
          </Form.Item>
        );

      default:
        // 单值输入
        return (
          <Form.Item
            name="value"
            rules={[{ required: true, message: '请输入比较值' }]}
          >
            <Input placeholder="请输入比较值" />
          </Form.Item>
        );
    }
  };

  return (
    <Modal
      title="配置条件"
      open={open}
      onOk={handleOk}
      onCancel={onCancel}
      destroyOnClose
      width={480}
    >
      <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
        <Form.Item
          name="field"
          label="字段"
          rules={[{ required: true, message: '请选择字段' }]}
        >
          <Select
            placeholder="请选择字段"
            options={fieldOptions}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>

        <Form.Item
          name="operator"
          label="操作符"
          rules={[{ required: true, message: '请选择操作符' }]}
        >
          <Select placeholder="请选择操作符" options={operatorOptions} />
        </Form.Item>

        {valueType !== 'none' && (
          <Form.Item label="比较值">
            {renderValueInput()}
          </Form.Item>
        )}
      </Form>
    </Modal>
  );
};

export default ConditionNodeConfig;

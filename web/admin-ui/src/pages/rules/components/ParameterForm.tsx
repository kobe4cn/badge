/**
 * 参数表单组件
 *
 * 根据模板参数定义动态生成表单，支持多种参数类型
 */

import React, { useEffect } from 'react';
import {
  Form,
  Input,
  InputNumber,
  Switch,
  Select,
  DatePicker,
  Card,
  Typography,
} from 'antd';
import type { ParameterDef } from '@/services/template';

const { Text } = Typography;

interface ParameterFormProps {
  /** 参数定义列表 */
  parameters: ParameterDef[];
  /** 参数值变更回调 */
  onChange: (values: Record<string, unknown>) => void;
  /** 初始参数值 */
  initialValues?: Record<string, unknown>;
}

export const ParameterForm: React.FC<ParameterFormProps> = ({
  parameters,
  onChange,
  initialValues,
}) => {
  const [form] = Form.useForm();

  useEffect(() => {
    // 初始值优先级：用户提供的值 > 参数默认值
    const defaults: Record<string, unknown> = {};
    parameters.forEach((param) => {
      if (initialValues?.[param.name] !== undefined) {
        defaults[param.name] = initialValues[param.name];
      } else if (param.default !== undefined) {
        defaults[param.name] = param.default;
      }
    });
    form.setFieldsValue(defaults);
    onChange(defaults);
  }, [parameters, initialValues]);

  /**
   * 根据参数类型渲染对应的表单控件
   *
   * 支持 string、number、boolean、enum、date、array 六种类型
   */
  const renderField = (param: ParameterDef) => {
    const rules = param.required
      ? [{ required: true, message: `请输入${param.label}` }]
      : [];

    const commonProps = {
      key: param.name,
      name: param.name,
      label: (
        <span>
          {param.label}
          {param.description && (
            <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
              ({param.description})
            </Text>
          )}
        </span>
      ),
      rules,
    };

    switch (param.type) {
      case 'number':
        return (
          <Form.Item {...commonProps}>
            <InputNumber
              min={param.min}
              max={param.max}
              style={{ width: '100%' }}
              placeholder={`请输入${param.label}`}
            />
          </Form.Item>
        );

      case 'boolean':
        return (
          <Form.Item {...commonProps} valuePropName="checked">
            <Switch />
          </Form.Item>
        );

      case 'enum':
        return (
          <Form.Item {...commonProps}>
            <Select
              placeholder={`请选择${param.label}`}
              options={param.options?.map((o) => ({
                value: o.value,
                label: o.label,
              }))}
            />
          </Form.Item>
        );

      case 'date':
        return (
          <Form.Item {...commonProps}>
            <DatePicker
              style={{ width: '100%' }}
              showTime
              placeholder={`请选择${param.label}`}
            />
          </Form.Item>
        );

      case 'array':
        return (
          <Form.Item {...commonProps}>
            <Select
              mode="tags"
              placeholder={`请输入${param.label}，回车添加`}
              style={{ width: '100%' }}
            />
          </Form.Item>
        );

      case 'string':
      default:
        // 带有选项的 string 类型使用下拉选择
        if (param.options && param.options.length > 0) {
          return (
            <Form.Item {...commonProps}>
              <Select
                placeholder={`请选择${param.label}`}
                options={param.options.map((o) => ({
                  value: o.value,
                  label: o.label,
                }))}
              />
            </Form.Item>
          );
        }
        return (
          <Form.Item {...commonProps}>
            <Input placeholder={`请输入${param.label}`} />
          </Form.Item>
        );
    }
  };

  if (parameters.length === 0) {
    return (
      <Card size="small">
        <Text type="secondary">此模板无需配置参数</Text>
      </Card>
    );
  }

  return (
    <Card title="配置参数" size="small">
      <Form
        form={form}
        layout="vertical"
        onValuesChange={(_, allValues) => onChange(allValues)}
      >
        {parameters.map(renderField)}
      </Form>
    </Card>
  );
};

export default ParameterForm;

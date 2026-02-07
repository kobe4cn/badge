/**
 * 权益表单弹窗组件
 *
 * 支持新建和编辑两种模式，编辑时 code 和 benefitType 不可修改
 * 因为权益编码是业务唯一标识，类型变更会影响已有发放逻辑
 */

import React from 'react';
import {
  ModalForm,
  ProFormText,
  ProFormTextArea,
  ProFormSelect,
  ProFormDigit,
} from '@ant-design/pro-components';
import type {
  Benefit,
  CreateBenefitRequest,
} from '@/services/benefit';

interface BenefitFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues?: Benefit;
  onSubmit: (values: CreateBenefitRequest) => Promise<boolean>;
  loading?: boolean;
}

const BenefitForm: React.FC<BenefitFormProps> = ({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  loading,
}) => {
  const isEdit = !!initialValues;

  return (
    <ModalForm<CreateBenefitRequest>
      title={isEdit ? '编辑权益' : '新建权益'}
      open={open}
      onOpenChange={onOpenChange}
      initialValues={
        initialValues
          ? {
              code: initialValues.code,
              name: initialValues.name,
              description: initialValues.description,
              benefitType: initialValues.benefitType,
              externalId: initialValues.externalId,
              externalSystem: initialValues.externalSystem,
              totalStock: initialValues.totalStock,
              iconUrl: initialValues.iconUrl,
            }
          : undefined
      }
      modalProps={{
        destroyOnClose: true,
        maskClosable: false,
      }}
      submitter={{
        searchConfig: {
          submitText: '提交',
          resetText: '取消',
        },
      }}
      submitTimeout={3000}
      loading={loading}
      onFinish={async (values) => {
        return await onSubmit(values);
      }}
    >
      <ProFormText
        name="code"
        label="权益编码"
        placeholder="请输入权益编码"
        disabled={isEdit}
        rules={[
          { required: true, message: '请输入权益编码' },
          { max: 50, message: '权益编码不能超过50个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 50,
        }}
      />

      <ProFormText
        name="name"
        label="权益名称"
        placeholder="请输入权益名称"
        rules={[
          { required: true, message: '请输入权益名称' },
          { max: 100, message: '权益名称不能超过100个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 100,
        }}
      />

      <ProFormTextArea
        name="description"
        label="描述"
        placeholder="请输入权益描述（可选）"
        rules={[
          { max: 500, message: '描述不能超过500个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 500,
        }}
      />

      <ProFormSelect
        name="benefitType"
        label="权益类型"
        placeholder="请选择权益类型"
        rules={[{ required: true, message: '请选择权益类型' }]}
        options={[
          { value: 'POINTS', label: '积分' },
          { value: 'COUPON', label: '优惠券' },
          { value: 'PHYSICAL', label: '实物' },
          { value: 'VIRTUAL', label: '虚拟物品' },
          { value: 'THIRD_PARTY', label: '第三方' },
        ]}
      />

      <ProFormText
        name="externalId"
        label="外部 ID"
        placeholder="请输入外部平台权益 ID（可选）"
        tooltip="外部平台权益 ID，对接时使用"
      />

      <ProFormText
        name="externalSystem"
        label="外部系统"
        placeholder="请输入外部平台名称（可选）"
        tooltip="外部平台名称，如 coupon-service"
      />

      <ProFormDigit
        name="totalStock"
        label="总库存"
        placeholder="不填则为无限制"
        min={0}
        fieldProps={{
          precision: 0,
        }}
      />

      <ProFormText
        name="iconUrl"
        label="图标 URL"
        placeholder="请输入权益图标 URL（可选）"
        rules={[
          {
            type: 'url',
            message: '请输入有效的 URL 地址',
          },
        ]}
      />
    </ModalForm>
  );
};

export default BenefitForm;

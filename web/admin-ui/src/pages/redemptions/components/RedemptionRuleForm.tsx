/**
 * 兑换规则表单弹窗组件
 *
 * 支持新建和编辑两种模式，频率配置在表单中扁平展示，提交时组装为嵌套结构
 */

import React from 'react';
import { Divider } from 'antd';
import {
  ModalForm,
  ProFormText,
  ProFormTextArea,
  ProFormDigit,
  ProFormList,
  ProFormGroup,
  ProFormDateTimePicker,
  ProFormSwitch,
} from '@ant-design/pro-components';
import type {
  RedemptionRule,
  CreateRedemptionRuleRequest,
} from '@/services/redemption';

interface RedemptionRuleFormProps {
  /** 弹窗是否可见 */
  open: boolean;
  /** 关闭弹窗回调 */
  onOpenChange: (open: boolean) => void;
  /** 编辑时传入现有规则数据，新建时为 undefined */
  initialValues?: RedemptionRule;
  /** 表单提交回调 */
  onSubmit: (values: CreateRedemptionRuleRequest) => Promise<boolean>;
  /** 提交中状态 */
  loading?: boolean;
}

/**
 * 表单内部数据结构
 *
 * 频率配置在表单层面是扁平字段，提交时需要组装为 FrequencyConfig 嵌套对象
 */
interface RedemptionRuleFormData {
  name: string;
  description?: string;
  benefitId: number;
  requiredBadges: Array<{ badgeId: number; quantity: number }>;
  maxPerUser?: number;
  maxPerDay?: number;
  maxPerWeek?: number;
  maxPerMonth?: number;
  maxPerYear?: number;
  startTime?: string;
  endTime?: string;
  autoRedeem?: boolean;
}

const RedemptionRuleForm: React.FC<RedemptionRuleFormProps> = ({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  loading,
}) => {
  const isEdit = !!initialValues;

  return (
    <ModalForm<RedemptionRuleFormData>
      title={isEdit ? '编辑兑换规则' : '新建兑换规则'}
      open={open}
      onOpenChange={onOpenChange}
      width={600}
      initialValues={
        initialValues
          ? {
              name: initialValues.name,
              description: initialValues.description,
              benefitId: initialValues.benefitId,
              requiredBadges: initialValues.requiredBadges.map((b) => ({
                badgeId: b.badgeId,
                quantity: b.quantity,
              })),
              maxPerUser: initialValues.frequencyConfig?.maxPerUser,
              maxPerDay: initialValues.frequencyConfig?.maxPerDay,
              maxPerWeek: initialValues.frequencyConfig?.maxPerWeek,
              maxPerMonth: initialValues.frequencyConfig?.maxPerMonth,
              maxPerYear: initialValues.frequencyConfig?.maxPerYear,
              startTime: initialValues.startTime,
              endTime: initialValues.endTime,
              autoRedeem: initialValues.autoRedeem,
            }
          : { autoRedeem: false }
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
        // 将表单扁平的频率字段组装为嵌套对象，后端接口要求此结构
        const frequencyConfig = {
          maxPerUser: values.maxPerUser,
          maxPerDay: values.maxPerDay,
          maxPerWeek: values.maxPerWeek,
          maxPerMonth: values.maxPerMonth,
          maxPerYear: values.maxPerYear,
        };

        const result = await onSubmit({
          name: values.name,
          description: values.description || undefined,
          benefitId: values.benefitId,
          requiredBadges: values.requiredBadges,
          frequencyConfig,
          startTime: values.startTime || undefined,
          endTime: values.endTime || undefined,
          autoRedeem: values.autoRedeem,
        });
        return result;
      }}
    >
      <ProFormText
        name="name"
        label="规则名称"
        placeholder="请输入规则名称"
        rules={[
          { required: true, message: '请输入规则名称' },
          { max: 100, message: '规则名称不能超过100个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 100,
        }}
      />

      <ProFormTextArea
        name="description"
        label="描述"
        placeholder="请输入规则描述（可选）"
        fieldProps={{
          showCount: true,
          maxLength: 500,
        }}
      />

      <ProFormDigit
        name="benefitId"
        label="权益 ID"
        placeholder="请输入关联权益 ID"
        tooltip="关联权益 ID"
        rules={[{ required: true, message: '请输入权益 ID' }]}
        fieldProps={{
          precision: 0,
        }}
      />

      <ProFormList
        name="requiredBadges"
        label="所需徽章"
        creatorButtonProps={{
          creatorButtonText: '添加所需徽章',
        }}
        rules={[
          {
            required: true,
            message: '请至少添加一个所需徽章',
            validator: async (_, value) => {
              if (!value || value.length === 0) {
                throw new Error('请至少添加一个所需徽章');
              }
            },
          },
        ]}
      >
        <ProFormGroup>
          <ProFormDigit
            name="badgeId"
            label="徽章 ID"
            placeholder="徽章 ID"
            rules={[{ required: true, message: '请输入徽章 ID' }]}
            fieldProps={{ precision: 0 }}
            width="sm"
          />
          <ProFormDigit
            name="quantity"
            label="数量"
            placeholder="数量"
            rules={[{ required: true, message: '请输入数量' }]}
            min={1}
            initialValue={1}
            fieldProps={{ precision: 0 }}
            width="sm"
          />
        </ProFormGroup>
      </ProFormList>

      <Divider>频率限制</Divider>

      <ProFormDigit
        name="maxPerUser"
        label="每用户上限"
        placeholder="不限制"
        min={1}
        fieldProps={{ precision: 0 }}
      />

      <ProFormDigit
        name="maxPerDay"
        label="每日上限"
        placeholder="不限制"
        min={1}
        fieldProps={{ precision: 0 }}
      />

      <ProFormDigit
        name="maxPerWeek"
        label="每周上限"
        placeholder="不限制"
        min={1}
        fieldProps={{ precision: 0 }}
      />

      <ProFormDigit
        name="maxPerMonth"
        label="每月上限"
        placeholder="不限制"
        min={1}
        fieldProps={{ precision: 0 }}
      />

      <ProFormDigit
        name="maxPerYear"
        label="每年上限"
        placeholder="不限制"
        min={1}
        fieldProps={{ precision: 0 }}
      />

      <Divider>时间与选项</Divider>

      <ProFormDateTimePicker
        name="startTime"
        label="开始时间"
        placeholder="请选择开始时间"
      />

      <ProFormDateTimePicker
        name="endTime"
        label="结束时间"
        placeholder="请选择结束时间"
      />

      <ProFormSwitch
        name="autoRedeem"
        label="自动兑换"
        tooltip="启用后满足条件自动触发兑换"
      />
    </ModalForm>
  );
};

export default RedemptionRuleForm;

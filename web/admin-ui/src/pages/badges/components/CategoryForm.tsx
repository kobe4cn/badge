/**
 * 分类表单弹窗组件
 *
 * 支持新建和编辑两种模式，使用 ModalForm 实现表单弹窗交互
 */

import React from 'react';
import {
  ModalForm,
  ProFormText,
  ProFormDigit,
} from '@ant-design/pro-components';
import type { BadgeCategory, CreateCategoryRequest } from '@/types';

/**
 * 表单数据类型
 *
 * 创建和编辑共用，编辑时会预填充初始值
 */
interface CategoryFormData {
  name: string;
  iconUrl?: string;
  sortOrder?: number;
}

interface CategoryFormProps {
  /** 弹窗是否可见 */
  open: boolean;
  /** 关闭弹窗回调 */
  onOpenChange: (open: boolean) => void;
  /** 编辑时传入现有分类数据，新建时为 undefined */
  initialValues?: BadgeCategory;
  /** 表单提交回调 */
  onSubmit: (values: CreateCategoryRequest) => Promise<boolean>;
  /** 提交中状态 */
  loading?: boolean;
}

/**
 * 分类表单弹窗
 *
 * - 新建模式：initialValues 为空
 * - 编辑模式：initialValues 包含现有数据
 */
const CategoryForm: React.FC<CategoryFormProps> = ({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  loading,
}) => {
  const isEdit = !!initialValues;

  return (
    <ModalForm<CategoryFormData>
      title={isEdit ? '编辑分类' : '新建分类'}
      open={open}
      onOpenChange={onOpenChange}
      initialValues={
        initialValues
          ? {
              name: initialValues.name,
              iconUrl: initialValues.iconUrl,
              sortOrder: initialValues.sortOrder,
            }
          : {
              sortOrder: 0,
            }
      }
      modalProps={{
        destroyOnClose: true,
        maskClosable: false,
      }}
      submitTimeout={3000}
      loading={loading}
      onFinish={async (values) => {
        const result = await onSubmit({
          name: values.name,
          iconUrl: values.iconUrl || undefined,
          sortOrder: values.sortOrder,
        });
        return result;
      }}
    >
      <ProFormText
        name="name"
        label="分类名称"
        placeholder="请输入分类名称"
        rules={[
          { required: true, message: '请输入分类名称' },
          { max: 50, message: '分类名称不能超过50个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 50,
        }}
      />

      <ProFormText
        name="iconUrl"
        label="图标 URL"
        placeholder="请输入分类图标 URL（可选）"
        tooltip="分类图标的 CDN 地址，用于前端展示"
        rules={[
          {
            type: 'url',
            message: '请输入有效的 URL 地址',
          },
        ]}
      />

      <ProFormDigit
        name="sortOrder"
        label="排序值"
        tooltip="数值越小越靠前，默认为 0"
        placeholder="请输入排序值"
        min={0}
        max={9999}
        fieldProps={{
          precision: 0,
        }}
      />
    </ModalForm>
  );
};

export default CategoryForm;

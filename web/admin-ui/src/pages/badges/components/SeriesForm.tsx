/**
 * 系列表单弹窗组件
 *
 * 支持新建和编辑两种模式，使用 ModalForm 实现表单弹窗交互
 */

import React, { useEffect, useState } from 'react';
import {
  ModalForm,
  ProFormText,
  ProFormTextArea,
  ProFormDigit,
  ProFormSelect,
  ProFormSwitch,
  ProFormUploadButton,
} from '@ant-design/pro-components';
import { App } from 'antd';
import type { UploadFile } from 'antd';
import { useAllCategories } from '@/hooks/useCategory';
import type { BadgeSeries, CreateSeriesRequest } from '@/types';

/**
 * 表单数据类型
 *
 * 创建和编辑共用，编辑时会预填充初始值
 */
interface SeriesFormData {
  name: string;
  categoryId: number;
  description?: string;
  coverUrl?: string;
  sortOrder?: number;
  status: boolean; // 表单中用 boolean 表示状态
  startTime?: string;
  endTime?: string;
}

interface SeriesFormProps {
  /** 弹窗是否可见 */
  open: boolean;
  /** 关闭弹窗回调 */
  onOpenChange: (open: boolean) => void;
  /** 编辑时传入现有系列数据，新建时为 undefined */
  initialValues?: BadgeSeries;
  /** 表单提交回调 */
  onSubmit: (values: CreateSeriesRequest & { status?: 'ACTIVE' | 'INACTIVE' }) => Promise<boolean>;
  /** 提交中状态 */
  loading?: boolean;
}

/**
 * 系列表单弹窗
 *
 * - 新建模式：initialValues 为空
 * - 编辑模式：initialValues 包含现有数据
 */
const SeriesForm: React.FC<SeriesFormProps> = ({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  loading,
}) => {
  const isEdit = !!initialValues;
  const { message } = App.useApp();

  // 分类下拉选项
  const { data: categories, isLoading: categoriesLoading } = useAllCategories();

  // 上传文件列表状态
  const [fileList, setFileList] = useState<UploadFile[]>([]);

  // 编辑模式下初始化封面图
  useEffect(() => {
    if (open && initialValues?.coverUrl) {
      setFileList([
        {
          uid: '-1',
          name: 'cover',
          status: 'done',
          url: initialValues.coverUrl,
        },
      ]);
    } else if (!open) {
      setFileList([]);
    }
  }, [open, initialValues?.coverUrl]);

  // 将分类数据转换为下拉选项格式
  const categoryOptions =
    categories?.map((cat) => ({
      label: cat.name,
      value: cat.id,
    })) || [];

  return (
    <ModalForm<SeriesFormData>
      title={isEdit ? '编辑系列' : '新建系列'}
      open={open}
      onOpenChange={onOpenChange}
      initialValues={
        initialValues
          ? {
              name: initialValues.name,
              categoryId: initialValues.categoryId,
              description: initialValues.description,
              coverUrl: initialValues.coverUrl,
              sortOrder: initialValues.sortOrder,
              status: initialValues.status === 'ACTIVE',
            }
          : {
              sortOrder: 0,
              status: true,
            }
      }
      modalProps={{
        destroyOnClose: true,
        maskClosable: false,
      }}
      submitTimeout={3000}
      loading={loading}
      onFinish={async (values) => {
        // 如果有上传的文件，使用上传后的 URL
        let coverUrl = values.coverUrl;
        if (fileList.length > 0 && fileList[0].response?.url) {
          coverUrl = fileList[0].response.url;
        } else if (fileList.length > 0 && fileList[0].url) {
          coverUrl = fileList[0].url;
        }

        const result = await onSubmit({
          categoryId: values.categoryId,
          name: values.name,
          description: values.description || undefined,
          coverUrl: coverUrl || undefined,
          sortOrder: values.sortOrder,
          status: values.status ? 'ACTIVE' : 'INACTIVE',
        });
        return result;
      }}
    >
      <ProFormText
        name="name"
        label="系列名称"
        placeholder="请输入系列名称"
        rules={[
          { required: true, message: '请输入系列名称' },
          { max: 100, message: '系列名称不能超过100个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 100,
        }}
      />

      <ProFormSelect
        name="categoryId"
        label="所属分类"
        placeholder="请选择所属分类"
        rules={[{ required: true, message: '请选择所属分类' }]}
        options={categoryOptions}
        fieldProps={{
          loading: categoriesLoading,
          showSearch: true,
          filterOption: (input, option) =>
            (option?.label ?? '').toLowerCase().includes(input.toLowerCase()),
        }}
      />

      <ProFormUploadButton
        name="cover"
        label="封面图"
        title="上传封面"
        max={1}
        fieldProps={{
          name: 'file',
          listType: 'picture-card',
          fileList: fileList,
          beforeUpload: (file) => {
            // 校验文件类型
            const isImage = file.type.startsWith('image/');
            if (!isImage) {
              message.error('只能上传图片文件');
              return false;
            }
            // 校验文件大小（不超过 2MB）
            const isLt2M = file.size / 1024 / 1024 < 2;
            if (!isLt2M) {
              message.error('图片大小不能超过 2MB');
              return false;
            }
            return true;
          },
          onChange: ({ fileList: newFileList }) => {
            setFileList(newFileList);
          },
          action: '/api/v1/upload/image',
        }}
        extra="支持 jpg、png 格式，文件大小不超过 2MB"
      />

      <ProFormText
        name="coverUrl"
        label="封面图 URL"
        placeholder="或直接输入封面图 URL"
        tooltip="如果同时上传文件和填写 URL，优先使用上传的文件"
        rules={[
          {
            type: 'url',
            message: '请输入有效的 URL 地址',
          },
        ]}
      />

      <ProFormTextArea
        name="description"
        label="系列描述"
        placeholder="请输入系列描述（可选）"
        fieldProps={{
          showCount: true,
          maxLength: 500,
          autoSize: { minRows: 2, maxRows: 4 },
        }}
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

      <ProFormSwitch
        name="status"
        label="状态"
        checkedChildren="启用"
        unCheckedChildren="禁用"
        tooltip="禁用后系列将不在前端展示"
      />
    </ModalForm>
  );
};

export default SeriesForm;

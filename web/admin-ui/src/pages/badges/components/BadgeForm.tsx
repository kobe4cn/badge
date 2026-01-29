/**
 * 徽章表单弹窗组件
 *
 * 支持新建和编辑两种模式，包含基本信息、素材、有效期和库存配置
 */

import React, { useEffect, useState } from 'react';
import {
  DrawerForm,
  ProFormText,
  ProFormTextArea,
  ProFormDigit,
  ProFormSelect,
  ProFormSwitch,
  ProFormDependency,
  ProFormDatePicker,
  ProFormUploadButton,
  ProFormGroup,
} from '@ant-design/pro-components';
import { App, Divider } from 'antd';
import type { UploadFile } from 'antd';
import { useAllCategories } from '@/hooks/useCategory';
import { useAllSeries } from '@/hooks/useSeries';
import type {
  Badge,
  CreateBadgeRequest,
  BadgeType,
  ValidityType,
  BadgeAssets,
  ValidityConfig,
} from '@/types';

/**
 * 表单数据类型
 *
 * 扁平化结构便于表单操作
 */
interface BadgeFormData {
  name: string;
  categoryId?: number;
  seriesId: number;
  badgeType: BadgeType;
  description?: string;
  obtainDescription?: string;
  sortOrder?: number;
  // 素材信息
  iconUrl: string;
  imageUrl?: string;
  animationUrl?: string;
  disabledIconUrl?: string;
  // 有效期配置
  validityType: ValidityType;
  relativeDays?: number;
  fixedDate?: string;
  // 库存配置
  isLimited: boolean;
  maxSupply?: number;
}

interface BadgeFormProps {
  /** 弹窗是否可见 */
  open: boolean;
  /** 关闭弹窗回调 */
  onOpenChange: (open: boolean) => void;
  /** 编辑时传入现有徽章数据，新建时为 undefined */
  initialValues?: Badge & { categoryId?: number };
  /** 表单提交回调 */
  onSubmit: (values: CreateBadgeRequest) => Promise<boolean>;
  /** 提交中状态 */
  loading?: boolean;
}

/**
 * 徽章类型选项
 */
const BADGE_TYPE_OPTIONS = [
  { label: '普通徽章', value: 'NORMAL' },
  { label: '限定徽章', value: 'LIMITED' },
  { label: '成就徽章', value: 'ACHIEVEMENT' },
  { label: '活动徽章', value: 'EVENT' },
];

/**
 * 有效期类型选项
 */
const VALIDITY_TYPE_OPTIONS = [
  { label: '永久有效', value: 'PERMANENT' },
  { label: '固定天数', value: 'RELATIVE_DAYS' },
  { label: '固定日期', value: 'FIXED_DATE' },
];

/**
 * 徽章表单弹窗
 *
 * 使用 DrawerForm 提供更多空间展示表单内容
 */
const BadgeForm: React.FC<BadgeFormProps> = ({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  loading,
}) => {
  const isEdit = !!initialValues;
  const { message } = App.useApp();

  // 分类和系列选择状态（用于联动）
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | undefined>(
    initialValues?.categoryId
  );

  // 上传文件列表状态
  const [iconFileList, setIconFileList] = useState<UploadFile[]>([]);
  const [imageFileList, setImageFileList] = useState<UploadFile[]>([]);
  const [animationFileList, setAnimationFileList] = useState<UploadFile[]>([]);

  // 分类下拉选项
  const { data: categories, isLoading: categoriesLoading } = useAllCategories();
  // 系列下拉选项（根据分类联动）
  const { data: seriesList, isLoading: seriesLoading } = useAllSeries(selectedCategoryId);

  // 编辑模式下初始化图片
  useEffect(() => {
    if (open && initialValues) {
      setSelectedCategoryId(initialValues.categoryId);

      if (initialValues.assets.iconUrl) {
        setIconFileList([
          { uid: '-1', name: 'icon', status: 'done', url: initialValues.assets.iconUrl },
        ]);
      }
      if (initialValues.assets.imageUrl) {
        setImageFileList([
          { uid: '-2', name: 'image', status: 'done', url: initialValues.assets.imageUrl },
        ]);
      }
      if (initialValues.assets.animationUrl) {
        setAnimationFileList([
          { uid: '-3', name: 'animation', status: 'done', url: initialValues.assets.animationUrl },
        ]);
      }
    } else if (!open) {
      // 关闭时重置
      setIconFileList([]);
      setImageFileList([]);
      setAnimationFileList([]);
      setSelectedCategoryId(undefined);
    }
  }, [open, initialValues]);

  // 分类选项
  const categoryOptions =
    categories?.map((cat) => ({
      label: cat.name,
      value: cat.id,
    })) || [];

  // 系列选项
  const seriesOptions =
    seriesList?.map((series) => ({
      label: series.name,
      value: series.id,
    })) || [];

  /**
   * 获取上传后的 URL
   */
  const getUploadedUrl = (
    fileList: UploadFile[],
    fallbackUrl?: string
  ): string | undefined => {
    if (fileList.length > 0) {
      if (fileList[0].response?.url) {
        return fileList[0].response.url;
      }
      if (fileList[0].url) {
        return fileList[0].url;
      }
    }
    return fallbackUrl;
  };

  /**
   * 构建表单初始值
   */
  const getInitialValues = (): Partial<BadgeFormData> => {
    if (initialValues) {
      return {
        name: initialValues.name,
        categoryId: initialValues.categoryId,
        seriesId: initialValues.seriesId,
        badgeType: initialValues.badgeType,
        description: initialValues.description,
        obtainDescription: initialValues.obtainDescription,
        sortOrder: initialValues.sortOrder,
        iconUrl: initialValues.assets.iconUrl,
        imageUrl: initialValues.assets.imageUrl,
        animationUrl: initialValues.assets.animationUrl,
        disabledIconUrl: initialValues.assets.disabledIconUrl,
        validityType: initialValues.validityConfig.validityType,
        relativeDays: initialValues.validityConfig.relativeDays,
        fixedDate: initialValues.validityConfig.fixedDate,
        isLimited: !!initialValues.maxSupply,
        maxSupply: initialValues.maxSupply,
      };
    }
    return {
      sortOrder: 0,
      validityType: 'PERMANENT',
      isLimited: false,
      badgeType: 'NORMAL',
    };
  };

  /**
   * 文件上传前校验
   */
  const beforeUpload = (file: File, isAnimation = false) => {
    if (isAnimation) {
      // 动画文件支持 JSON (Lottie) 或视频
      const isValid =
        file.type === 'application/json' ||
        file.type.startsWith('video/') ||
        file.name.endsWith('.json');
      if (!isValid) {
        message.error('动画文件仅支持 Lottie JSON 或视频格式');
        return false;
      }
    } else {
      // 图片文件
      const isImage = file.type.startsWith('image/');
      if (!isImage) {
        message.error('只能上传图片文件');
        return false;
      }
    }
    // 文件大小限制 5MB
    const isLt5M = file.size / 1024 / 1024 < 5;
    if (!isLt5M) {
      message.error('文件大小不能超过 5MB');
      return false;
    }
    return true;
  };

  return (
    <DrawerForm<BadgeFormData>
      title={isEdit ? '编辑徽章' : '新建徽章'}
      open={open}
      onOpenChange={onOpenChange}
      width={600}
      initialValues={getInitialValues()}
      drawerProps={{
        destroyOnClose: true,
        maskClosable: false,
      }}
      submitTimeout={3000}
      loading={loading}
      onFinish={async (values) => {
        // 构建资源配置
        const assets: BadgeAssets = {
          iconUrl: getUploadedUrl(iconFileList, values.iconUrl) || values.iconUrl,
          imageUrl: getUploadedUrl(imageFileList, values.imageUrl),
          animationUrl: getUploadedUrl(animationFileList, values.animationUrl),
          disabledIconUrl: values.disabledIconUrl,
        };

        // 构建有效期配置
        const validityConfig: ValidityConfig = {
          validityType: values.validityType,
        };
        if (values.validityType === 'RELATIVE_DAYS') {
          validityConfig.relativeDays = values.relativeDays;
        } else if (values.validityType === 'FIXED_DATE') {
          validityConfig.fixedDate = values.fixedDate;
        }

        const result = await onSubmit({
          seriesId: values.seriesId,
          badgeType: values.badgeType,
          name: values.name,
          description: values.description || undefined,
          obtainDescription: values.obtainDescription || undefined,
          sortOrder: values.sortOrder,
          assets,
          validityConfig,
          maxSupply: values.isLimited ? values.maxSupply : undefined,
        });
        return result;
      }}
    >
      {/* 基本信息 */}
      <Divider orientation="left" plain>
        基本信息
      </Divider>

      <ProFormText
        name="name"
        label="徽章名称"
        placeholder="请输入徽章名称"
        rules={[
          { required: true, message: '请输入徽章名称' },
          { max: 100, message: '徽章名称不能超过100个字符' },
        ]}
        fieldProps={{
          showCount: true,
          maxLength: 100,
        }}
      />

      <ProFormSelect
        name="badgeType"
        label="徽章类型"
        placeholder="请选择徽章类型"
        rules={[{ required: true, message: '请选择徽章类型' }]}
        options={BADGE_TYPE_OPTIONS}
      />

      <ProFormGroup>
        <ProFormSelect
          name="categoryId"
          label="所属分类"
          placeholder="请选择分类（可选）"
          tooltip="选择分类后可筛选系列"
          options={categoryOptions}
          fieldProps={{
            loading: categoriesLoading,
            showSearch: true,
            allowClear: true,
            filterOption: (input, option) =>
              (option?.label ?? '').toLowerCase().includes(input.toLowerCase()),
            onChange: (value) => {
              setSelectedCategoryId(value as number | undefined);
            },
          }}
          width="sm"
        />

        <ProFormSelect
          name="seriesId"
          label="所属系列"
          placeholder="请选择所属系列"
          rules={[{ required: true, message: '请选择所属系列' }]}
          options={seriesOptions}
          fieldProps={{
            loading: seriesLoading,
            showSearch: true,
            filterOption: (input, option) =>
              (option?.label ?? '').toLowerCase().includes(input.toLowerCase()),
          }}
          width="sm"
        />
      </ProFormGroup>

      <ProFormTextArea
        name="description"
        label="徽章描述"
        placeholder="请输入徽章描述（可选）"
        fieldProps={{
          showCount: true,
          maxLength: 500,
          autoSize: { minRows: 2, maxRows: 4 },
        }}
      />

      <ProFormTextArea
        name="obtainDescription"
        label="获取条件描述"
        placeholder="请输入获取条件说明，展示给用户（可选）"
        tooltip="向用户展示如何获得此徽章"
        fieldProps={{
          showCount: true,
          maxLength: 200,
          autoSize: { minRows: 2, maxRows: 3 },
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
        width="sm"
      />

      {/* 素材信息 */}
      <Divider orientation="left" plain>
        素材信息
      </Divider>

      <ProFormUploadButton
        name="iconUpload"
        label="徽章图标"
        title="上传图标"
        max={1}
        extra="徽章主图标，支持 jpg、png 格式，建议尺寸 200x200"
        fieldProps={{
          name: 'file',
          listType: 'picture-card',
          fileList: iconFileList,
          beforeUpload: (file) => beforeUpload(file),
          onChange: ({ fileList }) => setIconFileList(fileList),
          action: '/api/v1/upload/image',
        }}
      />

      <ProFormText
        name="iconUrl"
        label="图标 URL"
        placeholder="或直接输入图标 URL"
        tooltip="如果同时上传文件和填写 URL，优先使用上传的文件"
        rules={[
          { required: !iconFileList.length, message: '请上传图标或填写图标 URL' },
          { type: 'url', message: '请输入有效的 URL 地址' },
        ]}
      />

      <ProFormUploadButton
        name="imageUpload"
        label="详情图"
        title="上传详情图"
        max={1}
        extra="徽章详情大图（可选），支持 jpg、png 格式"
        fieldProps={{
          name: 'file',
          listType: 'picture-card',
          fileList: imageFileList,
          beforeUpload: (file) => beforeUpload(file),
          onChange: ({ fileList }) => setImageFileList(fileList),
          action: '/api/v1/upload/image',
        }}
      />

      <ProFormText
        name="imageUrl"
        label="详情图 URL"
        placeholder="或直接输入详情图 URL（可选）"
        rules={[{ type: 'url', message: '请输入有效的 URL 地址' }]}
      />

      <ProFormUploadButton
        name="animationUpload"
        label="动效资源"
        title="上传动效"
        max={1}
        extra="Lottie JSON 或视频文件（可选）"
        fieldProps={{
          name: 'file',
          listType: 'picture-card',
          fileList: animationFileList,
          beforeUpload: (file) => beforeUpload(file, true),
          onChange: ({ fileList }) => setAnimationFileList(fileList),
          action: '/api/v1/upload/file',
        }}
      />

      <ProFormText
        name="animationUrl"
        label="动效 URL"
        placeholder="或直接输入动效资源 URL（可选）"
        rules={[{ type: 'url', message: '请输入有效的 URL 地址' }]}
      />

      <ProFormText
        name="disabledIconUrl"
        label="灰态图标 URL"
        placeholder="未获取时展示的灰态图标（可选）"
        tooltip="用户未获取徽章时的展示图标"
        rules={[{ type: 'url', message: '请输入有效的 URL 地址' }]}
      />

      {/* 有效期配置 */}
      <Divider orientation="left" plain>
        有效期配置
      </Divider>

      <ProFormSelect
        name="validityType"
        label="有效期类型"
        placeholder="请选择有效期类型"
        rules={[{ required: true, message: '请选择有效期类型' }]}
        options={VALIDITY_TYPE_OPTIONS}
      />

      <ProFormDependency name={['validityType']}>
        {({ validityType }) => {
          if (validityType === 'RELATIVE_DAYS') {
            return (
              <ProFormDigit
                name="relativeDays"
                label="有效天数"
                placeholder="请输入有效天数"
                rules={[{ required: true, message: '请输入有效天数' }]}
                min={1}
                max={36500}
                fieldProps={{
                  precision: 0,
                  addonAfter: '天',
                }}
                width="sm"
              />
            );
          }
          if (validityType === 'FIXED_DATE') {
            return (
              <ProFormDatePicker
                name="fixedDate"
                label="过期日期"
                placeholder="请选择过期日期"
                rules={[{ required: true, message: '请选择过期日期' }]}
                width="sm"
              />
            );
          }
          return null;
        }}
      </ProFormDependency>

      {/* 库存配置 */}
      <Divider orientation="left" plain>
        库存配置
      </Divider>

      <ProFormSwitch
        name="isLimited"
        label="是否限量"
        checkedChildren="限量"
        unCheckedChildren="不限量"
        tooltip="开启后需设置库存总量"
      />

      <ProFormDependency name={['isLimited']}>
        {({ isLimited }) => {
          if (isLimited) {
            return (
              <ProFormDigit
                name="maxSupply"
                label="库存总量"
                placeholder="请输入库存总量"
                rules={[{ required: true, message: '请输入库存总量' }]}
                min={1}
                fieldProps={{
                  precision: 0,
                }}
                width="sm"
              />
            );
          }
          return null;
        }}
      </ProFormDependency>
    </DrawerForm>
  );
};

export default BadgeForm;

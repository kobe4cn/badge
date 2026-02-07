/**
 * 素材库页面
 *
 * 管理徽章系统使用的各类媒体资源，支持：
 * - 图片、动画、视频、3D 模型的上传和管理
 * - 分类和标签筛选
 * - 网格/列表视图切换
 */

import React, { useState, useCallback, useRef } from 'react';
import {
  Button,
  Card,
  Space,
  Tag,
  Tooltip,
  Image,
  Modal,
  Form,
  Input,
  Select,
  message,
  Popconfirm,
  Row,
  Col,
  Segmented,
  Empty,
  Spin,
} from 'antd';
import {
  PlusOutlined,
  ReloadOutlined,
  DeleteOutlined,
  EditOutlined,
  EyeOutlined,
  AppstoreOutlined,
  UnorderedListOutlined,
  FileImageOutlined,
  VideoCameraOutlined,
  GifOutlined,
  FileUnknownOutlined,
} from '@ant-design/icons';
import { PageContainer, ProTable, type ActionType } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getAssets,
  createAsset,
  updateAsset,
  deleteAsset,
  getAssetCategories,
  ASSET_TYPES,
  formatFileSize,
  type Asset,
  type AssetType,
  type AssetQueryParams,
  type CreateAssetRequest,
} from '@/services/asset';
import { formatDateTime } from '@/utils/format';

type ViewMode = 'grid' | 'list';

/**
 * 获取素材类型图标
 */
const getAssetTypeIcon = (type: AssetType) => {
  switch (type) {
    case 'IMAGE':
      return <FileImageOutlined />;
    case 'ANIMATION':
      return <GifOutlined />;
    case 'VIDEO':
      return <VideoCameraOutlined />;
    case 'MODEL_3D':
      return <FileUnknownOutlined />;
    default:
      return <FileImageOutlined />;
  }
};

/**
 * 获取素材类型标签颜色
 */
const getAssetTypeColor = (type: AssetType) => {
  switch (type) {
    case 'IMAGE':
      return 'blue';
    case 'ANIMATION':
      return 'purple';
    case 'VIDEO':
      return 'orange';
    case 'MODEL_3D':
      return 'cyan';
    default:
      return 'default';
  }
};

/**
 * 素材库页面组件
 */
const AssetLibraryPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const queryClient = useQueryClient();
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [editingAsset, setEditingAsset] = useState<Asset | null>(null);
  const [previewAsset, setPreviewAsset] = useState<Asset | null>(null);
  const [form] = Form.useForm();

  // 获取分类列表
  const { data: categories = [] } = useQuery({
    queryKey: ['assetCategories'],
    queryFn: getAssetCategories,
  });

  // 删除素材
  const deleteMutation = useMutation({
    mutationFn: deleteAsset,
    onSuccess: () => {
      message.success('删除成功');
      queryClient.invalidateQueries({ queryKey: ['assets'] });
      actionRef.current?.reload();
    },
    onError: () => {
      message.error('删除失败');
    },
  });

  // 创建/更新素材
  const saveMutation = useMutation({
    mutationFn: async (values: CreateAssetRequest & { id?: number }) => {
      if (values.id) {
        return updateAsset(values.id, values);
      }
      return createAsset(values);
    },
    onSuccess: () => {
      message.success(editingAsset ? '更新成功' : '创建成功');
      queryClient.invalidateQueries({ queryKey: ['assets'] });
      actionRef.current?.reload();
      handleCloseModal();
    },
    onError: () => {
      message.error(editingAsset ? '更新失败' : '创建失败');
    },
  });

  /**
   * 打开编辑弹窗
   */
  const handleEdit = useCallback((asset: Asset) => {
    setEditingAsset(asset);
    form.setFieldsValue({
      name: asset.name,
      assetType: asset.assetType,
      fileUrl: asset.fileUrl,
      thumbnailUrl: asset.thumbnailUrl,
      category: asset.category,
      tags: asset.tags,
    });
    setCreateModalOpen(true);
  }, [form]);

  /**
   * 关闭弹窗
   */
  const handleCloseModal = useCallback(() => {
    setCreateModalOpen(false);
    setEditingAsset(null);
    form.resetFields();
  }, [form]);

  /**
   * 提交表单
   */
  const handleSubmit = useCallback(async () => {
    try {
      const values = await form.validateFields();
      await saveMutation.mutateAsync({
        ...values,
        id: editingAsset?.id,
      });
    } catch {
      // 表单验证失败
    }
  }, [form, editingAsset, saveMutation]);

  /**
   * 表格列定义
   */
  const columns: ProColumns<Asset>[] = [
    {
      title: '预览',
      dataIndex: 'thumbnailUrl',
      width: 80,
      search: false,
      render: (_, record) => (
        <Image
          width={48}
          height={48}
          src={record.thumbnailUrl || record.fileUrl}
          fallback="/placeholder.png"
          style={{ objectFit: 'cover', borderRadius: 4 }}
          preview={false}
          onClick={() => setPreviewAsset(record)}
        />
      ),
    },
    {
      title: '名称',
      dataIndex: 'name',
      ellipsis: true,
    },
    {
      title: '类型',
      dataIndex: 'assetType',
      width: 100,
      valueType: 'select',
      valueEnum: Object.fromEntries(
        ASSET_TYPES.map((t) => [t.value, { text: t.label }])
      ),
      render: (_, record) => (
        <Tag color={getAssetTypeColor(record.assetType)} icon={getAssetTypeIcon(record.assetType)}>
          {ASSET_TYPES.find((t) => t.value === record.assetType)?.label || record.assetType}
        </Tag>
      ),
    },
    {
      title: '分类',
      dataIndex: 'category',
      valueType: 'select',
      valueEnum: Object.fromEntries(categories.map((c) => [c, { text: c }])),
      render: (_, record) => record.category || '-',
    },
    {
      title: '文件大小',
      dataIndex: 'fileSize',
      width: 100,
      search: false,
      render: (_, record) => formatFileSize(record.fileSize),
    },
    {
      title: '使用次数',
      dataIndex: 'usageCount',
      width: 80,
      search: false,
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 160,
      search: false,
      render: (_, record) => formatDateTime(record.createdAt),
    },
    {
      title: '操作',
      key: 'action',
      width: 120,
      search: false,
      render: (_, record) => (
        <Space size={0}>
          <Tooltip title="预览">
            <Button
              type="link"
              size="small"
              icon={<EyeOutlined />}
              onClick={() => setPreviewAsset(record)}
            />
          </Tooltip>
          <Tooltip title="编辑">
            <Button
              type="link"
              size="small"
              icon={<EditOutlined />}
              onClick={() => handleEdit(record)}
            />
          </Tooltip>
          <Popconfirm
            title="确认删除"
            description="删除后无法恢复，确定要删除此素材吗？"
            onConfirm={() => deleteMutation.mutate(record.id)}
            okText="确认"
            cancelText="取消"
          >
            <Tooltip title="删除">
              <Button
                type="link"
                size="small"
                danger
                icon={<DeleteOutlined />}
              />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  /**
   * 渲染网格视图
   */
  const renderGridView = (assets: Asset[], loading: boolean) => {
    if (loading) {
      return (
        <div style={{ textAlign: 'center', padding: 48 }}>
          <Spin tip="加载中..." />
        </div>
      );
    }

    if (assets.length === 0) {
      return <Empty description="暂无素材" />;
    }

    return (
      <Row gutter={[16, 16]}>
        {assets.map((asset) => (
          <Col key={asset.id} xs={12} sm={8} md={6} lg={4} xl={3}>
            <Card
              hoverable
              cover={
                <div
                  style={{
                    height: 120,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    background: '#f5f5f5',
                    overflow: 'hidden',
                  }}
                >
                  {asset.assetType === 'MODEL_3D' ? (
                    <FileUnknownOutlined style={{ fontSize: 48, color: '#8c8c8c' }} />
                  ) : (
                    <Image
                      src={asset.thumbnailUrl || asset.fileUrl}
                      alt={asset.name}
                      style={{ maxHeight: 120, objectFit: 'contain' }}
                      preview={false}
                    />
                  )}
                </div>
              }
              bodyStyle={{ padding: 12 }}
              actions={[
                <EyeOutlined key="view" onClick={() => setPreviewAsset(asset)} />,
                <EditOutlined key="edit" onClick={() => handleEdit(asset)} />,
                <Popconfirm
                  key="delete"
                  title="确认删除"
                  description="删除后无法恢复"
                  onConfirm={() => deleteMutation.mutate(asset.id)}
                >
                  <DeleteOutlined />
                </Popconfirm>,
              ]}
            >
              <Card.Meta
                title={
                  <Tooltip title={asset.name}>
                    <span style={{ fontSize: 12 }}>{asset.name}</span>
                  </Tooltip>
                }
                description={
                  <Space size={4}>
                    <Tag color={getAssetTypeColor(asset.assetType)} style={{ fontSize: 10 }}>
                      {ASSET_TYPES.find((t) => t.value === asset.assetType)?.label}
                    </Tag>
                    <span style={{ fontSize: 10, color: '#8c8c8c' }}>
                      {formatFileSize(asset.fileSize)}
                    </span>
                  </Space>
                }
              />
            </Card>
          </Col>
        ))}
      </Row>
    );
  };

  return (
    <PageContainer
      title="素材库"
      extra={
        <Space>
          <Segmented
            value={viewMode}
            onChange={(v) => setViewMode(v as ViewMode)}
            options={[
              { value: 'grid', icon: <AppstoreOutlined /> },
              { value: 'list', icon: <UnorderedListOutlined /> },
            ]}
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => setCreateModalOpen(true)}
          >
            上传素材
          </Button>
        </Space>
      }
    >
      {viewMode === 'list' ? (
        <ProTable<Asset>
          actionRef={actionRef}
          columns={columns}
          rowKey="id"
          request={async (params) => {
            const { current, pageSize, ...rest } = params;
            const result = await getAssets({
              page: current,
              pageSize,
              ...rest,
            } as AssetQueryParams);
            return {
              data: result.items,
              total: result.total,
              success: true,
            };
          }}
          pagination={{ defaultPageSize: 20 }}
          search={{ labelWidth: 'auto' }}
          toolBarRender={() => [
            <Button
              key="refresh"
              icon={<ReloadOutlined />}
              onClick={() => actionRef.current?.reload()}
            >
              刷新
            </Button>,
          ]}
        />
      ) : (
        <Card>
          <ProTable<Asset>
            actionRef={actionRef}
            columns={columns}
            rowKey="id"
            request={async (params) => {
              const { current, pageSize, ...rest } = params;
              const result = await getAssets({
                page: current,
                pageSize: 24,
                ...rest,
              } as AssetQueryParams);
              return {
                data: result.items,
                total: result.total,
                success: true,
              };
            }}
            search={{ labelWidth: 'auto' }}
            tableRender={(props, _defaultDom, domList) => {
              // 获取当前数据源
              const assets = (props.dataSource || []) as Asset[];
              const isLoading = props.loading as boolean;
              return (
                <div>
                  {domList.toolbar}
                  {renderGridView(assets, isLoading)}
                </div>
              );
            }}
            pagination={{ defaultPageSize: 24 }}
          />
        </Card>
      )}

      {/* 创建/编辑弹窗 */}
      <Modal
        title={editingAsset ? '编辑素材' : '上传素材'}
        open={createModalOpen}
        onCancel={handleCloseModal}
        onOk={handleSubmit}
        confirmLoading={saveMutation.isPending}
        width={560}
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{ assetType: 'IMAGE' }}
        >
          <Form.Item
            name="name"
            label="素材名称"
            rules={[{ required: true, message: '请输入素材名称' }]}
          >
            <Input placeholder="请输入素材名称" maxLength={100} />
          </Form.Item>

          <Form.Item
            name="assetType"
            label="素材类型"
            rules={[{ required: true, message: '请选择素材类型' }]}
          >
            <Select
              options={ASSET_TYPES.map((t) => ({ value: t.value, label: t.label }))}
              disabled={!!editingAsset}
            />
          </Form.Item>

          <Form.Item
            name="fileUrl"
            label="文件地址"
            rules={[
              { required: true, message: '请输入文件地址' },
              { type: 'url', message: '请输入有效的 URL' },
            ]}
          >
            <Input placeholder="请输入文件 URL（OSS 或 CDN 地址）" />
          </Form.Item>

          <Form.Item name="thumbnailUrl" label="缩略图地址">
            <Input placeholder="请输入缩略图 URL（可选）" />
          </Form.Item>

          <Form.Item name="category" label="分类">
            <Select
              placeholder="选择或输入分类"
              allowClear
              showSearch
              options={categories.map((c) => ({ value: c, label: c }))}
            />
          </Form.Item>

          <Form.Item name="tags" label="标签">
            <Select
              mode="tags"
              placeholder="输入标签后按回车添加"
              allowClear
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* 预览弹窗 */}
      <Modal
        title={previewAsset?.name}
        open={!!previewAsset}
        onCancel={() => setPreviewAsset(null)}
        footer={null}
        width={800}
      >
        {previewAsset && (
          <div style={{ textAlign: 'center' }}>
            {previewAsset.assetType === 'VIDEO' ? (
              <video
                src={previewAsset.fileUrl}
                controls
                style={{ maxWidth: '100%', maxHeight: 500 }}
              />
            ) : previewAsset.assetType === 'MODEL_3D' ? (
              <div style={{ padding: 48, background: '#f5f5f5', borderRadius: 8 }}>
                <FileUnknownOutlined style={{ fontSize: 64, color: '#8c8c8c' }} />
                <div style={{ marginTop: 16 }}>
                  <a href={previewAsset.fileUrl} target="_blank" rel="noopener noreferrer">
                    下载 3D 模型文件
                  </a>
                </div>
              </div>
            ) : (
              <Image
                src={previewAsset.fileUrl}
                alt={previewAsset.name}
                style={{ maxWidth: '100%', maxHeight: 500 }}
              />
            )}
            <div style={{ marginTop: 16, textAlign: 'left' }}>
              <Space direction="vertical" size={4}>
                <span>
                  <strong>类型：</strong>
                  {ASSET_TYPES.find((t) => t.value === previewAsset.assetType)?.label}
                </span>
                <span>
                  <strong>大小：</strong>
                  {formatFileSize(previewAsset.fileSize)}
                </span>
                {previewAsset.width && previewAsset.height && (
                  <span>
                    <strong>尺寸：</strong>
                    {previewAsset.width} x {previewAsset.height}
                  </span>
                )}
                <span>
                  <strong>使用次数：</strong>
                  {previewAsset.usageCount}
                </span>
              </Space>
            </div>
          </div>
        )}
      </Modal>
    </PageContainer>
  );
};

export default AssetLibraryPage;

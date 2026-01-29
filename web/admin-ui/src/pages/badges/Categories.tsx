/**
 * 徽章分类管理页面
 *
 * 提供分类的 CRUD 操作，包含列表展示、状态切换和排序管理
 */

import React, { useRef, useState } from 'react';
import { Button, Switch, Popconfirm, Space, InputNumber } from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  ReloadOutlined,
  EditOutlined,
  DeleteOutlined,
} from '@ant-design/icons';
import {
  useCategoryList,
  useCreateCategory,
  useUpdateCategory,
  useDeleteCategory,
  useToggleCategoryStatus,
  useUpdateCategorySortOrder,
} from '@/hooks/useCategory';
import { formatDate } from '@/utils/format';
import type { BadgeCategory, CreateCategoryRequest, UpdateCategoryRequest } from '@/types';
import type { CategoryListItem } from '@/services/category';
import CategoryForm from './components/CategoryForm';

const CategoriesPage: React.FC = () => {
  const actionRef = useRef<ActionType>();

  // 表单弹窗状态
  const [formOpen, setFormOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<BadgeCategory | undefined>();

  // 搜索参数状态（ProTable 会自动管理，此处用于 React Query）
  const [searchParams, setSearchParams] = useState<{
    page: number;
    pageSize: number;
    name?: string;
    status?: 'ACTIVE' | 'INACTIVE';
  }>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useCategoryList(searchParams);
  const createMutation = useCreateCategory();
  const updateMutation = useUpdateCategory();
  const deleteMutation = useDeleteCategory();
  const toggleStatusMutation = useToggleCategoryStatus();
  const updateSortMutation = useUpdateCategorySortOrder();

  /**
   * 打开新建弹窗
   */
  const handleCreate = () => {
    setEditingCategory(undefined);
    setFormOpen(true);
  };

  /**
   * 打开编辑弹窗
   */
  const handleEdit = (record: CategoryListItem) => {
    setEditingCategory(record);
    setFormOpen(true);
  };

  /**
   * 删除分类
   *
   * 使用 Popconfirm 确认，防止误操作
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
    // 删除后刷新列表
    actionRef.current?.reload();
  };

  /**
   * 切换分类状态
   */
  const handleToggleStatus = async (record: CategoryListItem, checked: boolean) => {
    const newStatus = checked ? 'ACTIVE' : 'INACTIVE';
    await toggleStatusMutation.mutateAsync({
      id: record.id,
      status: newStatus,
    });
    // 刷新列表以更新状态
    actionRef.current?.reload();
  };

  /**
   * 更新排序值
   *
   * 使用 onBlur 触发更新，避免频繁请求
   */
  const handleSortOrderChange = async (id: number, sortOrder: number | null) => {
    if (sortOrder === null || sortOrder === undefined) return;
    await updateSortMutation.mutateAsync({ id, sortOrder });
    actionRef.current?.reload();
  };

  /**
   * 表单提交处理
   */
  const handleFormSubmit = async (values: CreateCategoryRequest): Promise<boolean> => {
    try {
      if (editingCategory) {
        // 编辑模式
        await updateMutation.mutateAsync({
          id: editingCategory.id,
          data: values as UpdateCategoryRequest,
        });
      } else {
        // 新建模式
        await createMutation.mutateAsync(values);
      }
      // 刷新列表
      actionRef.current?.reload();
      return true;
    } catch {
      return false;
    }
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<CategoryListItem>[] = [
    {
      title: '分类 ID',
      dataIndex: 'id',
      width: 80,
      search: false,
      sorter: true,
    },
    {
      title: '名称',
      dataIndex: 'name',
      width: 150,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '图标',
      dataIndex: 'iconUrl',
      width: 80,
      search: false,
      render: (_, record) =>
        record.iconUrl ? (
          <img
            src={record.iconUrl}
            alt={record.name}
            style={{ width: 32, height: 32, objectFit: 'contain' }}
          />
        ) : (
          '-'
        ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      valueType: 'select',
      valueEnum: {
        ACTIVE: { text: '启用', status: 'Success' },
        INACTIVE: { text: '禁用', status: 'Default' },
      },
      render: (_, record) => (
        <Switch
          checked={record.status === 'ACTIVE'}
          checkedChildren="启用"
          unCheckedChildren="禁用"
          loading={toggleStatusMutation.isPending}
          onChange={(checked) => handleToggleStatus(record, checked)}
        />
      ),
    },
    {
      title: '排序',
      dataIndex: 'sortOrder',
      width: 100,
      search: false,
      sorter: true,
      render: (_, record) => (
        <InputNumber
          size="small"
          min={0}
          max={9999}
          defaultValue={record.sortOrder}
          onBlur={(e) => {
            const value = parseInt(e.target.value, 10);
            if (!isNaN(value) && value !== record.sortOrder) {
              handleSortOrderChange(record.id, value);
            }
          }}
          onPressEnter={(e) => {
            const target = e.target as HTMLInputElement;
            const value = parseInt(target.value, 10);
            if (!isNaN(value) && value !== record.sortOrder) {
              handleSortOrderChange(record.id, value);
            }
            target.blur();
          }}
          style={{ width: 70 }}
        />
      ),
    },
    {
      title: '系列数',
      dataIndex: 'seriesCount',
      width: 80,
      search: false,
      tooltip: '该分类下的系列数量',
    },
    {
      title: '徽章数',
      dataIndex: 'badgeCount',
      width: 80,
      search: false,
      tooltip: '该分类下的徽章总数',
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 170,
      search: false,
      sorter: true,
      render: (_, record) => formatDate(record.createdAt),
    },
    {
      title: '更新时间',
      dataIndex: 'updatedAt',
      width: 170,
      search: false,
      render: (_, record) => formatDate(record.updatedAt),
    },
    {
      title: '操作',
      valueType: 'option',
      width: 120,
      fixed: 'right',
      render: (_, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleEdit(record)}
          >
            编辑
          </Button>
          <Popconfirm
            title="确认删除"
            description={
              record.seriesCount > 0
                ? `该分类下有 ${record.seriesCount} 个系列，请先删除系列`
                : '确定要删除该分类吗？'
            }
            onConfirm={() => handleDelete(record.id)}
            okText="确认"
            cancelText="取消"
            disabled={record.seriesCount > 0}
          >
            <Button
              type="link"
              size="small"
              danger
              icon={<DeleteOutlined />}
              disabled={record.seriesCount > 0}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="分类管理">
      <ProTable<CategoryListItem>
        actionRef={actionRef}
        rowKey="id"
        columns={columns}
        dataSource={data?.items}
        loading={isLoading}
        pagination={{
          current: searchParams.page,
          pageSize: searchParams.pageSize,
          total: data?.total || 0,
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total) => `共 ${total} 条`,
        }}
        search={{
          labelWidth: 'auto',
          defaultCollapsed: false,
        }}
        options={{
          density: true,
          fullScreen: true,
          reload: () => refetch(),
          setting: true,
        }}
        toolBarRender={() => [
          <Button
            key="create"
            type="primary"
            icon={<PlusOutlined />}
            onClick={handleCreate}
          >
            新建分类
          </Button>,
          <Button
            key="reload"
            icon={<ReloadOutlined />}
            onClick={() => refetch()}
          >
            刷新
          </Button>,
        ]}
        request={async (params, sort) => {
          // 处理搜索和分页参数
          const newParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            name: params.name as string | undefined,
            status: params.status as 'ACTIVE' | 'INACTIVE' | undefined,
            // 处理排序
            sortField: Object.keys(sort || {})[0],
            sortOrder: Object.values(sort || {})[0] as 'ascend' | 'descend' | undefined,
          };

          // 更新搜索参数状态，触发 React Query 重新请求
          setSearchParams(newParams);

          // ProTable 需要返回数据，但实际数据由 React Query 管理
          // 这里返回空数据，让 dataSource 接管
          return {
            data: [],
            success: true,
            total: 0,
          };
        }}
        scroll={{ x: 1200 }}
      />

      {/* 新建/编辑表单弹窗 */}
      <CategoryForm
        open={formOpen}
        onOpenChange={setFormOpen}
        initialValues={editingCategory}
        onSubmit={handleFormSubmit}
        loading={createMutation.isPending || updateMutation.isPending}
      />
    </PageContainer>
  );
};

export default CategoriesPage;

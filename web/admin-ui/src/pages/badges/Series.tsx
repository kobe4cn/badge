/**
 * 徽章系列管理页面
 *
 * 管理徽章系列，系列是同一主题下的徽章集合，
 * 包含列表展示、CRUD 操作、状态切换和徽章预览
 */

import React, { useRef, useState } from 'react';
import { Button, Switch, Popconfirm, Space, InputNumber, Image, Tag } from 'antd';
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
  EyeOutlined,
} from '@ant-design/icons';
import {
  useSeriesList,
  useCreateSeries,
  useUpdateSeries,
  useDeleteSeries,
  useToggleSeriesStatus,
  useUpdateSeriesSortOrder,
} from '@/hooks/useSeries';
import { useAllCategories } from '@/hooks/useCategory';
import { formatDate } from '@/utils/format';
import type { BadgeSeries, CreateSeriesRequest, UpdateSeriesRequest, CategoryStatus } from '@/types';
import type { SeriesListItem } from '@/services/series';
import SeriesForm from './components/SeriesForm';
import SeriesBadgesDrawer from './components/SeriesBadgesDrawer';

const SeriesPage: React.FC = () => {
  const actionRef = useRef<ActionType>();

  // 表单弹窗状态
  const [formOpen, setFormOpen] = useState(false);
  const [editingSeries, setEditingSeries] = useState<BadgeSeries | undefined>();

  // 徽章预览抽屉状态
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedSeries, setSelectedSeries] = useState<SeriesListItem | null>(null);

  // 搜索参数状态
  const [searchParams, setSearchParams] = useState<{
    page: number;
    pageSize: number;
    name?: string;
    categoryId?: number;
    status?: CategoryStatus;
  }>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useSeriesList(searchParams);
  const { data: categories } = useAllCategories();
  const createMutation = useCreateSeries();
  const updateMutation = useUpdateSeries();
  const deleteMutation = useDeleteSeries();
  const toggleStatusMutation = useToggleSeriesStatus();
  const updateSortMutation = useUpdateSeriesSortOrder();

  // 分类选项映射（用于筛选下拉和表格显示）
  const categoryMap = React.useMemo(() => {
    const map = new Map<number, string>();
    categories?.forEach((cat) => {
      map.set(cat.id, cat.name);
    });
    return map;
  }, [categories]);

  const categoryValueEnum = React.useMemo(() => {
    const enumObj: Record<number, { text: string }> = {};
    categories?.forEach((cat) => {
      enumObj[cat.id] = { text: cat.name };
    });
    return enumObj;
  }, [categories]);

  /**
   * 打开新建弹窗
   */
  const handleCreate = () => {
    setEditingSeries(undefined);
    setFormOpen(true);
  };

  /**
   * 打开编辑弹窗
   */
  const handleEdit = (record: SeriesListItem) => {
    setEditingSeries(record);
    setFormOpen(true);
  };

  /**
   * 删除系列
   *
   * 使用 Popconfirm 确认，防止误操作
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
    actionRef.current?.reload();
  };

  /**
   * 切换系列状态
   */
  const handleToggleStatus = async (record: SeriesListItem, checked: boolean) => {
    const newStatus: CategoryStatus = checked ? 'ACTIVE' : 'INACTIVE';
    await toggleStatusMutation.mutateAsync({
      id: record.id,
      status: newStatus,
    });
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
   * 查看系列下的徽章
   */
  const handleViewBadges = (record: SeriesListItem) => {
    setSelectedSeries(record);
    setDrawerOpen(true);
  };

  /**
   * 表单提交处理
   */
  const handleFormSubmit = async (
    values: CreateSeriesRequest & { status?: CategoryStatus }
  ): Promise<boolean> => {
    try {
      if (editingSeries) {
        // 编辑模式
        const updateData: UpdateSeriesRequest = {
          name: values.name,
          description: values.description,
          coverUrl: values.coverUrl,
          sortOrder: values.sortOrder,
          startTime: values.startTime,
          endTime: values.endTime,
        };
        await updateMutation.mutateAsync({
          id: editingSeries.id,
          data: updateData,
        });
      } else {
        // 新建模式
        await createMutation.mutateAsync(values);
      }
      actionRef.current?.reload();
      return true;
    } catch {
      return false;
    }
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<SeriesListItem>[] = [
    {
      title: '系列 ID',
      dataIndex: 'id',
      width: 80,
      search: false,
      sorter: true,
    },
    {
      title: '封面图',
      dataIndex: 'coverUrl',
      width: 80,
      search: false,
      render: (_, record) =>
        record.coverUrl ? (
          <Image
            src={record.coverUrl}
            alt={record.name}
            width={48}
            height={48}
            style={{ objectFit: 'cover', borderRadius: 4 }}
            preview={{
              mask: '预览',
            }}
          />
        ) : (
          <div
            style={{
              width: 48,
              height: 48,
              background: '#f5f5f5',
              borderRadius: 4,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#999',
              fontSize: 12,
            }}
          >
            无图
          </div>
        ),
    },
    {
      title: '名称',
      dataIndex: 'name',
      width: 150,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '所属分类',
      dataIndex: 'categoryId',
      width: 120,
      valueType: 'select',
      valueEnum: categoryValueEnum,
      render: (_, record) => (
        <Tag>{record.categoryName || categoryMap.get(record.categoryId) || '-'}</Tag>
      ),
    },
    {
      title: '徽章数量',
      dataIndex: 'badgeCount',
      width: 100,
      search: false,
      tooltip: '该系列下的徽章数量',
      sorter: true,
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
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 170,
      search: false,
      sorter: true,
      render: (_, record) => formatDate(record.createdAt),
    },
    {
      title: '操作',
      valueType: 'option',
      width: 180,
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
          <Button
            type="link"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => handleViewBadges(record)}
          >
            徽章
          </Button>
          <Popconfirm
            title="确认删除"
            description={
              record.badgeCount > 0
                ? `该系列下有 ${record.badgeCount} 个徽章，请先删除徽章`
                : '确定要删除该系列吗？'
            }
            onConfirm={() => handleDelete(record.id)}
            okText="确认"
            cancelText="取消"
            disabled={record.badgeCount > 0}
          >
            <Button
              type="link"
              size="small"
              danger
              icon={<DeleteOutlined />}
              disabled={record.badgeCount > 0}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="系列管理">
      <ProTable<SeriesListItem>
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
            新建系列
          </Button>,
          <Button key="reload" icon={<ReloadOutlined />} onClick={() => refetch()}>
            刷新
          </Button>,
        ]}
        request={async (params, sort) => {
          // 处理搜索和分页参数
          const newParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            name: params.name as string | undefined,
            categoryId: params.categoryId as number | undefined,
            status: params.status as CategoryStatus | undefined,
            // 处理排序
            sortField: Object.keys(sort || {})[0],
            sortOrder: Object.values(sort || {})[0] as 'ascend' | 'descend' | undefined,
          };

          // 更新搜索参数状态，触发 React Query 重新请求
          setSearchParams(newParams);

          // ProTable 需要返回数据，但实际数据由 React Query 管理
          return {
            data: [],
            success: true,
            total: 0,
          };
        }}
        scroll={{ x: 1200 }}
      />

      {/* 新建/编辑表单弹窗 */}
      <SeriesForm
        open={formOpen}
        onOpenChange={setFormOpen}
        initialValues={editingSeries}
        onSubmit={handleFormSubmit}
        loading={createMutation.isPending || updateMutation.isPending}
      />

      {/* 系列徽章预览抽屉 */}
      <SeriesBadgesDrawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        series={selectedSeries}
        categoryName={
          selectedSeries
            ? selectedSeries.categoryName || categoryMap.get(selectedSeries.categoryId)
            : undefined
        }
      />
    </PageContainer>
  );
};

export default SeriesPage;

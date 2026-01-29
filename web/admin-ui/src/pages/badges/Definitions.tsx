/**
 * 徽章定义管理页面
 *
 * 提供徽章的 CRUD 操作，包含列表展示、多维度筛选、上下架和库存管理
 */

import React, { useRef, useState } from 'react';
import {
  Button,
  Space,
  InputNumber,
  Tag,
  Image,
  Dropdown,
  Modal,
} from 'antd';
import type { MenuProps } from 'antd';
import {
  PageContainer,
  ProTable,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  ReloadOutlined,
  EyeOutlined,
  EditOutlined,
  DeleteOutlined,
  UploadOutlined,
  DownloadOutlined,
  MoreOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import {
  useBadgeList,
  useCreateBadge,
  useUpdateBadge,
  useDeleteBadge,
  usePublishBadge,
  useUnpublishBadge,
  useUpdateBadgeSortOrder,
} from '@/hooks/useBadge';
import { useAllCategories } from '@/hooks/useCategory';
import { useAllSeries } from '@/hooks/useSeries';
import {
  formatDate,
  getBadgeStatusText,
  getBadgeTypeText,
  formatCount,
} from '@/utils/format';
import type {
  Badge,
  BadgeType,
  BadgeStatus,
  CreateBadgeRequest,
  UpdateBadgeRequest,
} from '@/types';
import type { BadgeListItem, BadgeListParams } from '@/services/badge';
import BadgeForm from './components/BadgeForm';
import BadgeDetailDrawer from './components/BadgeDetailDrawer';

/**
 * 徽章状态颜色映射
 */
const BADGE_STATUS_COLOR: Record<BadgeStatus, string> = {
  DRAFT: 'default',
  ACTIVE: 'success',
  INACTIVE: 'warning',
  ARCHIVED: 'error',
};

/**
 * 徽章类型颜色映射
 */
const BADGE_TYPE_COLOR: Record<BadgeType, string> = {
  NORMAL: 'blue',
  LIMITED: 'orange',
  ACHIEVEMENT: 'purple',
  EVENT: 'green',
};

const DefinitionsPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [modal, contextHolder] = Modal.useModal();

  // 表单弹窗状态
  const [formOpen, setFormOpen] = useState(false);
  const [editingBadge, setEditingBadge] = useState<
    (Badge & { categoryId?: number }) | undefined
  >();

  // 详情抽屉状态
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailBadgeId, setDetailBadgeId] = useState<number | null>(null);

  // 筛选联动：分类影响系列下拉
  const [filterCategoryId, setFilterCategoryId] = useState<number | undefined>();

  // 搜索参数状态
  const [searchParams, setSearchParams] = useState<BadgeListParams>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useBadgeList(searchParams);
  const createMutation = useCreateBadge();
  const updateMutation = useUpdateBadge();
  const deleteMutation = useDeleteBadge();
  const publishMutation = usePublishBadge();
  const unpublishMutation = useUnpublishBadge();
  const updateSortMutation = useUpdateBadgeSortOrder();

  // 分类和系列数据（用于筛选下拉）
  const { data: categories } = useAllCategories();
  const { data: seriesList } = useAllSeries(filterCategoryId);

  /**
   * 分类选项（筛选用）
   */
  const categoryValueEnum =
    categories?.reduce(
      (acc, cat) => {
        acc[cat.id] = { text: cat.name };
        return acc;
      },
      {} as Record<number, { text: string }>
    ) || {};

  /**
   * 系列选项（筛选用，联动分类）
   */
  const seriesValueEnum =
    seriesList?.reduce(
      (acc, series) => {
        acc[series.id] = { text: series.name };
        return acc;
      },
      {} as Record<number, { text: string }>
    ) || {};

  /**
   * 打开新建弹窗
   */
  const handleCreate = () => {
    setEditingBadge(undefined);
    setFormOpen(true);
  };

  /**
   * 打开编辑弹窗
   */
  const handleEdit = (record: BadgeListItem) => {
    setEditingBadge({
      ...record,
      categoryId: record.categoryId,
    });
    setFormOpen(true);
  };

  /**
   * 打开详情抽屉
   */
  const handleViewDetail = (record: BadgeListItem) => {
    setDetailBadgeId(record.id);
    setDetailOpen(true);
  };

  /**
   * 删除徽章
   *
   * 仅允许删除草稿状态的徽章
   */
  const handleDelete = async (id: number) => {
    await deleteMutation.mutateAsync(id);
    actionRef.current?.reload();
  };

  /**
   * 上架徽章
   */
  const handlePublish = async (record: BadgeListItem) => {
    // 上架前确认
    modal.confirm({
      title: '确认上架',
      icon: <ExclamationCircleOutlined />,
      content: `确定要上架徽章「${record.name}」吗？上架后用户可以获取该徽章。`,
      okText: '确认上架',
      cancelText: '取消',
      onOk: async () => {
        await publishMutation.mutateAsync(record.id);
        actionRef.current?.reload();
      },
    });
  };

  /**
   * 下架徽章
   */
  const handleUnpublish = async (record: BadgeListItem) => {
    modal.confirm({
      title: '确认下架',
      icon: <ExclamationCircleOutlined />,
      content: `确定要下架徽章「${record.name}」吗？下架后用户将无法获取该徽章。`,
      okText: '确认下架',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        await unpublishMutation.mutateAsync(record.id);
        actionRef.current?.reload();
      },
    });
  };

  /**
   * 更新排序值
   */
  const handleSortOrderChange = async (id: number, sortOrder: number | null) => {
    if (sortOrder === null || sortOrder === undefined) return;
    await updateSortMutation.mutateAsync({ id, sortOrder });
    actionRef.current?.reload();
  };

  /**
   * 表单提交处理
   */
  const handleFormSubmit = async (values: CreateBadgeRequest): Promise<boolean> => {
    try {
      if (editingBadge) {
        // 编辑模式
        await updateMutation.mutateAsync({
          id: editingBadge.id,
          data: values as UpdateBadgeRequest,
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
   * 操作菜单
   */
  const getActionMenuItems = (record: BadgeListItem): MenuProps['items'] => {
    const items: MenuProps['items'] = [
      {
        key: 'view',
        label: '查看详情',
        icon: <EyeOutlined />,
        onClick: () => handleViewDetail(record),
      },
      {
        key: 'edit',
        label: '编辑',
        icon: <EditOutlined />,
        onClick: () => handleEdit(record),
      },
    ];

    // 根据状态添加上架/下架操作
    if (record.status === 'DRAFT' || record.status === 'INACTIVE') {
      items.push({
        key: 'publish',
        label: '上架',
        icon: <UploadOutlined />,
        onClick: () => handlePublish(record),
      });
    }

    if (record.status === 'ACTIVE') {
      items.push({
        key: 'unpublish',
        label: '下架',
        icon: <DownloadOutlined />,
        danger: true,
        onClick: () => handleUnpublish(record),
      });
    }

    // 仅草稿状态可删除
    if (record.status === 'DRAFT') {
      items.push({ type: 'divider' });
      items.push({
        key: 'delete',
        label: '删除',
        icon: <DeleteOutlined />,
        danger: true,
        onClick: () => {
          modal.confirm({
            title: '确认删除',
            icon: <ExclamationCircleOutlined />,
            content: `确定要删除徽章「${record.name}」吗？删除后不可恢复。`,
            okText: '确认删除',
            okButtonProps: { danger: true },
            cancelText: '取消',
            onOk: () => handleDelete(record.id),
          });
        },
      });
    }

    return items;
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<BadgeListItem>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
      sorter: true,
    },
    {
      title: '图标',
      dataIndex: ['assets', 'iconUrl'],
      width: 70,
      search: false,
      render: (_, record) =>
        record.assets.iconUrl ? (
          <Image
            src={record.assets.iconUrl}
            alt={record.name}
            width={40}
            height={40}
            style={{ objectFit: 'contain' }}
            preview={{ mask: '预览' }}
          />
        ) : (
          <div
            style={{
              width: 40,
              height: 40,
              background: '#f5f5f5',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#999',
              fontSize: 12,
            }}
          >
            暂无
          </div>
        ),
    },
    {
      title: '徽章名称',
      dataIndex: 'name',
      width: 150,
      ellipsis: true,
      copyable: true,
    },
    {
      title: '类型',
      dataIndex: 'badgeType',
      width: 100,
      valueType: 'select',
      valueEnum: {
        NORMAL: { text: '普通徽章' },
        LIMITED: { text: '限定徽章' },
        ACHIEVEMENT: { text: '成就徽章' },
        EVENT: { text: '活动徽章' },
      },
      render: (_, record) => (
        <Tag color={BADGE_TYPE_COLOR[record.badgeType]}>
          {getBadgeTypeText(record.badgeType)}
        </Tag>
      ),
    },
    {
      title: '所属分类',
      dataIndex: 'categoryId',
      width: 120,
      valueType: 'select',
      valueEnum: categoryValueEnum,
      fieldProps: {
        showSearch: true,
        onChange: (value: number | undefined) => {
          setFilterCategoryId(value);
        },
      },
      render: (_, record) => record.categoryName || '-',
    },
    {
      title: '所属系列',
      dataIndex: 'seriesId',
      width: 120,
      valueType: 'select',
      valueEnum: seriesValueEnum,
      fieldProps: {
        showSearch: true,
      },
      render: (_, record) => record.seriesName || '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      valueType: 'select',
      valueEnum: {
        DRAFT: { text: '草稿', status: 'Default' },
        ACTIVE: { text: '已上线', status: 'Success' },
        INACTIVE: { text: '已下线', status: 'Warning' },
        ARCHIVED: { text: '已归档', status: 'Error' },
      },
      render: (_, record) => (
        <Tag color={BADGE_STATUS_COLOR[record.status]}>
          {getBadgeStatusText(record.status)}
        </Tag>
      ),
    },
    {
      title: '库存',
      dataIndex: 'maxSupply',
      width: 100,
      search: false,
      render: (_, record) =>
        record.maxSupply ? (
          <span>
            {formatCount(record.issuedCount)} / {formatCount(record.maxSupply)}
          </span>
        ) : (
          <span style={{ color: '#52c41a' }}>不限量</span>
        ),
    },
    {
      title: '发放数',
      dataIndex: 'issuedCount',
      width: 80,
      search: false,
      sorter: true,
      render: (_, record) => formatCount(record.issuedCount),
    },
    {
      title: '排序',
      dataIndex: 'sortOrder',
      width: 90,
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
      width: 150,
      fixed: 'right',
      render: (_, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => handleViewDetail(record)}
          >
            详情
          </Button>
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleEdit(record)}
          >
            编辑
          </Button>
          <Dropdown
            menu={{ items: getActionMenuItems(record) }}
            trigger={['click']}
          >
            <Button type="link" size="small" icon={<MoreOutlined />} />
          </Dropdown>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="徽章定义">
      {contextHolder}
      <ProTable<BadgeListItem>
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
            新建徽章
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
          const newParams: BadgeListParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            name: params.name as string | undefined,
            badgeType: params.badgeType as BadgeType | undefined,
            status: params.status as BadgeStatus | undefined,
            categoryId: params.categoryId as number | undefined,
            seriesId: params.seriesId as number | undefined,
            // 处理排序
            sortField: Object.keys(sort || {})[0],
            sortOrder: Object.values(sort || {})[0] as 'ascend' | 'descend' | undefined,
          };

          // 更新筛选分类状态（用于系列联动）
          if (params.categoryId !== filterCategoryId) {
            setFilterCategoryId(params.categoryId as number | undefined);
          }

          // 更新搜索参数状态
          setSearchParams(newParams);

          // ProTable 需要返回数据，但实际数据由 React Query 管理
          return {
            data: [],
            success: true,
            total: 0,
          };
        }}
        scroll={{ x: 1400 }}
      />

      {/* 新建/编辑表单弹窗 */}
      <BadgeForm
        open={formOpen}
        onOpenChange={setFormOpen}
        initialValues={editingBadge}
        onSubmit={handleFormSubmit}
        loading={createMutation.isPending || updateMutation.isPending}
      />

      {/* 详情抽屉 */}
      <BadgeDetailDrawer
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        badgeId={detailBadgeId}
      />
    </PageContainer>
  );
};

export default DefinitionsPage;

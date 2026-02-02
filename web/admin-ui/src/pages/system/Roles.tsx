/**
 * 系统角色管理页面
 *
 * 提供角色的 CRUD 操作，包含列表展示、权限分配和成员查看
 */

import React, { useRef, useState, useEffect } from 'react';
import { Button, Space, Tag, Modal, Tree, Descriptions } from 'antd';
import type { TreeDataNode } from 'antd';
import {
  PageContainer,
  ProTable,
  DrawerForm,
  ProFormText,
  ProFormTextArea,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ExclamationCircleOutlined,
  SafetyOutlined,
} from '@ant-design/icons';
import {
  useRoleList,
  useRoleDetail,
  useCreateRole,
  useUpdateRole,
  useDeleteRole,
  usePermissionTree,
} from '@/hooks/useSystem';
import { formatDate } from '@/utils/format';
import type {
  SystemRole,
  RoleListParams,
  CreateRoleRequest,
  UpdateRoleRequest,
  PermissionTreeNode,
} from '@/services/system';

const RolesPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [modal, contextHolder] = Modal.useModal();

  // 表单状态
  const [formOpen, setFormOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<SystemRole | undefined>();
  const [checkedPermissions, setCheckedPermissions] = useState<number[]>([]);

  // 详情抽屉
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailRoleId, setDetailRoleId] = useState<number | null>(null);

  // 搜索参数
  const [searchParams, setSearchParams] = useState<RoleListParams>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useRoleList(searchParams);
  const { data: permissionTree } = usePermissionTree();
  const { data: roleDetail } = useRoleDetail(detailRoleId || 0, !!detailRoleId);
  const createMutation = useCreateRole();
  const updateMutation = useUpdateRole();
  const deleteMutation = useDeleteRole();

  /**
   * 将权限树转换为 Ant Design Tree 格式
   */
  const treeData: TreeDataNode[] = permissionTree?.map((module: PermissionTreeNode) => ({
    title: module.moduleName,
    key: `module-${module.module}`,
    children: module.permissions.map((perm) => ({
      title: `${perm.name} (${perm.code})`,
      key: perm.id,
    })),
  })) || [];

  /**
   * 编辑角色时加载权限
   */
  useEffect(() => {
    if (editingRole && roleDetail && roleDetail.id === editingRole.id) {
      setCheckedPermissions(roleDetail.permissions.map((p) => p.id));
    }
  }, [editingRole, roleDetail]);

  /**
   * 打开新建弹窗
   */
  const handleCreate = () => {
    setEditingRole(undefined);
    setCheckedPermissions([]);
    setFormOpen(true);
  };

  /**
   * 打开编辑弹窗
   */
  const handleEdit = (record: SystemRole) => {
    setEditingRole(record);
    setDetailRoleId(record.id); // 触发加载详情
    setFormOpen(true);
  };

  /**
   * 查看详情
   */
  const handleViewDetail = (record: SystemRole) => {
    setDetailRoleId(record.id);
    setDetailOpen(true);
  };

  /**
   * 删除角色
   */
  const handleDelete = (record: SystemRole) => {
    if (record.builtIn) {
      modal.warning({
        title: '无法删除',
        content: '内置角色不能删除',
      });
      return;
    }

    modal.confirm({
      title: '确认删除',
      icon: <ExclamationCircleOutlined />,
      content: `确定要删除角色「${record.name}」吗？删除后该角色的用户将失去相关权限。`,
      okText: '确认删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        await deleteMutation.mutateAsync(record.id);
        actionRef.current?.reload();
      },
    });
  };

  /**
   * 权限树选中变化
   */
  const handlePermissionCheck = (
    checkedKeys: React.Key[] | { checked: React.Key[]; halfChecked: React.Key[] }
  ) => {
    const keys = Array.isArray(checkedKeys) ? checkedKeys : checkedKeys.checked;
    // 过滤掉模块节点（字符串类型），只保留权限ID（数字类型）
    setCheckedPermissions(keys.filter((k): k is number => typeof k === 'number'));
  };

  /**
   * 表单提交处理
   */
  const handleFormSubmit = async (values: { code?: string; name: string; description?: string }): Promise<boolean> => {
    try {
      if (editingRole) {
        const updateData: UpdateRoleRequest = {
          name: values.name,
          description: values.description,
          permissionIds: checkedPermissions,
        };
        await updateMutation.mutateAsync({
          id: editingRole.id,
          data: updateData,
        });
      } else {
        const createData: CreateRoleRequest = {
          code: values.code!,
          name: values.name,
          description: values.description,
          permissionIds: checkedPermissions,
        };
        await createMutation.mutateAsync(createData);
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
  const columns: ProColumns<SystemRole>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
    },
    {
      title: '角色编码',
      dataIndex: 'code',
      width: 120,
      copyable: true,
      search: false,
    },
    {
      title: '角色名称',
      dataIndex: 'name',
      width: 150,
    },
    {
      title: '描述',
      dataIndex: 'description',
      width: 200,
      ellipsis: true,
      search: false,
      render: (_, record) => record.description || '-',
    },
    {
      title: '用户数',
      dataIndex: 'userCount',
      width: 90,
      search: false,
    },
    {
      title: '权限数',
      dataIndex: 'permissionCount',
      width: 90,
      search: false,
    },
    {
      title: '类型',
      dataIndex: 'builtIn',
      width: 90,
      search: false,
      render: (_, record) => (
        record.builtIn ? (
          <Tag color="blue">内置</Tag>
        ) : (
          <Tag>自定义</Tag>
        )
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'createdAt',
      width: 170,
      search: false,
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
            icon={<SafetyOutlined />}
            onClick={() => handleViewDetail(record)}
          >
            权限
          </Button>
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleEdit(record)}
            disabled={record.builtIn}
          >
            编辑
          </Button>
          <Button
            type="link"
            size="small"
            danger
            icon={<DeleteOutlined />}
            onClick={() => handleDelete(record)}
            disabled={record.builtIn}
          >
            删除
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="角色管理">
      {contextHolder}
      <ProTable<SystemRole>
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
            新建角色
          </Button>,
        ]}
        request={async (params) => {
          const newParams: RoleListParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            name: params.name as string | undefined,
          };
          setSearchParams(newParams);
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1000 }}
      />

      {/* 新建/编辑角色弹窗 */}
      <DrawerForm
        title={editingRole ? '编辑角色' : '新建角色'}
        open={formOpen}
        onOpenChange={(open) => {
          setFormOpen(open);
          if (!open) {
            setEditingRole(undefined);
            setCheckedPermissions([]);
          }
        }}
        width={600}
        initialValues={editingRole}
        onFinish={handleFormSubmit}
        submitter={{
          searchConfig: {
            submitText: editingRole ? '保存' : '创建',
          },
        }}
      >
        {!editingRole && (
          <ProFormText
            name="code"
            label="角色编码"
            placeholder="请输入角色编码（如 custom_role）"
            rules={[
              { required: true, message: '请输入角色编码' },
              { pattern: /^[a-z][a-z0-9_]*$/, message: '角色编码必须小写字母开头，只能包含小写字母、数字和下划线' },
              { max: 50, message: '角色编码最多50个字符' },
            ]}
          />
        )}
        <ProFormText
          name="name"
          label="角色名称"
          placeholder="请输入角色名称"
          rules={[
            { required: true, message: '请输入角色名称' },
            { max: 100, message: '角色名称最多100个字符' },
          ]}
        />
        <ProFormTextArea
          name="description"
          label="描述"
          placeholder="请输入角色描述"
          fieldProps={{ rows: 3 }}
        />

        <div style={{ marginBottom: 8 }}>
          <label style={{ fontWeight: 500 }}>权限配置</label>
        </div>
        <div
          style={{
            border: '1px solid #d9d9d9',
            borderRadius: 6,
            padding: 12,
            maxHeight: 400,
            overflow: 'auto',
          }}
        >
          <Tree
            checkable
            checkStrictly
            treeData={treeData}
            checkedKeys={checkedPermissions}
            onCheck={handlePermissionCheck}
            defaultExpandAll
          />
        </div>
      </DrawerForm>

      {/* 角色详情弹窗 */}
      <Modal
        title="角色权限详情"
        open={detailOpen}
        onCancel={() => setDetailOpen(false)}
        footer={null}
        width={600}
      >
        {roleDetail && (
          <>
            <Descriptions column={2} bordered size="small" style={{ marginBottom: 16 }}>
              <Descriptions.Item label="角色编码">{roleDetail.code}</Descriptions.Item>
              <Descriptions.Item label="角色名称">{roleDetail.name}</Descriptions.Item>
              <Descriptions.Item label="描述" span={2}>
                {roleDetail.description || '-'}
              </Descriptions.Item>
            </Descriptions>

            <div style={{ marginBottom: 8 }}>
              <strong>已分配权限（{roleDetail.permissions.length} 个）</strong>
            </div>
            <div
              style={{
                border: '1px solid #d9d9d9',
                borderRadius: 6,
                padding: 12,
                maxHeight: 300,
                overflow: 'auto',
              }}
            >
              {roleDetail.permissions.length > 0 ? (
                <Space wrap>
                  {roleDetail.permissions.map((perm) => (
                    <Tag key={perm.id} color="blue">
                      {perm.name}
                    </Tag>
                  ))}
                </Space>
              ) : (
                <span style={{ color: '#999' }}>暂无权限</span>
              )}
            </div>
          </>
        )}
      </Modal>
    </PageContainer>
  );
};

export default RolesPage;

/**
 * 系统用户管理页面
 *
 * 提供用户的 CRUD 操作，包含列表展示、状态管理、角色分配和密码重置
 */

import React, { useRef, useState } from 'react';
import { Button, Space, Tag, Modal, Form, Input } from 'antd';
import {
  PageContainer,
  ProTable,
  DrawerForm,
  ProFormText,
  ProFormSelect,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  KeyOutlined,
  ExclamationCircleOutlined,
  LockOutlined,
  UnlockOutlined,
} from '@ant-design/icons';
import {
  useUserList,
  useCreateUser,
  useUpdateUser,
  useDeleteUser,
  useResetPassword,
  useAllRoles,
} from '@/hooks/useSystem';
import { formatDate } from '@/utils/format';
import type { SystemUser, UserListParams, CreateUserRequest, UpdateUserRequest } from '@/services/system';

/**
 * 用户状态颜色映射
 */
const USER_STATUS_COLOR: Record<string, string> = {
  ACTIVE: 'success',
  DISABLED: 'default',
  LOCKED: 'error',
};

const USER_STATUS_TEXT: Record<string, string> = {
  ACTIVE: '正常',
  DISABLED: '已禁用',
  LOCKED: '已锁定',
};

const UsersPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [modal, contextHolder] = Modal.useModal();

  // 表单状态
  const [formOpen, setFormOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<SystemUser | undefined>();

  // 重置密码表单
  const [passwordModalOpen, setPasswordModalOpen] = useState(false);
  const [resetPasswordUserId, setResetPasswordUserId] = useState<number | null>(null);
  const [passwordForm] = Form.useForm();

  // 搜索参数
  const [searchParams, setSearchParams] = useState<UserListParams>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useUserList(searchParams);
  const { data: roles } = useAllRoles();
  const createMutation = useCreateUser();
  const updateMutation = useUpdateUser();
  const deleteMutation = useDeleteUser();
  const resetPasswordMutation = useResetPassword();

  /**
   * 角色选项
   */
  const roleOptions = roles?.map((role) => ({
    label: role.name,
    value: role.id,
  })) || [];

  /**
   * 打开新建弹窗
   */
  const handleCreate = () => {
    setEditingUser(undefined);
    setFormOpen(true);
  };

  /**
   * 打开编辑弹窗
   */
  const handleEdit = (record: SystemUser) => {
    setEditingUser(record);
    setFormOpen(true);
  };

  /**
   * 删除用户
   */
  const handleDelete = (record: SystemUser) => {
    modal.confirm({
      title: '确认删除',
      icon: <ExclamationCircleOutlined />,
      content: `确定要删除用户「${record.displayName}」吗？删除后不可恢复。`,
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
   * 启用/禁用用户
   */
  const handleToggleStatus = async (record: SystemUser) => {
    const newStatus = record.status === 'ACTIVE' ? 'DISABLED' : 'ACTIVE';
    const actionText = newStatus === 'ACTIVE' ? '启用' : '禁用';

    modal.confirm({
      title: `确认${actionText}`,
      icon: <ExclamationCircleOutlined />,
      content: `确定要${actionText}用户「${record.displayName}」吗？`,
      okText: `确认${actionText}`,
      okButtonProps: newStatus === 'DISABLED' ? { danger: true } : {},
      cancelText: '取消',
      onOk: async () => {
        await updateMutation.mutateAsync({
          id: record.id,
          data: { status: newStatus },
        });
        actionRef.current?.reload();
      },
    });
  };

  /**
   * 打开重置密码弹窗
   */
  const handleResetPassword = (record: SystemUser) => {
    setResetPasswordUserId(record.id);
    passwordForm.resetFields();
    setPasswordModalOpen(true);
  };

  /**
   * 提交重置密码
   */
  const handlePasswordSubmit = async () => {
    try {
      const values = await passwordForm.validateFields();
      if (resetPasswordUserId) {
        await resetPasswordMutation.mutateAsync({
          id: resetPasswordUserId,
          data: { newPassword: values.newPassword },
        });
        setPasswordModalOpen(false);
        actionRef.current?.reload();
      }
    } catch {
      // 表单校验失败
    }
  };

  /**
   * 表单提交处理
   */
  const handleFormSubmit = async (values: CreateUserRequest | UpdateUserRequest): Promise<boolean> => {
    try {
      if (editingUser) {
        await updateMutation.mutateAsync({
          id: editingUser.id,
          data: values as UpdateUserRequest,
        });
      } else {
        await createMutation.mutateAsync(values as CreateUserRequest);
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
  const columns: ProColumns<SystemUser>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
    },
    {
      title: '用户名',
      dataIndex: 'username',
      width: 120,
      copyable: true,
    },
    {
      title: '显示名称',
      dataIndex: 'displayName',
      width: 120,
      search: false,
    },
    {
      title: '邮箱',
      dataIndex: 'email',
      width: 180,
      search: false,
      render: (_, record) => record.email || '-',
    },
    {
      title: '手机号',
      dataIndex: 'phone',
      width: 130,
      search: false,
      render: (_, record) => record.phone || '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      valueType: 'select',
      valueEnum: {
        ACTIVE: { text: '正常', status: 'Success' },
        DISABLED: { text: '已禁用', status: 'Default' },
        LOCKED: { text: '已锁定', status: 'Error' },
      },
      render: (_, record) => (
        <Tag color={USER_STATUS_COLOR[record.status]}>
          {USER_STATUS_TEXT[record.status]}
        </Tag>
      ),
    },
    {
      title: '角色',
      dataIndex: 'roleId',
      width: 120,
      valueType: 'select',
      fieldProps: {
        options: roleOptions,
      },
      search: false,
      render: () => '-', // 列表不显示角色，详情查看
    },
    {
      title: '最后登录',
      dataIndex: 'lastLoginAt',
      width: 170,
      search: false,
      render: (_, record) => (record.lastLoginAt ? formatDate(record.lastLoginAt) : '-'),
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
      width: 200,
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
            icon={<KeyOutlined />}
            onClick={() => handleResetPassword(record)}
          >
            重置密码
          </Button>
          {record.status === 'ACTIVE' ? (
            <Button
              type="link"
              size="small"
              danger
              icon={<LockOutlined />}
              onClick={() => handleToggleStatus(record)}
            >
              禁用
            </Button>
          ) : record.status === 'DISABLED' ? (
            <Button
              type="link"
              size="small"
              icon={<UnlockOutlined />}
              onClick={() => handleToggleStatus(record)}
            >
              启用
            </Button>
          ) : null}
          {record.id !== 1 && (
            <Button
              type="link"
              size="small"
              danger
              icon={<DeleteOutlined />}
              onClick={() => handleDelete(record)}
            >
              删除
            </Button>
          )}
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="用户管理">
      {contextHolder}
      <ProTable<SystemUser>
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
            新建用户
          </Button>,
        ]}
        request={async (params) => {
          const newParams: UserListParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            username: params.username as string | undefined,
            status: params.status as 'ACTIVE' | 'DISABLED' | 'LOCKED' | undefined,
          };
          setSearchParams(newParams);
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1200 }}
      />

      {/* 新建/编辑用户弹窗 */}
      <DrawerForm
        title={editingUser ? '编辑用户' : '新建用户'}
        open={formOpen}
        onOpenChange={setFormOpen}
        width={480}
        initialValues={editingUser}
        onFinish={handleFormSubmit}
        submitter={{
          searchConfig: {
            submitText: editingUser ? '保存' : '创建',
          },
        }}
      >
        <ProFormText
          name="username"
          label="用户名"
          placeholder="请输入用户名"
          rules={[
            { required: true, message: '请输入用户名' },
            { min: 3, max: 50, message: '用户名长度为3-50个字符' },
            { pattern: /^[a-zA-Z][a-zA-Z0-9_]*$/, message: '用户名必须以字母开头，只能包含字母、数字和下划线' },
          ]}
          disabled={!!editingUser}
        />
        {!editingUser && (
          <ProFormText.Password
            name="password"
            label="密码"
            placeholder="请输入密码"
            rules={[
              { required: true, message: '请输入密码' },
              { min: 6, max: 100, message: '密码长度为6-100个字符' },
            ]}
          />
        )}
        <ProFormText
          name="displayName"
          label="显示名称"
          placeholder="请输入显示名称"
          rules={[
            { required: true, message: '请输入显示名称' },
            { max: 100, message: '显示名称最多100个字符' },
          ]}
        />
        <ProFormText
          name="email"
          label="邮箱"
          placeholder="请输入邮箱"
          rules={[
            { type: 'email', message: '请输入有效的邮箱地址' },
          ]}
        />
        <ProFormText
          name="phone"
          label="手机号"
          placeholder="请输入手机号"
          rules={[
            { pattern: /^1[3-9]\d{9}$/, message: '请输入有效的手机号' },
          ]}
        />
        <ProFormSelect
          name="roleIds"
          label="角色"
          placeholder="请选择角色"
          mode="multiple"
          options={roleOptions}
          rules={[{ required: true, message: '请选择至少一个角色' }]}
        />
        {editingUser && (
          <ProFormSelect
            name="status"
            label="状态"
            options={[
              { label: '正常', value: 'ACTIVE' },
              { label: '禁用', value: 'DISABLED' },
            ]}
          />
        )}
      </DrawerForm>

      {/* 重置密码弹窗 */}
      <Modal
        title="重置密码"
        open={passwordModalOpen}
        onOk={handlePasswordSubmit}
        onCancel={() => setPasswordModalOpen(false)}
        confirmLoading={resetPasswordMutation.isPending}
      >
        <Form form={passwordForm} layout="vertical">
          <Form.Item
            name="newPassword"
            label="新密码"
            rules={[
              { required: true, message: '请输入新密码' },
              { min: 6, max: 100, message: '密码长度为6-100个字符' },
            ]}
          >
            <Input.Password placeholder="请输入新密码" />
          </Form.Item>
          <Form.Item
            name="confirmPassword"
            label="确认密码"
            dependencies={['newPassword']}
            rules={[
              { required: true, message: '请确认密码' },
              ({ getFieldValue }) => ({
                validator(_, value) {
                  if (!value || getFieldValue('newPassword') === value) {
                    return Promise.resolve();
                  }
                  return Promise.reject(new Error('两次输入的密码不一致'));
                },
              }),
            ]}
          >
            <Input.Password placeholder="请再次输入新密码" />
          </Form.Item>
        </Form>
      </Modal>
    </PageContainer>
  );
};

export default UsersPage;

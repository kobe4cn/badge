/**
 * API Key 管理页面
 *
 * 提供 API Key 的创建、删除、启用/禁用、重新生成等操作
 * 密钥仅在创建或重新生成时展示一次，之后只显示前缀
 */

import React, { useRef, useState } from 'react';
import { Button, Space, Tag, Modal, Typography, Switch, Input } from 'antd';
import {
  PageContainer,
  ProTable,
  ModalForm,
  ProFormText,
  ProFormSelect,
  ProFormDigit,
  ProFormDateTimePicker,
  type ActionType,
  type ProColumns,
} from '@ant-design/pro-components';
import {
  PlusOutlined,
  DeleteOutlined,
  ExclamationCircleOutlined,
  CopyOutlined,
  ReloadOutlined,
  KeyOutlined,
} from '@ant-design/icons';
import {
  useApiKeyList,
  useCreateApiKey,
  useDeleteApiKey,
  useRegenerateApiKey,
  useToggleApiKeyStatus,
  usePermissionList,
} from '@/hooks/useSystem';
import { formatDate } from '@/utils/format';
import type { ApiKeyDto, ApiKeyListParams, CreateApiKeyRequest, CreateApiKeyResponse } from '@/services/system';

const { Paragraph } = Typography;

const ApiKeysPage: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [modal, contextHolder] = Modal.useModal();

  // 创建表单
  const [createOpen, setCreateOpen] = useState(false);

  // 密钥展示弹窗（创建或重新生成后展示完整密钥）
  const [keyResultOpen, setKeyResultOpen] = useState(false);
  const [keyResult, setKeyResult] = useState<CreateApiKeyResponse | null>(null);

  // 搜索参数
  const [searchParams, setSearchParams] = useState<ApiKeyListParams>({
    page: 1,
    pageSize: 20,
  });

  // React Query Hooks
  const { data, isLoading, refetch } = useApiKeyList(searchParams);
  const { data: permissions } = usePermissionList();
  const createMutation = useCreateApiKey();
  const deleteMutation = useDeleteApiKey();
  const regenerateMutation = useRegenerateApiKey();
  const toggleStatusMutation = useToggleApiKeyStatus();

  /**
   * 权限选项：按模块分组
   */
  const permissionOptions = permissions?.map((perm) => ({
    label: `${perm.name} (${perm.code})`,
    value: perm.code,
  })) || [];

  /**
   * 创建 API Key
   */
  const handleCreate = async (values: CreateApiKeyRequest): Promise<boolean> => {
    try {
      const result = await createMutation.mutateAsync(values);
      setKeyResult(result);
      setKeyResultOpen(true);
      actionRef.current?.reload();
      return true;
    } catch {
      return false;
    }
  };

  /**
   * 删除 API Key
   */
  const handleDelete = (record: ApiKeyDto) => {
    modal.confirm({
      title: '确认删除',
      icon: <ExclamationCircleOutlined />,
      content: `确定要删除 API Key「${record.name}」吗？删除后使用该 Key 的外部系统将无法访问。`,
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
   * 重新生成 API Key
   */
  const handleRegenerate = (record: ApiKeyDto) => {
    modal.confirm({
      title: '确认重新生成',
      icon: <ExclamationCircleOutlined />,
      content: `重新生成后旧 Key 将立即失效，确定要重新生成 API Key「${record.name}」吗？`,
      okText: '确认',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        const result = await regenerateMutation.mutateAsync(record.id);
        setKeyResult(result);
        setKeyResultOpen(true);
        actionRef.current?.reload();
      },
    });
  };

  /**
   * 切换启用状态
   */
  const handleToggleStatus = async (record: ApiKeyDto, checked: boolean) => {
    await toggleStatusMutation.mutateAsync({ id: record.id, enabled: checked });
    refetch();
  };

  /**
   * 表格列定义
   */
  const columns: ProColumns<ApiKeyDto>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 70,
      search: false,
    },
    {
      title: '名称',
      dataIndex: 'name',
      width: 160,
      ellipsis: true,
    },
    {
      title: 'Key 前缀',
      dataIndex: 'keyPrefix',
      width: 120,
      search: false,
      render: (_, record) => (
        <Tag icon={<KeyOutlined />}>{record.keyPrefix}...</Tag>
      ),
    },
    {
      title: '权限',
      dataIndex: 'permissions',
      width: 200,
      search: false,
      ellipsis: true,
      render: (_, record) => (
        <Space size={[0, 4]} wrap>
          {record.permissions.slice(0, 3).map((p) => (
            <Tag key={p} color="blue">{p}</Tag>
          ))}
          {record.permissions.length > 3 && (
            <Tag>+{record.permissions.length - 3}</Tag>
          )}
        </Space>
      ),
    },
    {
      title: '速率限制',
      dataIndex: 'rateLimit',
      width: 100,
      search: false,
      render: (_, record) => record.rateLimit ? `${record.rateLimit}/分` : '-',
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      width: 90,
      search: false,
      render: (_, record) => (
        <Switch
          checked={record.enabled}
          checkedChildren="启用"
          unCheckedChildren="禁用"
          loading={toggleStatusMutation.isPending}
          onChange={(checked) => handleToggleStatus(record, checked)}
        />
      ),
    },
    {
      title: '最后使用',
      dataIndex: 'lastUsedAt',
      width: 170,
      search: false,
      render: (_, record) => record.lastUsedAt ? formatDate(record.lastUsedAt) : '从未',
    },
    {
      title: '过期时间',
      dataIndex: 'expiresAt',
      width: 170,
      search: false,
      render: (_, record) => record.expiresAt ? formatDate(record.expiresAt) : '永不过期',
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
      width: 160,
      fixed: 'right',
      render: (_, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<ReloadOutlined />}
            onClick={() => handleRegenerate(record)}
          >
            重新生成
          </Button>
          <Button
            type="link"
            size="small"
            danger
            icon={<DeleteOutlined />}
            onClick={() => handleDelete(record)}
          >
            删除
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="API Key 管理">
      {contextHolder}
      <ProTable<ApiKeyDto>
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
            onClick={() => setCreateOpen(true)}
          >
            创建 API Key
          </Button>,
        ]}
        request={async (params) => {
          const newParams: ApiKeyListParams = {
            page: params.current || 1,
            pageSize: params.pageSize || 20,
            name: params.name as string | undefined,
          };
          setSearchParams(newParams);
          return { data: [], success: true, total: 0 };
        }}
        scroll={{ x: 1400 }}
      />

      {/* 创建 API Key 弹窗 */}
      <ModalForm<CreateApiKeyRequest>
        title="创建 API Key"
        open={createOpen}
        onOpenChange={setCreateOpen}
        width={520}
        onFinish={handleCreate}
        submitter={{
          searchConfig: { submitText: '创建' },
        }}
      >
        <ProFormText
          name="name"
          label="名称"
          placeholder="请输入 API Key 名称（标识用途）"
          rules={[
            { required: true, message: '请输入名称' },
            { max: 100, message: '名称最多 100 个字符' },
          ]}
        />
        <ProFormSelect
          name="permissions"
          label="权限范围"
          placeholder="请选择允许的权限"
          mode="multiple"
          options={permissionOptions}
          rules={[{ required: true, message: '请选择至少一个权限' }]}
        />
        <ProFormDigit
          name="rateLimit"
          label="速率限制（次/分钟）"
          placeholder="默认 1000"
          min={1}
          max={100000}
          fieldProps={{ precision: 0 }}
        />
        <ProFormDateTimePicker
          name="expiresAt"
          label="过期时间"
          placeholder="留空表示永不过期"
          fieldProps={{
            showTime: true,
            format: 'YYYY-MM-DD HH:mm:ss',
          }}
        />
      </ModalForm>

      {/* 密钥展示弹窗 */}
      <Modal
        title="API Key 创建成功"
        open={keyResultOpen}
        onCancel={() => setKeyResultOpen(false)}
        footer={[
          <Button key="close" type="primary" onClick={() => setKeyResultOpen(false)}>
            我已保存，关闭
          </Button>,
        ]}
        closable={false}
        maskClosable={false}
      >
        {keyResult && (
          <div>
            <Paragraph type="warning" strong>
              请立即复制并妥善保存以下密钥，关闭后将无法再次查看完整密钥。
            </Paragraph>
            <Paragraph>
              <strong>名称：</strong>{keyResult.name}
            </Paragraph>
            <Input.TextArea
              value={keyResult.apiKey}
              readOnly
              autoSize={{ minRows: 2 }}
              style={{ marginBottom: 12, fontFamily: 'monospace' }}
            />
            <Button
              icon={<CopyOutlined />}
              onClick={() => {
                navigator.clipboard.writeText(keyResult.apiKey);
              }}
            >
              复制密钥
            </Button>
          </div>
        )}
      </Modal>
    </PageContainer>
  );
};

export default ApiKeysPage;

/**
 * 登录页面
 *
 * 提供用户名密码登录表单，支持表单验证和错误提示
 * 登录成功后跳转到 dashboard 或 returnUrl 指定的页面
 */

import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, App } from 'antd';
import { UserOutlined, LockOutlined, TrophyOutlined } from '@ant-design/icons';

import { login } from '@/services/auth';
import { useAuthStore } from '@/stores/authStore';

const { Title, Text } = Typography;

/**
 * 登录表单字段
 */
interface LoginFormValues {
  username: string;
  password: string;
}

/**
 * 登录页面组件
 */
const LoginPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [form] = Form.useForm<LoginFormValues>();
  const [loading, setLoading] = useState(false);
  const { message } = App.useApp();

  const { isAuthenticated, setAuth, setError } = useAuthStore();

  // 已登录用户重定向到 dashboard
  useEffect(() => {
    if (isAuthenticated) {
      const returnUrl = searchParams.get('returnUrl');
      navigate(returnUrl ? decodeURIComponent(returnUrl) : '/dashboard', { replace: true });
    }
  }, [isAuthenticated, navigate, searchParams]);

  /**
   * 处理登录表单提交
   */
  const handleSubmit = async (values: LoginFormValues) => {
    const { username, password } = values;

    setLoading(true);
    setError(null);

    try {
      const response = await login(username, password);

      // 保存认证信息到 store
      setAuth(response.user, response.token);

      message.success('登录成功');

      // 跳转到 returnUrl 或默认的 dashboard
      const returnUrl = searchParams.get('returnUrl');
      navigate(returnUrl ? decodeURIComponent(returnUrl) : '/dashboard', { replace: true });
    } catch (error: unknown) {
      // API 层已经处理了 401 等错误，这里处理业务错误
      const errorMessage =
        (error as { message?: string })?.message || '登录失败，请稍后重试';

      // 设置表单错误，显示在密码输入框下方
      form.setFields([
        {
          name: 'password',
          errors: [errorMessage],
        },
      ]);

      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        padding: 24,
      }}
    >
      <Card
        style={{
          width: 400,
          maxWidth: '100%',
          boxShadow: '0 8px 24px rgba(0, 0, 0, 0.15)',
          borderRadius: 8,
        }}
      >
        {/* Logo 和标题 */}
        <div style={{ textAlign: 'center', marginBottom: 32 }}>
          <TrophyOutlined
            style={{
              fontSize: 48,
              color: '#1677ff',
              marginBottom: 16,
            }}
          />
          <Title level={3} style={{ marginBottom: 8 }}>
            徽章管理系统
          </Title>
          <Text type="secondary">管理后台登录</Text>
        </div>

        {/* 登录表单 */}
        <Form<LoginFormValues>
          form={form}
          name="login"
          onFinish={handleSubmit}
          autoComplete="off"
          layout="vertical"
          size="large"
        >
          <Form.Item
            name="username"
            rules={[
              {
                required: true,
                message: '请输入用户名',
              },
            ]}
          >
            <Input
              id="username"
              prefix={<UserOutlined style={{ color: '#bfbfbf' }} />}
              placeholder="用户名"
              autoFocus
            />
          </Form.Item>

          <Form.Item
            name="password"
            rules={[
              {
                required: true,
                message: '请输入密码',
              },
            ]}
          >
            <Input.Password
              id="password"
              prefix={<LockOutlined style={{ color: '#bfbfbf' }} />}
              placeholder="密码"
            />
          </Form.Item>

          <Form.Item style={{ marginBottom: 16 }}>
            <Button
              type="primary"
              htmlType="submit"
              loading={loading}
              block
              style={{ height: 44 }}
            >
              登录
            </Button>
          </Form.Item>
        </Form>

        {/* 底部提示 */}
        <div style={{ textAlign: 'center' }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            徽章管理系统 &copy; 2024
          </Text>
        </div>
      </Card>
    </div>
  );
};

export default LoginPage;

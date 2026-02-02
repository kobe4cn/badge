/**
 * 认证 API Mock 数据
 *
 * 用于开发和测试环境的登录 Mock
 */

import type { AdminUser } from '@/stores/authStore';

/**
 * 测试用户数据
 *
 * 与 e2e/utils/test-data.ts 中的 testUsers 保持一致
 */
export const mockUsers: Record<string, { password: string; user: AdminUser }> = {
  admin: {
    password: 'admin123',
    user: {
      id: '1',
      username: 'admin',
      displayName: '系统管理员',
      role: 'admin',
      avatar: undefined,
    },
  },
  operator: {
    password: 'operator123',
    user: {
      id: '2',
      username: 'operator',
      displayName: '运营人员',
      role: 'operator',
      avatar: undefined,
    },
  },
  viewer: {
    password: 'viewer123',
    user: {
      id: '3',
      username: 'viewer',
      displayName: '访客',
      role: 'viewer',
      avatar: undefined,
    },
  },
};

/**
 * 生成 JWT Mock Token
 *
 * 简单的 base64 编码，仅用于测试
 */
export function generateMockToken(user: AdminUser): string {
  const payload = {
    sub: user.id,
    username: user.username,
    role: user.role,
    exp: Date.now() + 24 * 60 * 60 * 1000, // 24 小时后过期
  };
  return `mock.${btoa(JSON.stringify(payload))}.signature`;
}

/**
 * 验证登录凭据
 */
export function validateCredentials(
  username: string,
  password: string
): { success: true; user: AdminUser; token: string } | { success: false; message: string } {
  const userData = mockUsers[username];

  if (!userData) {
    return { success: false, message: '用户名或密码错误' };
  }

  if (userData.password !== password) {
    return { success: false, message: '用户名或密码错误' };
  }

  return {
    success: true,
    user: userData.user,
    token: generateMockToken(userData.user),
  };
}

/**
 * 认证服务单元测试
 *
 * 验证 auth.ts 中各函数是否正确调用 API 端点，
 * 以及 login / getCurrentUser 中后端 → 前端数据结构的转换逻辑
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@/services/api', () => ({
  get: vi.fn(),
  post: vi.fn(),
}));

import { login, logout, getCurrentUser, refreshToken, validateToken } from '../auth';
import { get, post } from '@/services/api';

const mockedPost = vi.mocked(post);
const mockedGet = vi.mocked(get);

beforeEach(() => {
  vi.clearAllMocks();
});

describe('login', () => {
  // 后端返回扁平结构 { token, user, permissions }，login 需要将 permissions 合并进 user 对象
  const rawResponse = {
    token: 'jwt-token-123',
    user: {
      id: 42,
      username: 'admin',
      displayName: '管理员',
      email: 'admin@test.com',
      avatarUrl: 'https://cdn.test.com/avatar.png',
      status: 'ACTIVE',
    },
    permissions: ['system:user:write', 'badge:manage:read'],
  };

  it('向 /admin/auth/login 发送正确的凭证', async () => {
    mockedPost.mockResolvedValue(rawResponse);

    await login('admin', 'pass123');

    expect(mockedPost).toHaveBeenCalledWith('/admin/auth/login', {
      username: 'admin',
      password: 'pass123',
    });
  });

  it('将后端扁平结构转换为前端嵌套格式', async () => {
    mockedPost.mockResolvedValue(rawResponse);

    const result = await login('admin', 'pass123');

    expect(result.token).toBe('jwt-token-123');
    expect(result.user).toEqual({
      id: '42', // 数值 id 应转为字符串
      username: 'admin',
      displayName: '管理员',
      roles: ['admin'], // system:*:write 权限推导为 admin 角色
      permissions: ['system:user:write', 'badge:manage:read'],
      avatar: 'https://cdn.test.com/avatar.png',
    });
  });

  it('根据权限列表正确推导角色 — operator', async () => {
    // 有 grant:write 但没有 system:write → 推导为 operator
    const operatorResponse = {
      ...rawResponse,
      permissions: ['grant:badge:write', 'badge:manage:read'],
    };
    mockedPost.mockResolvedValue(operatorResponse);

    const result = await login('op', 'pass');
    expect(result.user.roles).toEqual(['operator']);
  });

  it('根据权限列表正确推导角色 — viewer', async () => {
    // 只有 read 权限 → viewer
    const viewerResponse = {
      ...rawResponse,
      permissions: ['badge:manage:read'],
    };
    mockedPost.mockResolvedValue(viewerResponse);

    const result = await login('viewer', 'pass');
    expect(result.user.roles).toEqual(['viewer']);
  });

  it('avatarUrl 为空时 avatar 应为 undefined', async () => {
    const noAvatarResponse = {
      ...rawResponse,
      user: { ...rawResponse.user, avatarUrl: null },
    };
    mockedPost.mockResolvedValue(noAvatarResponse);

    const result = await login('admin', 'pass');
    expect(result.user.avatar).toBeUndefined();
  });
});

describe('logout', () => {
  it('向 /admin/auth/logout 发送 POST 请求', async () => {
    mockedPost.mockResolvedValue(undefined);

    await logout();

    expect(mockedPost).toHaveBeenCalledWith('/admin/auth/logout');
  });
});

describe('getCurrentUser', () => {
  // /me 接口的 roles 带完整对象，需提取 code 字段作为角色列表
  const meRawResponse = {
    user: {
      id: 7,
      username: 'operator1',
      displayName: '运营小王',
      email: null,
      avatarUrl: null,
      status: 'ACTIVE',
    },
    permissions: ['badge:manage:read', 'grant:badge:write'],
    roles: [
      { id: 1, code: 'operator', name: '运营人员', description: '负责徽章发放' },
    ],
  };

  it('向 /admin/auth/me 发送 GET 请求', async () => {
    mockedGet.mockResolvedValue(meRawResponse);

    await getCurrentUser();

    expect(mockedGet).toHaveBeenCalledWith('/admin/auth/me');
  });

  it('将 roles 对象数组转换为 code 字符串数组', async () => {
    mockedGet.mockResolvedValue(meRawResponse);

    const user = await getCurrentUser();

    expect(user.roles).toEqual(['operator']);
  });

  it('正确映射所有用户字段', async () => {
    mockedGet.mockResolvedValue(meRawResponse);

    const user = await getCurrentUser();

    expect(user).toEqual({
      id: '7',
      username: 'operator1',
      displayName: '运营小王',
      roles: ['operator'],
      permissions: ['badge:manage:read', 'grant:badge:write'],
      avatar: undefined, // avatarUrl 为 null → undefined
    });
  });
});

describe('refreshToken', () => {
  it('向 /admin/auth/refresh 发送 POST 请求', async () => {
    mockedPost.mockResolvedValue({ token: 'new-token' });

    const result = await refreshToken();

    expect(mockedPost).toHaveBeenCalledWith('/admin/auth/refresh');
    expect(result.token).toBe('new-token');
  });
});

describe('validateToken', () => {
  it('getCurrentUser 成功时返回 true', async () => {
    mockedGet.mockResolvedValue({
      user: { id: 1, username: 'u', displayName: 'U' },
      permissions: [],
      roles: [],
    });

    const isValid = await validateToken();
    expect(isValid).toBe(true);
  });

  it('getCurrentUser 失败时返回 false（不抛出异常）', async () => {
    mockedGet.mockRejectedValue(new Error('Unauthorized'));

    const isValid = await validateToken();
    expect(isValid).toBe(false);
  });
});

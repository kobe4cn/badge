/**
 * 认证状态管理
 *
 * 使用 Zustand 管理用户认证状态，支持持久化存储和状态恢复
 */

import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

/**
 * 管理员用户信息
 */
export interface AdminUser {
  /** 用户 ID */
  id: string;
  /** 用户名 */
  username: string;
  /** 显示名称 */
  displayName: string;
  /** 角色 */
  role: 'admin' | 'operator' | 'viewer';
  /** 头像 URL */
  avatar?: string;
}

/**
 * 认证状态接口
 */
interface AuthState {
  /** 当前用户信息 */
  user: AdminUser | null;
  /** JWT token */
  token: string | null;
  /** 是否已登录 */
  isAuthenticated: boolean;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
}

/**
 * 认证操作接口
 */
interface AuthActions {
  /** 设置登录状态 */
  setAuth: (user: AdminUser, token: string) => void;
  /** 清除登录状态 */
  clearAuth: () => void;
  /** 设置加载状态 */
  setLoading: (loading: boolean) => void;
  /** 设置错误信息 */
  setError: (error: string | null) => void;
  /** 从 localStorage 恢复认证状态 */
  restoreAuth: () => void;
}

type AuthStore = AuthState & AuthActions;

/**
 * 认证状态 Store
 *
 * 使用 persist 中间件实现状态持久化，token 和用户信息存储在 localStorage
 */
export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,

      setAuth: (user: AdminUser, token: string) => {
        // 同时更新 localStorage 中的 auth_token 以兼容现有的 API 拦截器
        localStorage.setItem('auth_token', token);
        localStorage.setItem('user_info', JSON.stringify(user));

        set({
          user,
          token,
          isAuthenticated: true,
          error: null,
        });
      },

      clearAuth: () => {
        // 清除 localStorage 中的认证信息
        localStorage.removeItem('auth_token');
        localStorage.removeItem('user_info');

        set({
          user: null,
          token: null,
          isAuthenticated: false,
          error: null,
        });
      },

      setLoading: (loading: boolean) => {
        set({ isLoading: loading });
      },

      setError: (error: string | null) => {
        set({ error });
      },

      restoreAuth: () => {
        const token = localStorage.getItem('auth_token');
        const userInfo = localStorage.getItem('user_info');

        if (token && userInfo) {
          try {
            const user = JSON.parse(userInfo) as AdminUser;
            set({
              user,
              token,
              isAuthenticated: true,
            });
          } catch {
            // 解析失败，清除无效数据
            get().clearAuth();
          }
        }
      },
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        user: state.user,
        token: state.token,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);

/**
 * 获取当前认证状态（非 hook 方式）
 *
 * 用于在组件外部（如 API 拦截器）获取认证状态
 */
export const getAuthState = () => useAuthStore.getState();

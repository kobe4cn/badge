/**
 * API 客户端配置
 *
 * 基于 axios 封装，提供统一的请求/响应处理、错误拦截和类型安全的请求方法
 */

import axios, {
  AxiosError,
  AxiosInstance,
  AxiosRequestConfig,
  InternalAxiosRequestConfig,
} from 'axios';
import { message } from 'antd';
import { env } from '@/config';
import type { ApiResponse, PaginatedResponse, PaginationParams } from '@/types';

/**
 * 统一错误类型
 *
 * 用于业务层统一处理错误
 */
export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * 常见错误码映射
 */
const ERROR_MESSAGES: Record<string, string> = {
  NETWORK_ERROR: '网络连接失败，请检查网络',
  TIMEOUT_ERROR: '请求超时，请稍后重试',
  UNAUTHORIZED: '登录已过期，请重新登录',
  FORBIDDEN: '没有权限执行此操作',
  NOT_FOUND: '请求的资源不存在',
  VALIDATION_ERROR: '参数校验失败',
  INTERNAL_ERROR: '服务器内部错误，请稍后重试',
  RATE_LIMITED: '请求过于频繁，请稍后重试',
};

/**
 * 创建 axios 实例
 *
 * 配置基础 URL、超时时间和默认请求头
 */
const apiClient: AxiosInstance = axios.create({
  baseURL: env.apiBaseUrl,
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json',
  },
});

/**
 * 获取认证 token
 *
 * 从 localStorage 读取 token，后续可根据认证方案调整
 */
function getAuthToken(): string | null {
  return localStorage.getItem('auth_token');
}

/**
 * 清除认证信息并跳转登录页
 */
function clearAuthAndRedirect(): void {
  localStorage.removeItem('auth_token');
  localStorage.removeItem('user_info');

  // 避免在登录页重复跳转
  if (!window.location.pathname.includes('/login')) {
    const returnUrl = encodeURIComponent(window.location.pathname + window.location.search);
    window.location.href = `/login?returnUrl=${returnUrl}`;
  }
}

/**
 * 格式化错误消息
 *
 * 将 API 错误转换为用户友好的消息
 */
function formatErrorMessage(error: ApiError): string {
  return ERROR_MESSAGES[error.code] || error.message || '请求失败，请稍后重试';
}

// Token 刷新队列：多个并发请求同时遇到 401 时，只触发一次刷新
let isRefreshing = false;
let failedQueue: Array<{
  resolve: (token: string) => void;
  reject: (error: unknown) => void;
}> = [];

function processQueue(error: unknown, token: string | null) {
  failedQueue.forEach(({ resolve, reject }) => {
    if (error) reject(error);
    else resolve(token!);
  });
  failedQueue = [];
}

/**
 * 请求拦截器
 *
 * 在每个请求发送前自动添加认证 token
 */
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const token = getAuthToken();
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error: AxiosError) => {
    return Promise.reject(error);
  }
);

/**
 * 响应拦截器
 *
 * 统一处理响应数据和错误，对常见错误码进行友好提示
 */
apiClient.interceptors.response.use(
  (response) => {
    // 后端返回 ApiResponse 格式，直接返回 data 字段
    const apiResponse = response.data as ApiResponse<unknown>;
    if (!apiResponse.success && apiResponse.error) {
      // 业务层面的错误，以 Promise.reject 形式抛出
      return Promise.reject(apiResponse.error);
    }
    return response;
  },
  async (error: AxiosError<ApiResponse<unknown>>) => {
    // 构建标准化错误对象
    const apiError: ApiError = {
      code: 'UNKNOWN_ERROR',
      message: '请求失败',
    };

    // 网络错误（无响应）
    if (!error.response) {
      if (error.code === 'ECONNABORTED') {
        apiError.code = 'TIMEOUT_ERROR';
        apiError.message = '请求超时';
      } else {
        apiError.code = 'NETWORK_ERROR';
        apiError.message = '网络连接失败';
      }
      message.error(formatErrorMessage(apiError));
      return Promise.reject(apiError);
    }

    // HTTP 层面的错误处理
    const status = error.response.status;
    const serverError = error.response.data?.error;

    // 合并服务端返回的错误信息
    if (serverError) {
      apiError.code = serverError.code || apiError.code;
      apiError.message = serverError.message || apiError.message;
      apiError.details = serverError.details;
    }

    // 根据 HTTP 状态码进行统一处理
    switch (status) {
      case 401:
        // 登录接口和刷新接口本身返回 401，不重试
        if (error.config?.url?.includes('/auth/login')) {
          apiError.code = serverError?.code || 'INVALID_CREDENTIALS';
          apiError.message = serverError?.message || '用户名或密码错误';
        } else if (error.config?.url?.includes('/auth/refresh')) {
          // 刷新接口失败，token 确实过期了
          apiError.code = 'UNAUTHORIZED';
          clearAuthAndRedirect();
        } else {
          // 业务接口 401：尝试自动刷新 token
          if (isRefreshing) {
            // 已有刷新请求进行中，排队等待结果
            return new Promise((resolve, reject) => {
              failedQueue.push({
                resolve: (newToken: string) => {
                  if (error.config) {
                    error.config.headers.Authorization = `Bearer ${newToken}`;
                    resolve(apiClient(error.config));
                  }
                },
                reject: (err: unknown) => reject(err),
              });
            });
          }

          isRefreshing = true;
          try {
            // 直接调用 apiClient 而非导入 refreshToken，避免循环依赖
            const refreshResponse = await apiClient.post<ApiResponse<{ token: string }>>(
              '/admin/auth/refresh'
            );
            const newToken = refreshResponse.data.data?.token;
            if (!newToken) throw new Error('刷新 token 响应格式异常');

            localStorage.setItem('auth_token', newToken);
            // 动态导入避免循环依赖
            const { useAuthStore } = await import('@/stores/authStore');
            useAuthStore.getState().updateToken(newToken);
            // 通知排队的请求使用新 token
            processQueue(null, newToken);
            // 重试原请求
            if (error.config) {
              error.config.headers.Authorization = `Bearer ${newToken}`;
              return apiClient(error.config);
            }
          } catch (refreshError) {
            processQueue(refreshError, null);
            clearAuthAndRedirect();
            return Promise.reject(refreshError);
          } finally {
            isRefreshing = false;
          }
        }
        break;

      case 403:
        apiError.code = 'FORBIDDEN';
        apiError.message = serverError?.message || '没有权限执行此操作';
        message.error(apiError.message);
        break;

      case 404:
        apiError.code = 'NOT_FOUND';
        // 404 可能是正常的业务场景，不统一提示，由业务层决定
        break;

      case 422:
        apiError.code = 'VALIDATION_ERROR';
        // 参数校验错误，显示具体的校验消息
        if (serverError?.message) {
          message.error(serverError.message);
        }
        break;

      case 429:
        apiError.code = 'RATE_LIMITED';
        apiError.message = '请求过于频繁，请稍后重试';
        message.error(apiError.message);
        break;

      case 500:
      case 502:
      case 503:
      case 504:
        apiError.code = 'INTERNAL_ERROR';
        apiError.message = '服务器内部错误，请稍后重试';
        message.error(apiError.message);
        break;

      default:
        // 其他错误
        if (serverError?.message) {
          message.error(serverError.message);
        }
    }

    return Promise.reject(apiError);
  }
);

// ============ 类型安全的请求方法 ============

/**
 * GET 请求
 */
export async function get<T>(
  url: string,
  params?: Record<string, unknown>,
  config?: AxiosRequestConfig
): Promise<T> {
  const response = await apiClient.get<ApiResponse<T>>(url, { params, ...config });
  return response.data.data as T;
}

/**
 * POST 请求
 */
export async function post<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<T> {
  const response = await apiClient.post<ApiResponse<T>>(url, data, config);
  return response.data.data as T;
}

/**
 * PUT 请求
 */
export async function put<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<T> {
  const response = await apiClient.put<ApiResponse<T>>(url, data, config);
  return response.data.data as T;
}

/**
 * PATCH 请求
 */
export async function patch<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<T> {
  const response = await apiClient.patch<ApiResponse<T>>(url, data, config);
  return response.data.data as T;
}

/**
 * DELETE 请求
 */
export async function del<T = void>(
  url: string,
  config?: AxiosRequestConfig
): Promise<T> {
  const response = await apiClient.delete<ApiResponse<T>>(url, config);
  return response.data.data as T;
}

/**
 * 分页列表请求
 *
 * 封装分页参数处理，返回标准分页响应
 */
export async function getList<T>(
  url: string,
  params?: PaginationParams & Record<string, unknown>
): Promise<PaginatedResponse<T>> {
  const { page = 1, pageSize = 20, ...rest } = params || {};
  const response = await apiClient.get<ApiResponse<PaginatedResponse<T>>>(url, {
    params: { page, pageSize, ...rest },
  });
  return response.data.data as PaginatedResponse<T>;
}

/**
 * 文件上传
 *
 * 使用 multipart/form-data 格式上传文件
 */
export async function upload<T>(
  url: string,
  file: File,
  fieldName = 'file',
  extraData?: Record<string, string>
): Promise<T> {
  const formData = new FormData();
  formData.append(fieldName, file);

  if (extraData) {
    Object.entries(extraData).forEach(([key, value]) => {
      formData.append(key, value);
    });
  }

  const response = await apiClient.post<ApiResponse<T>>(url, formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
  return response.data.data as T;
}

/**
 * 导出原始 axios 实例（用于特殊场景）
 */
export { apiClient };

export default apiClient;

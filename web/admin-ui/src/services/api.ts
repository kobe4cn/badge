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
  (error: AxiosError<ApiResponse<unknown>>) => {
    // HTTP 层面的错误处理
    const status = error.response?.status;
    const apiError = error.response?.data?.error;

    // 根据 HTTP 状态码进行统一提示
    switch (status) {
      case 401:
        message.error('登录已过期，请重新登录');
        // 可在此触发登出逻辑或跳转登录页
        localStorage.removeItem('auth_token');
        window.location.href = '/login';
        break;
      case 403:
        message.error('没有权限执行此操作');
        break;
      case 404:
        // 404 可能是正常的业务场景，不统一提示
        break;
      case 422:
        // 参数校验错误，由业务层处理具体提示
        break;
      case 500:
        message.error('服务器内部错误，请稍后重试');
        break;
      default:
        if (!error.response) {
          message.error('网络连接失败，请检查网络');
        }
    }

    // 返回标准化的错误对象
    return Promise.reject(
      apiError || {
        code: 'NETWORK_ERROR',
        message: error.message || '请求失败',
      }
    );
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

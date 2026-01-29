/**
 * API 通用类型定义
 *
 * 定义后端统一响应格式和分页结构，与 Rust 后端保持一致
 */

/**
 * API 统一响应格式
 *
 * 后端使用 ApiResponse<T> 包装所有返回数据，
 * 成功时 data 有值，失败时 error 有值
 */
export interface ApiResponse<T> {
  /** 是否成功 */
  success: boolean;
  /** 响应数据 */
  data?: T;
  /** 错误信息 */
  error?: ApiError;
}

/**
 * API 错误信息
 *
 * 包含错误码和详细信息，便于前端精确处理不同错误场景
 */
export interface ApiError {
  /** 错误码（如 BADGE_NOT_FOUND, VALIDATION_ERROR） */
  code: string;
  /** 用户可读的错误信息 */
  message: string;
  /** 附加错误详情（如字段校验失败） */
  details?: Record<string, string>;
}

/**
 * 分页响应
 *
 * 后端分页接口统一返回此结构
 */
export interface PaginatedResponse<T> {
  /** 当前页数据 */
  items: T[];
  /** 总条数 */
  total: number;
  /** 当前页码（从 1 开始） */
  page: number;
  /** 每页条数 */
  pageSize: number;
  /** 总页数 */
  totalPages: number;
}

/**
 * 分页请求参数
 */
export interface PaginationParams {
  /** 页码（从 1 开始） */
  page?: number;
  /** 每页条数 */
  pageSize?: number;
}

/**
 * 排序参数
 */
export interface SortParams {
  /** 排序字段 */
  sortField?: string;
  /** 排序方向 */
  sortOrder?: 'ascend' | 'descend';
}

/**
 * 通用列表查询参数
 */
export type ListParams = PaginationParams & SortParams;

/**
 * 通用时间范围
 */
export interface TimeRange {
  /** 开始时间 */
  startTime?: string;
  /** 结束时间 */
  endTime?: string;
}

/**
 * 素材库服务
 *
 * 提供素材的 CRUD 操作 API
 */

import request from './api';
import type { PaginatedResponse } from '@/types';

/**
 * 素材类型
 */
export type AssetType = 'IMAGE' | 'ANIMATION' | 'VIDEO' | 'MODEL_3D';

/**
 * 素材状态
 */
export type AssetStatus = 'active' | 'archived';

/**
 * 素材数据
 */
export interface Asset {
  id: number;
  /** 素材名称 */
  name: string;
  /** 素材类型 */
  assetType: AssetType;
  /** 文件 URL */
  fileUrl: string;
  /** 缩略图 URL */
  thumbnailUrl?: string;
  /** 文件大小（字节） */
  fileSize: number;
  /** 文件格式 */
  fileFormat?: string;
  /** 宽度 */
  width?: number;
  /** 高度 */
  height?: number;
  /** 扩展元数据 */
  metadata?: Record<string, unknown>;
  /** 分类 */
  category?: string;
  /** 标签 */
  tags?: string[];
  /** 状态 */
  status: AssetStatus;
  /** 使用次数 */
  usageCount: number;
  /** 创建人 */
  createdBy?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * 素材查询参数
 */
export interface AssetQueryParams {
  assetType?: AssetType;
  category?: string;
  tag?: string;
  status?: AssetStatus;
  keyword?: string;
  page?: number;
  pageSize?: number;
}

/**
 * 创建素材请求
 */
export interface CreateAssetRequest {
  name: string;
  assetType: AssetType;
  fileUrl: string;
  thumbnailUrl?: string;
  fileSize?: number;
  fileFormat?: string;
  width?: number;
  height?: number;
  metadata?: Record<string, unknown>;
  category?: string;
  tags?: string[];
}

/**
 * 更新素材请求
 */
export interface UpdateAssetRequest {
  name?: string;
  thumbnailUrl?: string;
  metadata?: Record<string, unknown>;
  category?: string;
  tags?: string[];
  status?: AssetStatus;
}

/**
 * 获取素材列表
 */
export async function getAssets(params?: AssetQueryParams): Promise<PaginatedResponse<Asset>> {
  return request.get('/assets', { params });
}

/**
 * 获取单个素材
 */
export async function getAsset(id: number): Promise<Asset> {
  return request.get(`/assets/${id}`);
}

/**
 * 创建素材
 */
export async function createAsset(data: CreateAssetRequest): Promise<Asset> {
  return request.post('/assets', data);
}

/**
 * 更新素材
 */
export async function updateAsset(id: number, data: UpdateAssetRequest): Promise<Asset> {
  return request.put(`/assets/${id}`, data);
}

/**
 * 删除素材
 */
export async function deleteAsset(id: number): Promise<void> {
  return request.delete(`/assets/${id}`);
}

/**
 * 增加素材使用次数
 */
export async function incrementAssetUsage(id: number): Promise<void> {
  return request.post(`/assets/${id}/use`);
}

/**
 * 获取素材分类列表
 */
export async function getAssetCategories(): Promise<string[]> {
  return request.get('/assets/categories');
}

/**
 * 素材类型配置
 */
export const ASSET_TYPES: { value: AssetType; label: string; accept: string }[] = [
  { value: 'IMAGE', label: '图片', accept: '.png,.jpg,.jpeg,.webp,.svg' },
  { value: 'ANIMATION', label: '动画', accept: '.gif,.json,.lottie' },
  { value: 'VIDEO', label: '视频', accept: '.mp4,.webm' },
  { value: 'MODEL_3D', label: '3D 模型', accept: '.glb,.gltf' },
];

/**
 * 获取素材类型配置
 */
export function getAssetTypeConfig(type: AssetType) {
  return ASSET_TYPES.find((t) => t.value === type) || ASSET_TYPES[0];
}

/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

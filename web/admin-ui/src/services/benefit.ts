/**
 * 权益管理 API 服务
 *
 * 封装权益的 CRUD 操作和发放记录查询接口
 */

import { get, post, put, del, getList } from './api';
import type { PaginatedResponse } from '@/types';

/**
 * 权益类型
 */
export type BenefitType = 'POINTS' | 'COUPON' | 'PHYSICAL' | 'VIRTUAL' | 'THIRD_PARTY';

/**
 * 权益状态
 */
export type BenefitStatus = 'DRAFT' | 'ACTIVE' | 'INACTIVE' | 'EXPIRED';

/**
 * 权益实体
 */
export interface Benefit {
  id: number;
  code: string;
  name: string;
  description?: string;
  benefitType: BenefitType;
  externalId?: string;
  externalSystem?: string;
  totalStock?: number;
  remainingStock?: number;
  status: BenefitStatus;
  config?: Record<string, unknown>;
  iconUrl?: string;
  redeemedCount: number;
  createdAt: string;
  updatedAt: string;
}

/**
 * 创建权益请求
 */
export interface CreateBenefitRequest {
  code: string;
  name: string;
  description?: string;
  benefitType: BenefitType;
  externalId?: string;
  externalSystem?: string;
  totalStock?: number;
  config?: Record<string, unknown>;
  iconUrl?: string;
}

/**
 * 更新权益请求
 */
export interface UpdateBenefitRequest {
  name?: string;
  description?: string;
  externalId?: string;
  externalSystem?: string;
  totalStock?: number;
  config?: Record<string, unknown>;
  iconUrl?: string;
  status?: BenefitStatus;
}

/**
 * 权益查询参数
 */
export interface BenefitQueryParams {
  page?: number;
  pageSize?: number;
  benefitType?: BenefitType;
  status?: BenefitStatus;
  keyword?: string;
  [key: string]: unknown;
}

/**
 * 关联徽章请求
 */
export interface LinkBadgeRequest {
  badgeId: number;
  quantity: number;
}

/**
 * 权益发放记录
 */
export interface BenefitGrant {
  id: number;
  grantNo: string;
  userId: string;
  benefitId: number;
  benefitName: string;
  benefitType: BenefitType;
  sourceType: string;
  sourceId?: string;
  quantity: number;
  status: 'PENDING' | 'GRANTED' | 'FAILED' | 'EXPIRED';
  expiresAt?: string;
  grantedAt?: string;
  createdAt: string;
}

/**
 * 发放记录查询参数
 */
export interface BenefitGrantQueryParams {
  page?: number;
  pageSize?: number;
  userId?: string;
  benefitId?: number;
  status?: string;
  startDate?: string;
  endDate?: string;
  [key: string]: unknown;
}

// ==================== API 方法 ====================

/**
 * 获取权益列表
 */
export function listBenefits(
  params?: BenefitQueryParams
): Promise<PaginatedResponse<Benefit>> {
  return getList<Benefit>('/admin/benefits', params);
}

/**
 * 获取权益详情
 */
export function getBenefit(id: number): Promise<Benefit> {
  return get<Benefit>(`/admin/benefits/${id}`);
}

/**
 * 创建权益
 */
export function createBenefit(data: CreateBenefitRequest): Promise<Benefit> {
  return post<Benefit>('/admin/benefits', data);
}

/**
 * 更新权益
 */
export function updateBenefit(id: number, data: UpdateBenefitRequest): Promise<Benefit> {
  return put<Benefit>(`/admin/benefits/${id}`, data);
}

/**
 * 删除权益
 */
export function deleteBenefit(id: number): Promise<void> {
  return del(`/admin/benefits/${id}`);
}

/**
 * 关联徽章
 */
export function linkBadge(benefitId: number, data: LinkBadgeRequest): Promise<void> {
  return post(`/admin/benefits/${benefitId}/link-badge`, data);
}

/**
 * 获取权益发放记录
 */
export function listBenefitGrants(
  params?: BenefitGrantQueryParams
): Promise<PaginatedResponse<BenefitGrant>> {
  return getList<BenefitGrant>('/admin/benefit-grants', params);
}

/**
 * 权益服务对象
 */
export const benefitService = {
  list: listBenefits,
  get: getBenefit,
  create: createBenefit,
  update: updateBenefit,
  delete: deleteBenefit,
  linkBadge,
  listGrants: listBenefitGrants,
};

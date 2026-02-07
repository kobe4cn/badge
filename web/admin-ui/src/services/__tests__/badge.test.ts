/**
 * 徽章服务单元测试
 *
 * 验证 badge.ts 中各 CRUD 和状态管理函数是否正确调用 API 端点，
 * 并确认 badgeService 聚合对象正确映射到独立函数
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@/services/api', () => ({
  get: vi.fn(),
  getList: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  patch: vi.fn(),
  del: vi.fn(),
}));

import {
  getBadges,
  getBadge,
  createBadge,
  updateBadge,
  deleteBadge,
  publishBadge,
  unpublishBadge,
  archiveBadge,
  updateBadgeSortOrder,
  getAllBadges,
  badgeService,
} from '../badge';
import { get, getList, post, put, patch, del } from '@/services/api';

const mockedGet = vi.mocked(get);
const mockedGetList = vi.mocked(getList);
const mockedPost = vi.mocked(post);
const mockedPut = vi.mocked(put);
const mockedPatch = vi.mocked(patch);
const mockedDel = vi.mocked(del);

beforeEach(() => {
  vi.clearAllMocks();
});

describe('getBadges', () => {
  it('使用 getList 请求分页徽章列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 10, name: '测试', status: 'ACTIVE' as const };
    const result = await getBadges(params);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/badges', params);
    expect(result).toBe(mockResponse);
  });
});

describe('getBadge', () => {
  it('通过 ID 获取徽章详情', async () => {
    const mockBadge = { id: 5, name: '签到徽章' };
    mockedGet.mockResolvedValue(mockBadge);

    const result = await getBadge(5);

    expect(mockedGet).toHaveBeenCalledWith('/admin/badges/5');
    expect(result).toBe(mockBadge);
  });
});

describe('createBadge', () => {
  it('向 /admin/badges 发送创建请求', async () => {
    const newBadge = { name: '新徽章', badgeType: 'NORMAL', seriesId: 1 };
    const createdBadge = { id: 10, ...newBadge };
    mockedPost.mockResolvedValue(createdBadge);

    const result = await createBadge(newBadge as any);

    expect(mockedPost).toHaveBeenCalledWith('/admin/badges', newBadge);
    expect(result).toBe(createdBadge);
  });
});

describe('updateBadge', () => {
  it('通过 PUT 更新指定 ID 的徽章', async () => {
    const updateData = { name: '更新后的名称' };
    const updatedBadge = { id: 3, name: '更新后的名称' };
    mockedPut.mockResolvedValue(updatedBadge);

    const result = await updateBadge(3, updateData as any);

    expect(mockedPut).toHaveBeenCalledWith('/admin/badges/3', updateData);
    expect(result).toBe(updatedBadge);
  });
});

describe('deleteBadge', () => {
  it('通过 DELETE 删除指定 ID 的徽章', async () => {
    mockedDel.mockResolvedValue(undefined);

    await deleteBadge(8);

    expect(mockedDel).toHaveBeenCalledWith('/admin/badges/8');
  });
});

describe('publishBadge', () => {
  it('通过 POST 上架徽章', async () => {
    mockedPost.mockResolvedValue(undefined);

    await publishBadge(12);

    expect(mockedPost).toHaveBeenCalledWith('/admin/badges/12/publish');
  });
});

describe('unpublishBadge', () => {
  // 端点是 /offline 而非 /unpublish，需注意命名与路径的差异
  it('通过 POST 下架徽章（端点为 /offline）', async () => {
    mockedPost.mockResolvedValue(undefined);

    await unpublishBadge(12);

    expect(mockedPost).toHaveBeenCalledWith('/admin/badges/12/offline');
  });
});

describe('archiveBadge', () => {
  it('通过 POST 归档徽章', async () => {
    mockedPost.mockResolvedValue(undefined);

    await archiveBadge(15);

    expect(mockedPost).toHaveBeenCalledWith('/admin/badges/15/archive');
  });
});

describe('updateBadgeSortOrder', () => {
  it('通过 PATCH 更新排序值', async () => {
    mockedPatch.mockResolvedValue(undefined);

    await updateBadgeSortOrder(7, 100);

    expect(mockedPatch).toHaveBeenCalledWith('/admin/badges/7/sort', { sortOrder: 100 });
  });
});

describe('getAllBadges', () => {
  // 使用超大 pageSize 模拟「不分页」，用于下拉选择等场景
  it('请求所有徽章时使用 pageSize=1000', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 1000, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const result = await getAllBadges();

    expect(mockedGetList).toHaveBeenCalledWith('/admin/badges', { page: 1, pageSize: 1000 });
    expect(result).toBe(mockResponse);
  });
});

describe('badgeService 聚合对象', () => {
  // 确保面向对象风格的调用方式与独立函数引用同一函数
  it('所有方法正确映射到独立函数', () => {
    expect(badgeService.getList).toBe(getBadges);
    expect(badgeService.get).toBe(getBadge);
    expect(badgeService.create).toBe(createBadge);
    expect(badgeService.update).toBe(updateBadge);
    expect(badgeService.delete).toBe(deleteBadge);
    expect(badgeService.publish).toBe(publishBadge);
    expect(badgeService.unpublish).toBe(unpublishBadge);
    expect(badgeService.archive).toBe(archiveBadge);
    expect(badgeService.updateSortOrder).toBe(updateBadgeSortOrder);
    expect(badgeService.getAll).toBe(getAllBadges);
  });
});

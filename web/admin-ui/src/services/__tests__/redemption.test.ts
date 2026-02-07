/**
 * 兑换服务单元测试
 *
 * 验证 redemption.ts 中兑换规则 CRUD 和兑换记录查询函数的端点调用，
 * 以及 toggleRedemptionRule 复用 PUT 接口仅传递 { enabled } 的设计
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@/services/api', () => ({
  get: vi.fn(),
  getList: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
}));

import {
  listRedemptionRules,
  getRedemptionRule,
  createRedemptionRule,
  updateRedemptionRule,
  deleteRedemptionRule,
  toggleRedemptionRule,
  listRedemptionOrders,
  getRedemptionOrder,
  redemptionService,
} from '../redemption';
import { get, getList, post, put, del } from '@/services/api';

const mockedGet = vi.mocked(get);
const mockedGetList = vi.mocked(getList);
const mockedPost = vi.mocked(post);
const mockedPut = vi.mocked(put);
const mockedDel = vi.mocked(del);

beforeEach(() => {
  vi.clearAllMocks();
});

describe('listRedemptionRules', () => {
  it('使用 getList 请求兑换规则分页列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 10, enabled: true };
    const result = await listRedemptionRules(params);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/redemption/rules', params);
    expect(result).toBe(mockResponse);
  });

  it('无参数时也能正常调用', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    await listRedemptionRules();

    expect(mockedGetList).toHaveBeenCalledWith('/admin/redemption/rules', undefined);
  });
});

describe('getRedemptionRule', () => {
  it('通过 ID 获取兑换规则详情', async () => {
    const mockRule = { id: 1, name: '新手礼包', enabled: true };
    mockedGet.mockResolvedValue(mockRule);

    const result = await getRedemptionRule(1);

    expect(mockedGet).toHaveBeenCalledWith('/admin/redemption/rules/1');
    expect(result).toBe(mockRule);
  });
});

describe('createRedemptionRule', () => {
  it('向 /admin/redemption/rules 发送创建请求', async () => {
    const data = {
      name: '集齐兑换',
      benefitId: 10,
      requiredBadges: [{ badgeId: 1, quantity: 3 }],
    };
    const mockCreated = { id: 5, ...data, enabled: true };
    mockedPost.mockResolvedValue(mockCreated);

    const result = await createRedemptionRule(data);

    expect(mockedPost).toHaveBeenCalledWith('/admin/redemption/rules', data);
    expect(result).toBe(mockCreated);
  });
});

describe('updateRedemptionRule', () => {
  it('通过 PUT 更新指定 ID 的兑换规则', async () => {
    const updateData = { name: '修改后的规则', autoRedeem: true };
    const mockUpdated = { id: 3, name: '修改后的规则', autoRedeem: true };
    mockedPut.mockResolvedValue(mockUpdated);

    const result = await updateRedemptionRule(3, updateData);

    expect(mockedPut).toHaveBeenCalledWith('/admin/redemption/rules/3', updateData);
    expect(result).toBe(mockUpdated);
  });
});

describe('deleteRedemptionRule', () => {
  it('通过 DELETE 删除指定 ID 的兑换规则', async () => {
    mockedDel.mockResolvedValue(undefined);

    await deleteRedemptionRule(8);

    expect(mockedDel).toHaveBeenCalledWith('/admin/redemption/rules/8');
  });
});

describe('toggleRedemptionRule', () => {
  /**
   * toggleRedemptionRule 复用 PUT /rules/:id 接口，
   * 仅传递 { enabled } 字段来切换启用状态，
   * 而不是设计独立的 enable/disable 端点
   */
  it('启用规则时发送 { enabled: true }', async () => {
    const mockRule = { id: 2, name: '规则A', enabled: true };
    mockedPut.mockResolvedValue(mockRule);

    const result = await toggleRedemptionRule(2, true);

    expect(mockedPut).toHaveBeenCalledWith('/admin/redemption/rules/2', { enabled: true });
    expect(result).toBe(mockRule);
  });

  it('禁用规则时发送 { enabled: false }', async () => {
    const mockRule = { id: 2, name: '规则A', enabled: false };
    mockedPut.mockResolvedValue(mockRule);

    const result = await toggleRedemptionRule(2, false);

    expect(mockedPut).toHaveBeenCalledWith('/admin/redemption/rules/2', { enabled: false });
    expect(result).toBe(mockRule);
  });
});

describe('listRedemptionOrders', () => {
  it('使用 getList 请求兑换记录分页列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 20, status: 'COMPLETED' as const };
    const result = await listRedemptionOrders(params);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/redemption/orders', params);
    expect(result).toBe(mockResponse);
  });

  it('支持按用户 ID 和日期范围筛选', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = {
      page: 1,
      pageSize: 20,
      userId: 'user-123',
      startDate: '2025-01-01',
      endDate: '2025-06-30',
    };
    const result = await listRedemptionOrders(params);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/redemption/orders', params);
    expect(result).toBe(mockResponse);
  });
});

describe('getRedemptionOrder', () => {
  it('通过订单号获取兑换记录详情', async () => {
    const mockOrder = { id: 1, orderNo: 'RD20250101001', status: 'COMPLETED' };
    mockedGet.mockResolvedValue(mockOrder);

    const result = await getRedemptionOrder('RD20250101001');

    expect(mockedGet).toHaveBeenCalledWith('/admin/redemption/orders/RD20250101001');
    expect(result).toBe(mockOrder);
  });
});

describe('redemptionService 聚合对象', () => {
  it('所有方法正确映射到独立函数', () => {
    expect(redemptionService.listRules).toBe(listRedemptionRules);
    expect(redemptionService.getRule).toBe(getRedemptionRule);
    expect(redemptionService.createRule).toBe(createRedemptionRule);
    expect(redemptionService.updateRule).toBe(updateRedemptionRule);
    expect(redemptionService.deleteRule).toBe(deleteRedemptionRule);
    expect(redemptionService.toggleRule).toBe(toggleRedemptionRule);
    expect(redemptionService.listOrders).toBe(listRedemptionOrders);
    expect(redemptionService.getOrder).toBe(getRedemptionOrder);
  });
});

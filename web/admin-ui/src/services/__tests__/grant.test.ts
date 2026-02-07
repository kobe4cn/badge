/**
 * 发放服务单元测试
 *
 * 重点验证：
 * 1. createBatchTask 的数据转换逻辑（前端驼峰 → 后端 snake_case + 包裹在 params 中）
 * 2. 各函数调用正确的 HTTP 方法和端点
 * 3. grantService 聚合对象的映射完整性
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@/services/api', () => ({
  get: vi.fn(),
  getList: vi.fn(),
  post: vi.fn(),
  upload: vi.fn(),
  apiClient: { get: vi.fn() },
}));

import {
  manualGrant,
  batchGrant,
  getGrantLogs,
  getGrantLogDetail,
  getGrantRecords,
  searchUsers,
  getBatchTasks,
  getBatchTask,
  createBatchTask,
  cancelBatchTask,
  getBatchTaskFailures,
  previewUserFilter,
  uploadUserCsv,
  grantService,
} from '../grant';
import { get, getList, post, upload } from '@/services/api';

const mockedGet = vi.mocked(get);
const mockedGetList = vi.mocked(getList);
const mockedPost = vi.mocked(post);
const mockedUpload = vi.mocked(upload);

beforeEach(() => {
  vi.clearAllMocks();
});

describe('manualGrant', () => {
  it('向 /admin/grants/manual 发送发放请求', async () => {
    const grantData = { badgeId: 1, userIds: ['u1', 'u2'], quantity: 1, reason: '活动奖励' };
    const mockResult = { totalCount: 2, successCount: 2, failedCount: 0, results: [] };
    mockedPost.mockResolvedValue(mockResult);

    const result = await manualGrant(grantData as any);

    expect(mockedPost).toHaveBeenCalledWith('/admin/grants/manual', grantData);
    expect(result).toBe(mockResult);
  });
});

describe('batchGrant', () => {
  it('向 /admin/grants/batch 发送批量发放请求', async () => {
    const data = { badgeId: 5, fileUrl: 'https://cdn.test.com/users.csv' };
    const mockResult = { taskId: 100, status: 'PENDING', message: '任务已创建' };
    mockedPost.mockResolvedValue(mockResult);

    const result = await batchGrant(data as any);

    expect(mockedPost).toHaveBeenCalledWith('/admin/grants/batch', data);
    expect(result).toBe(mockResult);
  });
});

describe('getGrantLogs', () => {
  it('使用 getList 请求发放日志分页列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 20, badgeId: 3 };
    const result = await getGrantLogs(params as any);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/grants/logs', params);
    expect(result).toBe(mockResponse);
  });
});

describe('getGrantLogDetail', () => {
  it('通过 ID 获取发放日志详情', async () => {
    const mockDetail = { id: 99, badgeId: 1, grantType: 'MANUAL' };
    mockedGet.mockResolvedValue(mockDetail);

    const result = await getGrantLogDetail(99);

    expect(mockedGet).toHaveBeenCalledWith('/admin/grants/logs/99');
    expect(result).toBe(mockDetail);
  });
});

describe('getGrantRecords', () => {
  it('使用 getList 请求发放记录分页列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 20, userId: 'u1' };
    const result = await getGrantRecords(params as any);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/grants/records', params);
    expect(result).toBe(mockResponse);
  });
});

describe('searchUsers', () => {
  it('按关键词搜索用户', async () => {
    const mockUsers = [{ id: 'u1', nickname: '测试用户' }];
    mockedGet.mockResolvedValue(mockUsers);

    const result = await searchUsers('测试');

    expect(mockedGet).toHaveBeenCalledWith('/admin/users/search', { keyword: '测试' });
    expect(result).toBe(mockUsers);
  });
});

describe('getBatchTasks', () => {
  it('使用 getList 请求批量任务列表', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const params = { page: 1, pageSize: 10, status: 'PENDING' };
    const result = await getBatchTasks(params as any);

    expect(mockedGetList).toHaveBeenCalledWith('/admin/tasks', params);
    expect(result).toBe(mockResponse);
  });
});

describe('getBatchTask', () => {
  it('通过 ID 获取批量任务详情', async () => {
    const mockTask = { id: 50, taskType: 'batch_grant', status: 'COMPLETED' };
    mockedGet.mockResolvedValue(mockTask);

    const result = await getBatchTask(50);

    expect(mockedGet).toHaveBeenCalledWith('/admin/tasks/50');
    expect(result).toBe(mockTask);
  });
});

describe('createBatchTask', () => {
  /**
   * 这是发放模块最关键的转换逻辑：
   * 前端使用驼峰命名（badgeId, userIds），
   * 但后端 batch_tasks 表的 params 字段是 JSONB，要求 snake_case（badge_id, user_ids）。
   * 同时需要将所有业务参数包裹在 { task_type, params } 结构中。
   */
  it('将前端驼峰格式转换为后端 { task_type, params } 结构', async () => {
    const frontendData = {
      badgeId: 10,
      quantity: 1,
      reason: '系统补发',
      userIds: ['u1', 'u2', 'u3'],
      userFilter: { level: 'VIP' },
      name: '补发任务-01',
    };
    const mockTask = { id: 200, taskType: 'batch_grant', status: 'PENDING' };
    mockedPost.mockResolvedValue(mockTask);

    await createBatchTask(frontendData as any);

    expect(mockedPost).toHaveBeenCalledWith('/admin/tasks', {
      task_type: 'batch_grant',
      params: {
        badge_id: 10,
        quantity: 1,
        reason: '系统补发',
        user_ids: ['u1', 'u2', 'u3'],
        user_filter: { level: 'VIP' },
        name: '补发任务-01',
      },
    });
  });

  it('可选字段缺失时仍然传递 undefined（由后端忽略）', async () => {
    const minimalData = { badgeId: 1, quantity: 1, reason: '测试' };
    mockedPost.mockResolvedValue({ id: 201 });

    await createBatchTask(minimalData as any);

    const calledPayload = mockedPost.mock.calls[0][1] as any;
    expect(calledPayload.task_type).toBe('batch_grant');
    expect(calledPayload.params.badge_id).toBe(1);
    // 未提供的字段应为 undefined
    expect(calledPayload.params.user_ids).toBeUndefined();
    expect(calledPayload.params.user_filter).toBeUndefined();
  });
});

describe('cancelBatchTask', () => {
  it('向 /admin/tasks/:id/cancel 发送取消请求', async () => {
    mockedPost.mockResolvedValue(undefined);

    await cancelBatchTask(77);

    expect(mockedPost).toHaveBeenCalledWith('/admin/tasks/77/cancel');
  });
});

describe('getBatchTaskFailures', () => {
  it('获取指定任务的失败明细', async () => {
    const mockResponse = { items: [], total: 0, page: 1, pageSize: 20, totalPages: 0 };
    mockedGetList.mockResolvedValue(mockResponse);

    const result = await getBatchTaskFailures(33, { page: 1, pageSize: 50 });

    expect(mockedGetList).toHaveBeenCalledWith('/admin/tasks/33/failures', { page: 1, pageSize: 50 });
    expect(result).toBe(mockResponse);
  });
});

describe('previewUserFilter', () => {
  it('向 /admin/grants/preview-filter 发送筛选条件', async () => {
    const filter = { userLevel: 'VIP', registeredBefore: '2025-01-01' };
    const mockPreview = { totalCount: 500, sampleUsers: [] };
    mockedPost.mockResolvedValue(mockPreview);

    const result = await previewUserFilter(filter as any);

    expect(mockedPost).toHaveBeenCalledWith('/admin/grants/preview-filter', filter);
    expect(result).toBe(mockPreview);
  });
});

describe('uploadUserCsv', () => {
  it('使用 upload 方法上传 CSV 文件', async () => {
    const file = new File(['uid1\nuid2'], 'users.csv', { type: 'text/csv' });
    const mockResult = { totalRows: 2, validRows: 2, invalidRows: 0 };
    mockedUpload.mockResolvedValue(mockResult);

    const result = await uploadUserCsv(file);

    expect(mockedUpload).toHaveBeenCalledWith('/admin/grants/upload-csv', file);
    expect(result).toBe(mockResult);
  });
});

describe('grantService 聚合对象', () => {
  it('所有方法正确映射到独立函数', () => {
    expect(grantService.manualGrant).toBe(manualGrant);
    expect(grantService.batchGrant).toBe(batchGrant);
    expect(grantService.getGrantLogs).toBe(getGrantLogs);
    expect(grantService.getGrantLogDetail).toBe(getGrantLogDetail);
    expect(grantService.getGrantRecords).toBe(getGrantRecords);
    expect(grantService.searchUsers).toBe(searchUsers);
    expect(grantService.getBatchTasks).toBe(getBatchTasks);
    expect(grantService.getBatchTask).toBe(getBatchTask);
    expect(grantService.createBatchTask).toBe(createBatchTask);
    expect(grantService.cancelBatchTask).toBe(cancelBatchTask);
    expect(grantService.getBatchTaskFailures).toBe(getBatchTaskFailures);
    expect(grantService.previewUserFilter).toBe(previewUserFilter);
    expect(grantService.uploadUserCsv).toBe(uploadUserCsv);
  });
});

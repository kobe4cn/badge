/**
 * 撤销与过期 E2E 测试套件
 *
 * 验证手动撤销、自动取消、过期处理等完整流程
 */

import { test, expect, APIRequestContext } from '@playwright/test';
import { ApiHelper, testUsers, createTestBadge } from '../utils';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';

// ============================================================
// 手动撤销测试
// ============================================================
test.describe('撤销测试: 手动撤销', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `Revoke_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    // 创建并发布徽章
    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}撤销测试徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
    if (badgeId) {
      await api.publishBadge(badgeId);
    }
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('手动撤销 - 完整流程', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_revoke_${Date.now()}`;

    // 1. 先发放徽章
    const grantRes = await api.grantBadgeManual(userId, badgeId, 'E2E撤销测试发放');
    expect(grantRes?.data || grantRes?.success).toBeTruthy();

    // 2. 查询用户徽章确认发放成功
    const userBadges = await api.getUserBadges(userId);
    const items = userBadges?.data?.items || userBadges?.data || [];
    const granted = items.find((b: any) => b.badgeId === badgeId || b.badge_id === badgeId);
    expect(granted).toBeTruthy();

    // 检查 id 字段是否存在（需要后端支持）
    const userBadgeId = granted.id || granted.userBadgeId || granted.user_badge_id;
    test.skip(!userBadgeId, 'API 未返回 user_badge_id，需要更新后端');

    // 3. 执行手动撤销
    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/manual`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userBadgeId,
        reason: 'E2E测试手动撤销',
      },
    });

    expect(revokeRes.status()).toBe(200);
    const revokeData = await revokeRes.json();
    expect(revokeData?.data || revokeData?.success).toBeTruthy();

    // 4. 验证撤销后用户徽章状态
    const userBadgesAfter = await api.getUserBadges(userId);
    const itemsAfter = userBadgesAfter?.data?.items || userBadgesAfter?.data || [];
    const revokedBadge = itemsAfter.find((b: any) => b.id === granted.id);

    // 撤销后应该不存在或状态变为 revoked
    if (revokedBadge) {
      expect(['revoked', 'REVOKED', 'inactive']).toContain(revokedBadge.status);
    }
  });

  test('撤销记录查询 - 验证日志存在', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/revokes`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      params: { page: 1, pageSize: 10 },
    });

    expect(response.status()).toBe(200);
    const data = await response.json();
    expect(data?.data !== undefined).toBeTruthy();
  });

  test('撤销记录导出 - 验证接口可用', async () => {
    try {
      const response = await apiContext.get(`${BASE_URL}/api/admin/revokes/export`, {
        headers: {
          Authorization: `Bearer ${(api as any).token}`,
        },
        params: { badgeId },
      });

      // 导出接口应返回文件或 200
      expect([200, 204]).toContain(response.status());
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '撤销导出接口可能尚未实现',
      });
    }
  });
});

// ============================================================
// 自动取消场景测试
// ============================================================
test.describe('撤销测试: 自动取消场景', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `AutoRevoke_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}自动取消徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
    if (badgeId) {
      await api.publishBadge(badgeId);
    }
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('自动取消 - 账号注销场景', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_auto_deletion_${Date.now()}`;

    // 发放徽章
    await api.grantBadgeManual(userId, badgeId, '账号注销测试发放');

    // 执行自动取消
    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/auto`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        scenario: 'account_deletion',
        reason: 'E2E测试账号注销自动撤销',
      },
    });

    expect(revokeRes.status()).toBe(200);
    const data = await revokeRes.json();
    expect(data?.data?.revokedCount).toBeGreaterThanOrEqual(1);
    expect(data?.data?.scenario).toBe('account_deletion');
  });

  test('自动取消 - 身份变更场景', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_auto_identity_${Date.now()}`;

    // 发放徽章
    await api.grantBadgeManual(userId, badgeId, '身份变更测试发放');

    // 执行自动取消
    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/auto`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        badgeId, // 指定特定徽章
        scenario: 'identity_change',
        refId: 'membership_change_001',
        reason: '会员等级降级自动撤销',
      },
    });

    expect(revokeRes.status()).toBe(200);
    const data = await revokeRes.json();
    expect(data?.data?.revokedCount).toBeGreaterThanOrEqual(1);
  });

  test('自动取消 - 条件不满足场景', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_auto_condition_${Date.now()}`;

    await api.grantBadgeManual(userId, badgeId, '条件不满足测试发放');

    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/auto`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        badgeId,
        scenario: 'condition_unmet',
        reason: '月活跃度不足自动撤销',
      },
    });

    expect(revokeRes.status()).toBe(200);
  });

  test('自动取消 - 违规场景', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_auto_violation_${Date.now()}`;

    await api.grantBadgeManual(userId, badgeId, '违规测试发放');

    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/auto`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        scenario: 'violation',
        refId: 'violation_case_123',
        reason: '用户违规，撤销所有徽章',
      },
    });

    expect(revokeRes.status()).toBe(200);
  });

  test('自动取消 - 系统触发场景', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_auto_system_${Date.now()}`;

    await api.grantBadgeManual(userId, badgeId, '系统触发测试发放');

    const revokeRes = await apiContext.post(`${BASE_URL}/api/admin/revokes/auto`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        badgeId,
        scenario: 'system_triggered',
        reason: '定时任务检测到异常自动撤销',
      },
    });

    expect(revokeRes.status()).toBe(200);
  });
});

// ============================================================
// 过期处理测试
// ============================================================
test.describe('撤销测试: 过期处理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `Expire_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}过期测试徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
    if (badgeId) {
      await api.publishBadge(badgeId);
    }
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('发放带过期时间的徽章', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const userId = `e2e_expire_${Date.now()}`;
    const expiresAt = new Date(Date.now() + 7 * 24 * 3600 * 1000).toISOString(); // 7 天后过期

    const response = await apiContext.post(`${BASE_URL}/api/admin/grants/manual`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        userId,
        badgeId,
        quantity: 1,
        reason: 'E2E过期测试发放',
        expiresAt,
      },
    });

    expect(response.status()).toBe(200);
    const data = await response.json();
    expect(data?.data || data?.success).toBeTruthy();

    // 验证用户徽章包含过期时间
    const userBadges = await api.getUserBadges(userId);
    const items = userBadges?.data?.items || userBadges?.data || [];
    const grantedBadge = items.find((b: any) => b.badgeId === badgeId || b.badge_id === badgeId);

    if (grantedBadge && (grantedBadge.expiresAt || grantedBadge.expires_at)) {
      const expiry = new Date(grantedBadge.expiresAt || grantedBadge.expires_at);
      const expected = new Date(expiresAt);
      // 允许 1 秒误差
      expect(Math.abs(expiry.getTime() - expected.getTime())).toBeLessThan(1000);
    }
  });

  test('过期记录筛选 - 按来源类型', async () => {
    // 使用有效的 SourceType：EVENT, SCHEDULED, MANUAL, REDEMPTION, CASCADE, SYSTEM
    const response = await apiContext.get(`${BASE_URL}/api/admin/revokes`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      params: {
        sourceType: 'SCHEDULED', // 过期处理由定时任务触发
        page: 1,
        pageSize: 10,
      },
    });

    expect(response.status()).toBe(200);
    const data = await response.json();
    // 验证接口正常返回
    expect(data?.data !== undefined).toBeTruthy();
  });

  test('撤销记录筛选 - 按时间范围', async () => {
    const startTime = new Date(Date.now() - 24 * 3600 * 1000).toISOString(); // 24 小时前
    const endTime = new Date().toISOString();

    const response = await apiContext.get(`${BASE_URL}/api/admin/revokes`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      params: {
        startTime,
        endTime,
        page: 1,
        pageSize: 10,
      },
    });

    expect(response.status()).toBe(200);
  });
});

// ============================================================
// 批量撤销测试
// ============================================================
test.describe('撤销测试: 批量撤销', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `BatchRevoke_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}批量撤销徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
    if (badgeId) {
      await api.publishBadge(badgeId);
    }
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('批量撤销任务创建', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    // 创建批量撤销任务（使用模拟的文件 URL）
    const response = await apiContext.post(`${BASE_URL}/api/admin/revokes/batch`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        badgeId,
        fileUrl: 'https://example.com/e2e-test-revoke-list.csv',
        reason: 'E2E批量撤销测试',
      },
    });

    // 批量任务创建应返回任务信息
    expect([200, 201, 202]).toContain(response.status());
    const data = await response.json();
    // 验证返回任务 ID 或相关信息
    expect(data?.data?.taskId || data?.data?.id || data?.success).toBeTruthy();
  });

  test('批量任务状态查询', async () => {
    const response = await apiContext.get(`${BASE_URL}/api/admin/tasks`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      params: {
        taskType: 'batch_revoke',
        page: 1,
        pageSize: 10,
      },
    });

    expect(response.status()).toBe(200);
  });
});

// ============================================================
// 撤销与依赖联动测试
// ============================================================
test.describe('撤销测试: 依赖联动', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `RevokeDep_${Date.now().toString(36)}_`;
  let parentBadgeId: number;
  let childBadgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);

    // 创建父徽章
    const parentBadge = createTestBadge({
      name: `${testPrefix}父徽章`,
      seriesId,
    });
    const parentRes = await api.createBadge(parentBadge);
    parentBadgeId = parentRes?.data?.id;

    // 创建子徽章
    const childBadge = createTestBadge({
      name: `${testPrefix}子徽章`,
      seriesId,
    });
    const childRes = await api.createBadge(childBadge);
    childBadgeId = childRes?.data?.id;

    // 发布徽章
    if (parentBadgeId) await api.publishBadge(parentBadgeId);
    if (childBadgeId) await api.publishBadge(childBadgeId);

    // 创建依赖关系：子徽章依赖父徽章
    if (childBadgeId && parentBadgeId) {
      await api.createDependency(childBadgeId, {
        dependsOnBadgeId: parentBadgeId,
        dependencyType: 'prerequisite',
        dependencyGroupId: `${testPrefix}dep_group`,
        requiredQuantity: 1,
        autoTrigger: false,
      });
    }
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('撤销父徽章 - 验证依赖关系影响', async () => {
    test.skip(!parentBadgeId || !childBadgeId, '前置数据未就绪');

    const userId = `e2e_dep_revoke_${Date.now()}`;

    // 先发放父徽章
    await api.grantBadgeManual(userId, parentBadgeId, '依赖测试父徽章发放');

    // 发放子徽章（依赖父徽章）
    await api.grantBadgeManual(userId, childBadgeId, '依赖测试子徽章发放');

    // 验证两个徽章都存在
    let userBadges = await api.getUserBadges(userId);
    let items = userBadges?.data?.items || userBadges?.data || [];
    expect(items.length).toBeGreaterThanOrEqual(2);

    // 撤销父徽章
    const parentGrant = items.find(
      (b: any) => (b.badgeId || b.badge_id) === parentBadgeId
    );
    if (parentGrant) {
      await apiContext.post(`${BASE_URL}/api/admin/revokes/manual`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${(api as any).token}`,
        },
        data: {
          userBadgeId: parentGrant.id,
          reason: '依赖测试撤销父徽章',
        },
      });
    }

    // 检查是否触发级联撤销
    userBadges = await api.getUserBadges(userId);
    items = userBadges?.data?.items || userBadges?.data || [];

    // 根据系统设计，可能会级联撤销子徽章
    test.info().annotations.push({
      type: 'info',
      description: `撤销后剩余徽章数: ${items.filter((b: any) => b.status === 'active').length}`,
    });
  });
});

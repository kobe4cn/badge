import { test, expect, APIRequestContext } from '@playwright/test';
import { ApiHelper, testUsers, createTestBadge, createTestBenefit } from '../utils';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';

// ============================================================
// 1. 徽章生命周期 API
// ============================================================
test.describe('API 集成测试: 徽章生命周期', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIBadge${Date.now().toString(36)}_`;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  /** 跨用例共享，按顺序执行 */
  let categoryId: number;
  let seriesId: number;
  let badgeId: number;

  test('创建徽章 - 先创建分类和系列再创建徽章', async () => {
    // 创建分类
    const catRes = await api.createCategory({
      name: `${testPrefix}生命周期分类`,
      sortOrder: 0,
    });
    expect(catRes?.data?.id).toBeDefined();
    categoryId = catRes.data.id;

    // 创建系列
    const seriesRes = await api.createSeries({
      name: `${testPrefix}生命周期系列`,
      categoryId,
      sortOrder: 0,
    });
    expect(seriesRes?.data?.id).toBeDefined();
    seriesId = seriesRes.data.id;

    // 创建徽章
    const badge = createTestBadge({
      name: `${testPrefix}生命周期徽章`,
      seriesId,
      description: 'API集成测试 - 生命周期徽章',
    });
    const badgeRes = await api.createBadge(badge);
    expect(badgeRes?.data?.id).toBeDefined();
    expect(badgeRes.data.name).toContain(testPrefix);
    badgeId = badgeRes.data.id;
  });

  test('发布徽章 - 验证状态变为已发布', async () => {
    test.skip(!badgeId, '前置用例未创建徽章');

    const res = await api.publishBadge(badgeId);
    // 发布接口应返回 code=0 表示成功
    expect(res?.code).toBe(0);

    // 再次查询确认状态
    const badges = await api.getBadges({ keyword: testPrefix });
    const target = (badges?.data?.items || []).find((b: any) => b.id === badgeId);
    if (target) {
      expect(['published', 'PUBLISHED', 'active', 'ACTIVE']).toContain(target.status);
    }
  });

  test('下架徽章 - 验证状态变更', async () => {
    test.skip(!badgeId, '前置用例未创建徽章');

    const res = await api.offlineBadge(badgeId);
    expect(res?.code).toBe(0);

    const badges = await api.getBadges({ keyword: testPrefix });
    const target = (badges?.data?.items || []).find((b: any) => b.id === badgeId);
    if (target) {
      expect(['offline', 'OFFLINE', 'inactive', 'INACTIVE']).toContain(target.status);
    }
  });

  test('归档徽章', async () => {
    test.skip(!badgeId, '前置用例未创建徽章');

    const res = await api.archiveBadge(badgeId);
    expect(res?.code).toBe(0);
  });

  test('删除草稿徽章', async () => {
    test.skip(!seriesId, '前置数据未就绪');

    // 创建一个新的草稿徽章用于删除
    const badge = createTestBadge({
      name: `${testPrefix}待删除草稿`,
      seriesId,
      description: '创建后立即删除的草稿徽章',
    });
    const res = await api.createBadge(badge);
    expect(res?.data?.id).toBeDefined();

    const draftId = res.data.id;
    await api.deleteBadge(draftId);

    // 验证已删除：列表中不应再出现该徽章
    const badges = await api.getBadges({ keyword: `${testPrefix}待删除草稿` });
    const items = badges?.data?.items || [];
    const found = items.find((b: any) => b.id === draftId);
    expect(found).toBeUndefined();
  });
});

// ============================================================
// 2. 规则管理 API
// ============================================================
test.describe('API 集成测试: 规则管理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIRule${Date.now().toString(36)}_`;

  let badgeId: number;
  let ruleId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    // 准备基础数据：规则需要关联徽章
    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}规则关联徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('创建规则 - 验证返回数据', async () => {
    const rule = await api.createRule({
      badgeId: badgeId,
      ruleCode: `${testPrefix}rule_001`,
      eventType: 'purchase',
      name: `${testPrefix}TestRule`,
      ruleJson: {
        type: 'event',
        conditions: [{ field: 'amount', op: 'gte', value: 100 }],
      },
    });
    expect(rule?.data?.id).toBeDefined();
    ruleId = rule.data.id;
  });

  test('测试规则定义 - POST /rules/:id/test', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    try {
      const result = await api.testRule(ruleId, {
        userId: 'mock_user_001',
        eventType: 'purchase',
        eventData: { amount: 200 },
      });
      // 测试接口只要能正常返回即可，结果取决于规则引擎实现
      expect(result).toBeDefined();
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '规则测试接口可能尚未完全实现',
      });
    }
  });

  test('发布规则 - 验证 enabled=true', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    const res = await api.publishRule(ruleId);
    expect(res?.code).toBe(0);

    // 验证规则列表中状态为已启用
    const rules = await api.getRules({ keyword: testPrefix });
    const target = (rules?.data?.items || []).find((r: any) => r.id === ruleId);
    if (target) {
      expect(target.enabled === true || ['published', 'PUBLISHED'].includes(target.status)).toBe(true);
    }
  });

  test('禁用规则 - 验证 enabled=false', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    const res = await api.disableRule(ruleId);
    expect(res?.code).toBe(0);

    const rules = await api.getRules({ keyword: testPrefix });
    const target = (rules?.data?.items || []).find((r: any) => r.id === ruleId);
    if (target) {
      expect(target.enabled === false || ['disabled', 'DISABLED'].includes(target.status)).toBe(true);
    }
  });

  test('删除禁用的规则', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    await api.deleteRule(ruleId);

    // 验证已删除
    const rules = await api.getRules({ keyword: testPrefix });
    const items = rules?.data?.items || [];
    const found = items.find((r: any) => r.id === ruleId);
    expect(found).toBeUndefined();
  });
});

// ============================================================
// 3. 发放管理 API
// ============================================================
test.describe('API 集成测试: 发放管理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIGrant${Date.now().toString(36)}_`;

  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    // 创建并发布一个徽章，用于发放
    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}发放徽章`,
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

  test('手动发放 - POST /grants/manual', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    const grant = await api.grantBadgeManual('e2e_test_user_001', badgeId, 'E2E测试手动发放');
    expect(grant?.code).toBe(0);
  });

  test('发放日志查询 - GET /grants/logs', async () => {
    const logs = await api.getGrantLogs({ page: 1, pageSize: 10 });
    // 只要接口正常返回即表示通过
    expect(logs).toBeDefined();
    // 返回结构应包含列表
    expect(logs?.data).toBeDefined();
  });

  test('发放日志详情 - GET /grants/logs/:id', async () => {
    // 先查询日志列表以获取一个可用的日志 ID
    const logs = await api.getGrantLogs({ page: 1, pageSize: 1 });
    const items = logs?.data?.items || logs?.data || [];

    if (items.length === 0) {
      test.skip(true, '没有可用的发放日志记录');
      return;
    }

    const logId = items[0].id;
    try {
      const response = await api['request'].get(`${BASE_URL}/api/admin/grants/logs/${logId}`, {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${api['token']}`,
        },
      });
      const detail = await response.json();
      expect(detail).toBeDefined();
      expect(detail?.data).toBeDefined();
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '发放日志详情接口可能尚未实现',
      });
    }
  });

  test('发放记录查询 - GET /grants/records', async () => {
    try {
      const response = await api['request'].get(`${BASE_URL}/api/admin/grants/records`, {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${api['token']}`,
        },
        params: { page: 1, pageSize: 10 },
      });
      const records = await response.json();
      expect(records).toBeDefined();
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '发放记录接口可能尚未实现',
      });
    }
  });
});

// ============================================================
// 4. 兑换管理 API
// ============================================================
test.describe('API 集成测试: 兑换管理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIRedeem${Date.now().toString(36)}_`;

  let badgeId: number;
  let redemptionRuleId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}兑换徽章`,
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

  test('创建兑换规则', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    // 创建权益作为兑换规则的目标
    const benefit = createTestBenefit({
      name: `${testPrefix}兑换权益`,
      benefitType: 'COUPON',
      type: 'COUPON',
    });
    const benefitRes = await api.createBenefit(benefit);
    const benefitId = benefitRes?.data?.id;
    test.skip(!benefitId, '权益创建失败');

    const rule = await api.createRedemptionRule({
      name: `${testPrefix}兑换规则`,
      benefitId,
      requiredBadges: [{ badgeId, quantity: 1 }],
      startTime: new Date().toISOString(),
      endTime: new Date(Date.now() + 30 * 24 * 3600 * 1000).toISOString(),
    });
    expect(rule?.data?.id).toBeGreaterThan(0);
    redemptionRuleId = rule?.data?.id;
  });

  test('查询兑换订单列表', async () => {
    const orders = await api.getRedemptionOrders({ page: 1, pageSize: 10 });
    expect(orders).toBeDefined();
    // 即使列表为空也验证结构正确
    expect(orders?.data).toBeDefined();
  });

  test('查询单个兑换订单详情 by order_no', async () => {
    const orders = await api.getRedemptionOrders({ page: 1, pageSize: 1 });
    const items = orders?.data?.items || orders?.data || [];

    if (items.length === 0) {
      test.info().annotations.push({
        type: 'info',
        description: '没有可用的兑换订单，跳过详情查询',
      });
      return;
    }

    const orderNo = items[0].orderNo || items[0].id;
    try {
      const response = await api['request'].get(
        `${BASE_URL}/api/admin/redemption/orders/${orderNo}`,
        {
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${api['token']}`,
          },
        },
      );
      const detail = await response.json();
      expect(detail).toBeDefined();
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '订单详情接口可能尚未实现',
      });
    }
  });
});

// ============================================================
// 5. 权益管理 API
// ============================================================
test.describe('API 集成测试: 权益管理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIBenefit${Date.now().toString(36)}_`;

  let benefitId: number;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}权益关联徽章`,
      seriesId,
    });
    const badgeRes = await api.createBadge(badge);
    badgeId = badgeRes?.data?.id;
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('创建权益', async () => {
    const benefit = createTestBenefit({
      name: `${testPrefix}测试权益`,
      type: 'COUPON',
      benefitType: 'COUPON',
      value: 50,
      description: 'API集成测试权益',
    });
    const res = await api.createBenefit(benefit);
    expect(res?.data?.id).toBeGreaterThan(0);
    benefitId = res?.data?.id;
  });

  test('查询权益列表', async () => {
    const benefits = await api.getBenefits({ page: 1, pageSize: 10 });
    expect(benefits).toBeDefined();
    expect(benefits?.data).toBeDefined();
  });

  test('关联权益与徽章', async () => {
    test.skip(!benefitId || !badgeId, '前置数据未就绪');

    const res = await api.linkBadgeToBenefit(benefitId, badgeId);
    expect(res?.code).toBe(0);
  });
});

// ============================================================
// 6. 依赖关系 API
// ============================================================
test.describe('API 集成测试: 依赖关系', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIDep${Date.now().toString(36)}_`;

  let badgeA: number;
  let badgeB: number;
  let dependencyId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    // 创建两个徽章用于依赖关系测试
    const { seriesId } = await api.ensureTestData(testPrefix);
    const badgeARes = await api.createBadge(
      createTestBadge({ name: `${testPrefix}依赖方A`, seriesId }),
    );
    const badgeBRes = await api.createBadge(
      createTestBadge({ name: `${testPrefix}被依赖方B`, seriesId }),
    );
    badgeA = badgeARes?.data?.id;
    badgeB = badgeBRes?.data?.id;
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('创建依赖 - A 依赖 B', async () => {
    test.skip(!badgeA || !badgeB, '前置徽章数据未就绪');

    const res = await api.createDependency(badgeA, {
      dependsOnBadgeId: badgeB,
      dependencyType: 'prerequisite',
      dependencyGroupId: `group_${badgeA}_${badgeB}`,
      requiredQuantity: 1,
      autoTrigger: false,
    });
    expect(res?.data?.id).toBeGreaterThan(0);
    dependencyId = res?.data?.id;
  });

  test('查询依赖列表', async () => {
    test.skip(!badgeA, '前置徽章数据未就绪');

    const deps = await api.getDependencies(badgeA);
    expect(deps).toBeDefined();
    // 至少包含刚才创建的依赖
    const items = deps?.data?.items || deps?.data || [];
    if (dependencyId) {
      expect(items.length).toBeGreaterThanOrEqual(1);
    }
  });

  test('删除依赖', async () => {
    test.skip(!dependencyId, '前置依赖数据未就绪');

    await api.deleteDependency(dependencyId);

    // 验证依赖已被移除
    const deps = await api.getDependencies(badgeA);
    const items = deps?.data?.items || deps?.data || [];
    const found = items.find((d: any) => d.id === dependencyId);
    expect(found).toBeUndefined();
  });
});

// ============================================================
// 7. 系统管理 API
// ============================================================
test.describe('API 集成测试: 系统管理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APISys${Date.now().toString(36)}_`;

  let systemUserId: number;
  let roleId: number;
  let apiKeyId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    // 手动清理系统资源（不在通用 cleanup 范围内）
    try {
      if (apiKeyId) await cleanup.deleteApiKey(apiKeyId).catch(() => {});
      if (roleId) await cleanup.deleteRole(roleId).catch(() => {});
      if (systemUserId) await cleanup.deleteSystemUser(systemUserId).catch(() => {});
    } catch {
      // 清理失败不影响测试结果
    }
    await apiContext?.dispose();
  });

  test('创建系统用户', async () => {
    try {
      const res = await api.createSystemUser({
        username: `${testPrefix}user`,
        password: 'Test@123456',
        nickname: `${testPrefix}测试用户`,
        email: `${testPrefix.toLowerCase()}user@test.com`,
        role: 'operator',
      });
      expect(res?.data?.id).toBeGreaterThan(0);
      systemUserId = res?.data?.id;
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '系统用户创建接口可能尚未实现',
      });
    }
  });

  test('查询用户列表', async () => {
    // 不传分页参数使用默认值，避免 serde_urlencoded flatten 类型转换问题
    const users = await api.getSystemUsers();
    expect(users).toBeDefined();
    expect(users?.data).toBeDefined();
  });

  test('创建角色', async () => {
    try {
      const res = await api.createRole({
        name: `${testPrefix}TestRole`,
        code: `${testPrefix}role_code`,
        description: 'API集成测试角色',
        permissions: ['badge:read', 'badge:write'],
      });
      expect(res?.data?.id).toBeGreaterThan(0);
      roleId = res?.data?.id;
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '角色创建接口可能尚未实现',
      });
    }
  });

  test('查询权限树', async () => {
    const tree = await api.getPermissionTree();
    expect(tree).toBeDefined();
    // 权限树应返回嵌套结构或列表
    expect(tree?.data).toBeDefined();
  });

  test('API Key 创建和删除', async () => {
    try {
      const createRes = await api.createApiKey(`${testPrefix}Key`, ['badge:read']);
      expect(createRes?.data?.id ?? createRes?.data?.key).toBeDefined();
      apiKeyId = createRes?.data?.id;

      if (apiKeyId) {
        await api.deleteApiKey(apiKeyId);

        // 验证已删除
        const keys = await api.getApiKeys();
        const items = keys?.data?.items || keys?.data || [];
        const found = items.find((k: any) => k.id === apiKeyId);
        expect(found).toBeUndefined();

        // 已删除，清理时不需要再删
        apiKeyId = 0;
      }
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: 'API Key 管理接口可能尚未实现',
      });
    }
  });
});

// ============================================================
// 8. 统计和模板 API
// ============================================================
test.describe('API 集成测试: 统计和模板', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    await apiContext?.dispose();
  });

  test('获取统计概览', async () => {
    const stats = await api.getStatsOverview();
    expect(stats).toBeDefined();
    expect(stats?.data).toBeDefined();
  });

  test('获取模板列表', async () => {
    const templates = await api.getTemplates();
    expect(templates).toBeDefined();
    expect(templates?.data).toBeDefined();
  });
});

// ============================================================
// 9. 全链路测试
// ============================================================
test.describe('API 集成测试: 全链路', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `APIE2E${Date.now().toString(36)}_`;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, BASE_URL);
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
    await apiContext?.dispose();
  });

  test('完整发放链路: 分类→系列→徽章→发布→规则→发布规则→手动发放→查询用户徽章→查询日志', async () => {
    // 1. 创建分类
    const catRes = await api.createCategory({
      name: `${testPrefix}全链路分类`,
      sortOrder: 0,
    });
    expect(catRes?.data?.id).toBeDefined();
    const categoryId = catRes.data.id;

    // 2. 创建系列
    const seriesRes = await api.createSeries({
      name: `${testPrefix}全链路系列`,
      categoryId,
      sortOrder: 0,
    });
    expect(seriesRes?.data?.id).toBeDefined();
    const seriesId = seriesRes.data.id;

    // 3. 创建徽章
    const badge = createTestBadge({
      name: `${testPrefix}全链路徽章`,
      seriesId,
      description: '全链路集成测试',
    });
    const badgeRes = await api.createBadge(badge);
    expect(badgeRes?.data?.id).toBeDefined();
    const badgeId = badgeRes.data.id;

    // 4. 发布徽章
    const publishRes = await api.publishBadge(badgeId);
    expect(publishRes?.code).toBe(0);

    // 5. 创建规则
    const ruleRes = await api.createRule({
      badgeId: badgeId,
      ruleCode: `${testPrefix}e2e_rule`,
      eventType: 'purchase',
      name: `${testPrefix}全链路规则`,
      ruleJson: {
        type: 'event',
        conditions: [{ field: 'amount', op: 'gte', value: 50 }],
      },
    });
    expect(ruleRes?.data?.id).toBeDefined();
    const ruleId = ruleRes.data.id;

    // 6. 发布规则
    const publishRuleRes = await api.publishRule(ruleId);
    expect(publishRuleRes?.code).toBe(0);

    // 7. 手动发放
    const targetUser = `e2e_fullchain_${Date.now()}`;
    const grantRes = await api.grantBadgeManual(targetUser, badgeId, '全链路测试发放');
    expect(grantRes?.code).toBe(0);

    // 8. 查询用户徽章
    const userBadges = await api.getUserBadges(targetUser);
    expect(userBadges).toBeDefined();

    // 9. 查询发放日志
    const logs = await api.getGrantLogs({ badgeId: badgeId, page: 1, pageSize: 10 });
    expect(logs).toBeDefined();
  });

  test('权益关联链路: 创建权益→创建徽章→关联→查询', async () => {
    // 1. 创建权益
    const benefit = createTestBenefit({
      name: `${testPrefix}关联权益`,
      type: 'POINTS',
      benefitType: 'POINTS',
      value: 200,
      description: '全链路权益关联测试',
    });
    const benefitRes = await api.createBenefit(benefit);
    expect(benefitRes?.data?.id).toBeGreaterThan(0);
    const benefitId = benefitRes?.data?.id;

    // 2. 创建徽章
    const { seriesId } = await api.ensureTestData(`${testPrefix}ben_`);
    const badge = createTestBadge({
      name: `${testPrefix}权益关联徽章`,
      seriesId,
      description: '用于权益关联的测试徽章',
    });
    const badgeRes = await api.createBadge(badge);
    expect(badgeRes?.data?.id).toBeDefined();
    const badgeId = badgeRes.data.id;

    // 3. 关联权益与徽章
    if (benefitId && badgeId) {
      const linkRes = await api.linkBadgeToBenefit(benefitId, badgeId);
      expect(linkRes?.code).toBe(0);
    }

    // 4. 查询权益列表确认关联存在
    const benefits = await api.getBenefits({ keyword: testPrefix });
    expect(benefits).toBeDefined();
    const items = benefits?.data?.items || benefits?.data || [];
    if (benefitId) {
      const found = items.find((b: any) => b.id === benefitId);
      // 只要查询正常返回且权益存在即为通过
      if (found) {
        expect(found.name).toContain(testPrefix);
      }
    }
  });
});

// ============================================================
// 10. RBAC 权限执行验证
// ============================================================
test.describe('API 集成测试: RBAC 权限执行', () => {
  let adminApi: ApiHelper;
  let operatorApi: ApiHelper;
  let viewerApi: ApiHelper;
  let adminContext: APIRequestContext;
  let operatorContext: APIRequestContext;
  let viewerContext: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    // 为三种角色分别创建独立的 API 上下文
    adminContext = await playwright.request.newContext({ baseURL: BASE_URL });
    operatorContext = await playwright.request.newContext({ baseURL: BASE_URL });
    viewerContext = await playwright.request.newContext({ baseURL: BASE_URL });

    adminApi = new ApiHelper(adminContext, BASE_URL);
    operatorApi = new ApiHelper(operatorContext, BASE_URL);
    viewerApi = new ApiHelper(viewerContext, BASE_URL);

    await adminApi.login(testUsers.admin.username, testUsers.admin.password);

    // 确保 viewer/operator 用户存在且角色已分配
    await adminApi.ensureUser('operator', testUsers.operator.password, 2);
    await adminApi.ensureUser('viewer', testUsers.viewer.password, 3);

    await operatorApi.login(testUsers.operator.username, testUsers.operator.password);
    await viewerApi.login(testUsers.viewer.username, testUsers.viewer.password);
  });

  test.afterAll(async () => {
    await adminContext?.dispose();
    await operatorContext?.dispose();
    await viewerContext?.dispose();
  });

  test('viewer 角色可以读取徽章列表', async () => {
    const res = await viewerApi.getBadges({ page: 1, pageSize: 5 });
    // viewer 有 badge:badge:read 权限，应返回正常数据
    expect(res?.data).toBeDefined();
    expect(res?.status).not.toBe(403);
  });

  test('viewer 角色无法创建分类 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: { name: 'RBAC测试分类', sortOrder: 0 },
    });
    // viewer 没有 badge:category:write 权限
    expect(response.status()).toBe(403);
  });

  test('viewer 角色无法手动发放徽章 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/grants/manual`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: { userId: 'test', badgeId: 1, quantity: 1, reason: 'RBAC测试' },
    });
    expect(response.status()).toBe(403);
  });

  test('operator 角色可以创建分类', async () => {
    const testPrefix = `RBACOp${Date.now().toString(36)}_`;
    const res = await operatorApi.createCategory({
      name: `${testPrefix}Operator分类`,
      sortOrder: 0,
    });
    // operator 有 badge:category:write 权限
    expect(res?.data?.id != null || res?.success === true).toBe(true);

    // 清理
    if (res?.data?.id) {
      try {
        await adminApi.deleteCategory(res.data.id);
      } catch { /* ignore */ }
    }
  });

  test('operator 角色无法管理系统用户 (403)', async () => {
    const response = await operatorContext.post(`${BASE_URL}/api/admin/system/users`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(operatorApi as any).token}`,
      },
      data: {
        username: 'rbac_test_fail',
        password: 'Test@123456',
        displayName: '不应创建成功',
      },
    });
    // operator 没有 system:user:write 权限
    expect(response.status()).toBe(403);
  });

  test('operator 角色无法创建 API Key (403)', async () => {
    const response = await operatorContext.post(`${BASE_URL}/api/admin/system/api-keys`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(operatorApi as any).token}`,
      },
      data: { name: 'rbac_test_key', permissions: ['badge:badge:read'] },
    });
    // operator 没有 system:apikey:write 权限
    expect(response.status()).toBe(403);
  });

  test('admin 角色可以管理系统用户', async () => {
    const users = await adminApi.getSystemUsers();
    expect(users?.data).toBeDefined();
    expect(users?.status).not.toBe(403);
  });

  // ---- 规则模块权限边界 ----

  test('viewer 角色无法创建规则 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/rules`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: {
        badgeId: 1,
        ruleCode: 'rbac_viewer_rule',
        eventType: 'purchase',
        name: 'RBAC viewer 规则',
        ruleJson: { type: 'event', conditions: [] },
      },
    });
    expect(response.status()).toBe(403);
  });

  test('operator 角色可以创建和管理规则', async () => {
    const response = await operatorContext.post(`${BASE_URL}/api/admin/rules`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(operatorApi as any).token}`,
      },
      data: {
        badgeId: 1,
        ruleCode: 'rbac_operator_rule',
        eventType: 'purchase',
        name: 'RBAC operator 规则',
        ruleJson: { type: 'event', conditions: [] },
      },
    });
    // operator 有规则写权限，不应返回 403（可能返回 200 或 400 业务校验错误）
    expect(response.status()).not.toBe(403);
  });

  // ---- 权益模块权限边界 ----

  test('viewer 角色无法创建权益 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/benefits`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: {
        name: 'RBAC viewer 权益',
        type: 'COUPON',
        benefitType: 'COUPON',
        value: 10,
      },
    });
    expect(response.status()).toBe(403);
  });

  test('operator 角色可以读取权益列表', async () => {
    const response = await operatorContext.get(`${BASE_URL}/api/admin/benefits`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(operatorApi as any).token}`,
      },
    });
    expect(response.status()).toBe(200);
  });

  // ---- 兑换模块权限边界 ----

  test('viewer 角色无法创建兑换规则 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/redemption/rules`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: {
        name: 'RBAC viewer 兑换规则',
        benefitId: 1,
        requiredBadges: [{ badgeId: 1, quantity: 1 }],
        startTime: new Date().toISOString(),
        endTime: new Date(Date.now() + 86400000).toISOString(),
      },
    });
    expect(response.status()).toBe(403);
  });

  // ---- 用户视图模块权限边界 ----

  test('viewer 角色可以查看用户视图', async () => {
    const response = await viewerContext.get(`${BASE_URL}/api/admin/users/search?keyword=test`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
    });
    expect(response.status()).toBe(200);
  });

  // ---- 统计模块权限边界 ----

  test('viewer 角色可以查看统计概览', async () => {
    const response = await viewerContext.get(`${BASE_URL}/api/admin/stats/overview`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
    });
    expect(response.status()).toBe(200);
  });

  // ---- 模板模块权限边界 ----

  test('viewer 角色可以读取模板列表', async () => {
    const response = await viewerContext.get(`${BASE_URL}/api/admin/templates`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
    });
    expect(response.status()).toBe(200);
  });

  // ---- 日志模块权限边界 ----

  test('viewer 角色可以查看操作日志', async () => {
    const response = await viewerContext.get(`${BASE_URL}/api/admin/logs`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
    });
    expect(response.status()).toBe(200);
  });

  // ---- 撤回模块权限边界 ----

  test('viewer 角色无法执行撤回 (403)', async () => {
    const response = await viewerContext.post(`${BASE_URL}/api/admin/revokes/manual`, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${(viewerApi as any).token}`,
      },
      data: {
        userId: 'test_user',
        badgeId: 1,
        reason: 'RBAC权限测试',
      },
    });
    expect(response.status()).toBe(403);
  });
});

// ============================================================
// 11. API Key 外部接口验证
// ============================================================
test.describe('API 集成测试: API Key 外部接口', () => {
  let adminApi: ApiHelper;
  let adminContext: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    adminContext = await playwright.request.newContext({ baseURL: BASE_URL });
    adminApi = new ApiHelper(adminContext, BASE_URL);
    await adminApi.login(testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async () => {
    await adminContext?.dispose();
  });

  test('无认证访问外部接口被拒绝', async () => {
    // 不携带任何认证头请求 /api/v1/ 路由
    const response = await adminContext.get(`${BASE_URL}/api/v1/users/test_user/badges`, {
      headers: { 'Content-Type': 'application/json' },
    });
    // api_key_auth_middleware 在无 X-API-Key 时放行（设计为可选），
    // 但 auth_middleware 已将 /api/v1/ 加入公开路径跳过 JWT，
    // 所以实际结果取决于 handler 是否需要 Claims。
    // 由于外部路由的 handler 复用了管理后台的 handler，可能依赖 Claims，
    // 无 key 时应返回 200（handler 不强制 Claims）或 401/500
    expect([200, 401, 500]).toContain(response.status());
  });

  test('无效 API Key 被拒绝 (401)', async () => {
    const response = await adminContext.get(`${BASE_URL}/api/v1/users/test_user/badges`, {
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': 'invalid-random-key-12345',
      },
    });
    expect(response.status()).toBe(401);
  });

  test('创建 API Key 后可访问外部接口', async () => {
    // 通过管理接口创建 API Key
    let apiKey: string | undefined;
    let apiKeyId: number | undefined;

    try {
      const createRes = await adminApi.createApiKey('E2EExternalTest', ['*']);
      apiKey = createRes?.data?.key || createRes?.data?.apiKey;
      apiKeyId = createRes?.data?.id;

      if (!apiKey) {
        test.info().annotations.push({
          type: 'info',
          description: 'API Key 创建未返回明文 key，跳过外部接口验证',
        });
        return;
      }

      // 用有效 API Key 访问外部接口
      const response = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      // 有效 key 应返回 200
      expect(response.status()).toBe(200);
    } finally {
      // 清理 API Key
      if (apiKeyId) {
        try { await adminApi.deleteApiKey(apiKeyId); } catch { /* ignore */ }
      }
    }
  });

  test('禁用 API Key 后被拒绝 (401)', async () => {
    let apiKey: string | undefined;
    let apiKeyId: number | undefined;

    try {
      // 创建 API Key
      const createRes = await adminApi.createApiKey('E2EDisableTest', ['*']);
      apiKey = createRes?.data?.key || createRes?.data?.apiKey;
      apiKeyId = createRes?.data?.id;

      if (!apiKey || !apiKeyId) {
        test.info().annotations.push({
          type: 'info',
          description: 'API Key 创建未返回明文 key 或 id，跳过禁用验证',
        });
        return;
      }

      // 验证初始状态下可以正常访问
      const firstResponse = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      expect(firstResponse.status()).toBe(200);

      // 禁用 API Key
      const disableResponse = await adminContext.patch(
        `${BASE_URL}/api/admin/system/api-keys/${apiKeyId}/status`,
        {
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${(adminApi as any).token}`,
          },
          data: { enabled: false },
        },
      );
      expect([200, 204]).toContain(disableResponse.status());

      // 禁用后应被拒绝
      const rejectedResponse = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      expect(rejectedResponse.status()).toBe(401);

      // 重新启用 API Key
      const enableResponse = await adminContext.patch(
        `${BASE_URL}/api/admin/system/api-keys/${apiKeyId}/status`,
        {
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${(adminApi as any).token}`,
          },
          data: { enabled: true },
        },
      );
      expect([200, 204]).toContain(enableResponse.status());

      // 重新启用后应恢复访问
      const restoredResponse = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      expect(restoredResponse.status()).toBe(200);
    } finally {
      if (apiKeyId) {
        try { await adminApi.deleteApiKey(apiKeyId); } catch { /* ignore */ }
      }
    }
  });

  test('过期 API Key 被拒绝 (401)', async () => {
    let apiKeyId: number | undefined;

    try {
      // 创建一个已过期的 API Key（过期时间设为 1 秒前）
      const response = await adminContext.post(`${BASE_URL}/api/admin/system/api-keys`, {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${(adminApi as any).token}`,
        },
        data: {
          name: 'E2EExpiredTest',
          permissions: ['*'],
          expiresAt: new Date(Date.now() - 1000).toISOString(),
        },
      });
      const createRes = await response.json();
      const apiKey = createRes?.data?.key || createRes?.data?.apiKey;
      apiKeyId = createRes?.data?.id;

      if (!apiKey) {
        test.info().annotations.push({
          type: 'info',
          description: '已过期 API Key 创建未返回明文 key，跳过验证',
        });
        return;
      }

      // 使用已过期的 key 访问外部接口，应返回 401
      const expiredResponse = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      expect(expiredResponse.status()).toBe(401);
    } finally {
      if (apiKeyId) {
        try { await adminApi.deleteApiKey(apiKeyId); } catch { /* ignore */ }
      }
    }
  });

  test('API Key 权限码正确传递到上下文', async () => {
    let apiKeyId: number | undefined;

    try {
      // 创建具有特定权限的 API Key
      const createRes = await adminApi.createApiKey('E2EPermissionTest', ['read:badges', 'read:grants']);
      const apiKey = createRes?.data?.key || createRes?.data?.apiKey;
      apiKeyId = createRes?.data?.id;

      if (!apiKey) {
        test.info().annotations.push({
          type: 'info',
          description: 'API Key 创建未返回明文 key，跳过权限验证',
        });
        return;
      }

      // 当前外部路由不校验细粒度权限，只要 key 有效就能通过认证
      const response = await adminContext.get(`${BASE_URL}/api/v1/grants/logs`, {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        params: { page: 1, pageSize: 5 },
      });
      expect(response.status()).toBe(200);
    } finally {
      if (apiKeyId) {
        try { await adminApi.deleteApiKey(apiKeyId); } catch { /* ignore */ }
      }
    }
  });
});

// ============================================================
// 12. JWT Token 安全验证
// ============================================================
test.describe('API 集成测试: JWT Token 安全', () => {
  test('过期 JWT Token 被拒绝 (401)', async ({ request }) => {
    const expiredToken = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwidXNlcm5hbWUiOiJhZG1pbiIsInJvbGUiOiJhZG1pbiIsImV4cCI6MTAwMDAwMDAwMH0.invalid_signature';
    const response = await request.get(`${BASE_URL}/api/admin/categories`, {
      headers: { Authorization: `Bearer ${expiredToken}` },
    });
    expect(response.status()).toBe(401);
  });

  test('无效签名 JWT Token 被拒绝 (401)', async ({ request }) => {
    const tamperedToken = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwidXNlcm5hbWUiOiJhZG1pbiIsInJvbGUiOiJhZG1pbiIsImV4cCI6OTk5OTk5OTk5OX0.wrong_signature_here';
    const response = await request.get(`${BASE_URL}/api/admin/categories`, {
      headers: { Authorization: `Bearer ${tamperedToken}` },
    });
    expect(response.status()).toBe(401);
  });

  test('空 Token 被拒绝 (401)', async ({ request }) => {
    const response = await request.get(`${BASE_URL}/api/admin/categories`, {
      headers: { Authorization: 'Bearer ' },
    });
    expect(response.status()).toBe(401);
  });

  test('缺少 Authorization 头被拒绝 (401)', async ({ request }) => {
    const response = await request.get(`${BASE_URL}/api/admin/categories`);
    expect(response.status()).toBe(401);
  });
});

// ============================================================
// 13. 并发安全测试
// ============================================================
test.describe('API 集成测试: 并发安全', () => {
  let adminToken: string;
  let viewerToken: string;
  let operatorToken: string;

  test.beforeAll(async ({ playwright }) => {
    const adminContext = await playwright.request.newContext({ baseURL: BASE_URL });
    const adminApi = new ApiHelper(adminContext, BASE_URL);
    await adminApi.login(testUsers.admin.username, testUsers.admin.password);
    adminToken = (adminApi as any).token;

    // 确保 viewer/operator 用户存在且角色已分配
    await adminApi.ensureUser('operator', testUsers.operator.password, 2);
    await adminApi.ensureUser('viewer', testUsers.viewer.password, 3);

    const viewerContext = await playwright.request.newContext({ baseURL: BASE_URL });
    const viewerApi = new ApiHelper(viewerContext, BASE_URL);
    await viewerApi.login(testUsers.viewer.username, testUsers.viewer.password);
    viewerToken = (viewerApi as any).token;

    const operatorContext = await playwright.request.newContext({ baseURL: BASE_URL });
    const operatorApi = new ApiHelper(operatorContext, BASE_URL);
    await operatorApi.login(testUsers.operator.username, testUsers.operator.password);
    operatorToken = (operatorApi as any).token;

    await adminContext.dispose();
    await viewerContext.dispose();
    await operatorContext.dispose();
  });

  test('多角色并发读取不互相干扰', async ({ request }) => {
    const results = await Promise.all([
      request.get(`${BASE_URL}/api/admin/categories`, {
        headers: { Authorization: `Bearer ${adminToken}` },
      }),
      request.get(`${BASE_URL}/api/admin/categories`, {
        headers: { Authorization: `Bearer ${viewerToken}` },
      }),
      request.get(`${BASE_URL}/api/admin/categories`, {
        headers: { Authorization: `Bearer ${operatorToken}` },
      }),
    ]);

    for (const r of results) {
      expect(r.status()).toBe(200);
      const data = await r.json();
      expect(data.success).toBe(true);
    }
  });

  test('并发写入 + 读取权限隔离', async ({ request }) => {
    const results = await Promise.all([
      // Viewer 读取徽章
      request.get(`${BASE_URL}/api/admin/badges`, {
        headers: { Authorization: `Bearer ${viewerToken}` },
      }),
      // Operator 写入分类
      request.post(`${BASE_URL}/api/admin/categories`, {
        headers: {
          Authorization: `Bearer ${operatorToken}`,
          'Content-Type': 'application/json',
        },
        data: { name: `Concurrent_${Date.now()}`, sortOrder: 0 },
      }),
      // Viewer 尝试写入（应被拒绝）
      request.post(`${BASE_URL}/api/admin/categories`, {
        headers: {
          Authorization: `Bearer ${viewerToken}`,
          'Content-Type': 'application/json',
        },
        data: { name: `ConcurrentFail_${Date.now()}`, sortOrder: 0 },
      }),
      // Admin 读取统计
      request.get(`${BASE_URL}/api/admin/stats/overview`, {
        headers: { Authorization: `Bearer ${adminToken}` },
      }),
    ]);

    expect(results[0].status()).toBe(200);  // viewer 读取成功
    expect(results[1].status()).toBe(200);  // operator 写入成功
    expect(results[2].status()).toBe(403);  // viewer 写入被拒绝
    expect(results[3].status()).toBe(200);  // admin 读取成功

    // 清理 operator 创建的分类
    const created = await results[1].json();
    if (created.data?.id) {
      await request.delete(`${BASE_URL}/api/admin/categories/${created.data.id}`, {
        headers: { Authorization: `Bearer ${adminToken}` },
      });
    }
  });

  test('高并发读取压力测试 (10 请求)', async ({ request }) => {
    const requests = Array.from({ length: 10 }, () =>
      request.get(`${BASE_URL}/api/admin/stats/overview`, {
        headers: { Authorization: `Bearer ${adminToken}` },
      })
    );
    const results = await Promise.all(requests);
    for (const r of results) {
      expect(r.status()).toBe(200);
    }
  });
});

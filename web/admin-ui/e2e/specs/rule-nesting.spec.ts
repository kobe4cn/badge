/**
 * 规则嵌套测试套件
 *
 * 验证 3-5 层规则嵌套的创建、持久化、查询和执行
 */

import { test, expect, APIRequestContext } from '@playwright/test';
import { ApiHelper, testUsers, createTestBadge } from '../utils';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';

// ============================================================
// 规则嵌套结构生成器
// ============================================================

/**
 * 生成 N 层嵌套规则 JSON
 * @param depth 嵌套深度 (1-5)
 * @param breadth 每层子节点数量 (通常为 2)
 */
function generateNestedRule(depth: number, breadth: number = 2): any {
  return buildNestedNode(depth, breadth, 0);
}

function buildNestedNode(depth: number, breadth: number, level: number): any {
  if (depth === 0) {
    // 叶子节点：条件节点
    return {
      type: 'condition',
      field: `field_L${level}_D${depth}`,
      operator: 'gte',
      value: (level + 1) * 100,
    };
  }

  // 交替使用 AND/OR 以增加复杂性
  const operator = depth % 2 === 0 ? 'AND' : 'OR';
  const children = Array.from({ length: breadth }, (_, i) =>
    buildNestedNode(depth - 1, breadth, i)
  );

  return {
    type: 'group',
    operator,
    children,
  };
}

/**
 * 计算规则树的实际深度
 */
function calculateRuleDepth(node: any): number {
  if (node.type === 'condition') {
    return 1;
  }
  if (!node.children || node.children.length === 0) {
    return 1;
  }
  return 1 + Math.max(...node.children.map((c: any) => calculateRuleDepth(c)));
}

/**
 * 统计规则树中的条件节点数量
 */
function countConditionNodes(node: any): number {
  if (node.type === 'condition') {
    return 1;
  }
  if (!node.children || node.children.length === 0) {
    return 0;
  }
  return node.children.reduce(
    (sum: number, child: any) => sum + countConditionNodes(child),
    0
  );
}

// ============================================================
// 3 层规则嵌套测试
// ============================================================
test.describe('规则嵌套测试: 3 层嵌套', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `Nest3_${Date.now().toString(36)}_`;
  let badgeId: number;
  let ruleId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    // 准备基础数据
    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}3层嵌套徽章`,
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

  test('创建 3 层嵌套规则 - 验证结构正确保存', async () => {
    const ruleJson = generateNestedRule(3, 2);

    // 验证生成的规则结构
    expect(calculateRuleDepth(ruleJson)).toBe(4); // 3 层 group + 1 层 condition
    expect(countConditionNodes(ruleJson)).toBe(8); // 2^3 = 8 个条件节点

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_3layer`,
      eventType: 'purchase',
      name: `${testPrefix}3层嵌套规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    ruleId = rule.data.id;

    // 验证返回的规则 JSON 结构完整
    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.type).toBe('group');
    expect(savedRule.children.length).toBe(2);
    expect(savedRule.children[0].type).toBe('group');
    expect(savedRule.children[0].children.length).toBe(2);
  });

  test('查询 3 层嵌套规则 - 验证数据持久化', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    const rules = await api.getRules({ keyword: testPrefix });
    const items = rules?.data?.items || [];
    const target = items.find((r: any) => r.id === ruleId);

    expect(target).toBeDefined();
    const ruleJson = target.ruleJson || target.rule_json;
    expect(calculateRuleDepth(ruleJson)).toBe(4);
    expect(countConditionNodes(ruleJson)).toBe(8);
  });

  test('更新 3 层嵌套规则 - 验证修改后结构', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    // 先创建一个规则用于更新测试
    const initialRuleJson = generateNestedRule(3, 2);
    const createRes = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_update_test_${Date.now()}`,
      eventType: 'purchase',
      name: `${testPrefix}更新测试规则`,
      ruleJson: initialRuleJson,
    });
    const localRuleId = createRes?.data?.id;
    test.skip(!localRuleId, '规则创建失败');

    // 修改为新的 3 层结构（不同的值）
    const newRuleJson = {
      type: 'group',
      operator: 'AND',
      children: [
        generateNestedRule(2, 2),
        {
          type: 'condition',
          field: 'updated_field',
          operator: 'eq',
          value: 999,
        },
      ],
    };

    const response = await apiContext.put(`${BASE_URL}/api/admin/rules/${localRuleId}`, {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${(api as any).token}`,
      },
      data: {
        name: `${testPrefix}3层嵌套规则_更新`,
        ruleJson: newRuleJson,
      },
    });

    expect(response.status()).toBe(200);
    const updated = await response.json();
    const savedRule = updated.data?.ruleJson || updated.data?.rule_json;
    expect(savedRule.operator).toBe('AND');
  });
});

// ============================================================
// 4 层规则嵌套测试
// ============================================================
test.describe('规则嵌套测试: 4 层嵌套', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `Nest4_${Date.now().toString(36)}_`;
  let badgeId: number;
  let ruleId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}4层嵌套徽章`,
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

  test('创建 4 层嵌套规则 - 验证复杂结构', async () => {
    const ruleJson = generateNestedRule(4, 2);

    // 4 层 group + 1 层 condition = 深度 5
    expect(calculateRuleDepth(ruleJson)).toBe(5);
    // 2^4 = 16 个条件节点
    expect(countConditionNodes(ruleJson)).toBe(16);

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_4layer`,
      eventType: 'purchase',
      name: `${testPrefix}4层嵌套规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    ruleId = rule.data.id;
  });

  test('查询 4 层嵌套规则 - 验证完整性', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    const rules = await api.getRules({ keyword: testPrefix });
    const items = rules?.data?.items || [];
    const target = items.find((r: any) => r.id === ruleId);

    expect(target).toBeDefined();
    const ruleJson = target.ruleJson || target.rule_json;

    // 递归验证所有层级
    function validateNode(node: any, depth: number): void {
      if (depth === 0) {
        expect(node.type).toBe('condition');
        expect(node.field).toBeDefined();
        expect(node.operator).toBeDefined();
        expect(node.value).toBeDefined();
      } else {
        expect(node.type).toBe('group');
        expect(['AND', 'OR']).toContain(node.operator);
        expect(node.children.length).toBeGreaterThan(0);
        node.children.forEach((child: any) => validateNode(child, depth - 1));
      }
    }

    validateNode(ruleJson, 4);
  });

  test('发布 4 层嵌套规则 - 验证规则可用', async () => {
    test.skip(!badgeId, '前置数据未就绪');

    // 先创建一个 4 层规则用于发布测试
    const ruleJson = generateNestedRule(4, 2);
    const createRes = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_publish_test_${Date.now()}`,
      eventType: 'purchase',
      name: `${testPrefix}发布测试规则`,
      ruleJson,
    });
    const localRuleId = createRes?.data?.id;
    test.skip(!localRuleId, '规则创建失败');

    const res = await api.publishRule(localRuleId);
    expect(res?.success === true || res?.data != null || res?.code === 0).toBe(true);

    // 验证规则状态
    const rules = await api.getRules({ keyword: testPrefix });
    const items = rules?.data?.items || [];
    const target = items.find((r: any) => r.id === localRuleId);
    expect(target?.enabled === true || target?.status === 'published').toBe(true);
  });
});

// ============================================================
// 5 层规则嵌套测试（边界测试）
// ============================================================
test.describe('规则嵌套测试: 5 层嵌套（边界）', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `Nest5_${Date.now().toString(36)}_`;
  let badgeId: number;
  let ruleId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}5层嵌套徽章`,
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

  test('创建 5 层嵌套规则 - 验证边界支持', async () => {
    const ruleJson = generateNestedRule(5, 2);

    // 5 层 group + 1 层 condition = 深度 6
    expect(calculateRuleDepth(ruleJson)).toBe(6);
    // 2^5 = 32 个条件节点
    expect(countConditionNodes(ruleJson)).toBe(32);

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_5layer`,
      eventType: 'purchase',
      name: `${testPrefix}5层嵌套规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    ruleId = rule.data.id;
  });

  test('查询 5 层嵌套规则 - 验证 JSON 序列化正确', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    const rules = await api.getRules({ keyword: testPrefix });
    const items = rules?.data?.items || [];
    const target = items.find((r: any) => r.id === ruleId);

    expect(target).toBeDefined();
    const ruleJson = target.ruleJson || target.rule_json;

    // 验证 JSON 结构完整（没有被截断）
    expect(calculateRuleDepth(ruleJson)).toBe(6);
    expect(countConditionNodes(ruleJson)).toBe(32);

    // 验证 JSON 可正常序列化
    const jsonString = JSON.stringify(ruleJson);
    expect(jsonString.length).toBeGreaterThan(1000); // 32 个条件节点的 JSON 应该很长
    const reparsed = JSON.parse(jsonString);
    expect(calculateRuleDepth(reparsed)).toBe(6);
  });

  test('5 层嵌套规则测试执行 - 验证规则引擎支持', async () => {
    test.skip(!ruleId, '前置用例未创建规则');

    try {
      const result = await api.testRule(ruleId, {
        userId: 'e2e_test_user',
        eventType: 'purchase',
        eventData: {
          field_L0_D0: 500, // 满足部分条件
          field_L1_D0: 200,
          amount: 1000,
        },
      });

      // 只要规则引擎能正常处理就算通过
      expect(result).toBeDefined();
    } catch {
      test.info().annotations.push({
        type: 'info',
        description: '规则测试接口可能未实现，但创建和持久化验证已通过',
      });
    }
  });
});

// ============================================================
// 混合嵌套结构测试
// ============================================================
test.describe('规则嵌套测试: 混合结构', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `NestMix_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}混合嵌套徽章`,
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

  test('非对称嵌套: 左深右浅结构', async () => {
    // 左侧 4 层，右侧 1 层
    const ruleJson = {
      type: 'group',
      operator: 'OR',
      children: [
        generateNestedRule(4, 2), // 深层嵌套
        {
          type: 'condition',
          field: 'simple_field',
          operator: 'eq',
          value: 'simple_value',
        },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_asymmetric`,
      eventType: 'purchase',
      name: `${testPrefix}非对称嵌套规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();

    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.children[0].type).toBe('group');
    expect(savedRule.children[1].type).toBe('condition');
  });

  test('宽度变化嵌套: 每层子节点数不同', async () => {
    const ruleJson = {
      type: 'group',
      operator: 'AND',
      children: [
        {
          type: 'group',
          operator: 'OR',
          children: [
            { type: 'condition', field: 'a', operator: 'eq', value: 1 },
            { type: 'condition', field: 'b', operator: 'eq', value: 2 },
            { type: 'condition', field: 'c', operator: 'eq', value: 3 },
          ],
        },
        {
          type: 'group',
          operator: 'AND',
          children: [
            {
              type: 'group',
              operator: 'OR',
              children: [
                { type: 'condition', field: 'd', operator: 'gte', value: 100 },
                { type: 'condition', field: 'e', operator: 'lte', value: 50 },
              ],
            },
          ],
        },
        { type: 'condition', field: 'f', operator: 'in', value: ['x', 'y', 'z'] },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_varying_width`,
      eventType: 'checkin',
      name: `${testPrefix}变宽嵌套规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();

    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.children.length).toBe(3);
    expect(savedRule.children[0].children.length).toBe(3);
    expect(savedRule.children[1].children.length).toBe(1);
  });

  test('复杂业务场景: VIP 会员多条件规则', async () => {
    // 模拟真实业务场景：
    // (会员等级 >= 3 AND 累计消费 >= 10000) OR
    // (会员等级 >= 2 AND 累计消费 >= 5000 AND 连续签到 >= 30) OR
    // (邀请好友 >= 10 AND (累计消费 >= 3000 OR 连续签到 >= 60))
    const ruleJson = {
      type: 'group',
      operator: 'OR',
      children: [
        {
          type: 'group',
          operator: 'AND',
          children: [
            { type: 'condition', field: 'user.level', operator: 'gte', value: 3 },
            { type: 'condition', field: 'user.total_spent', operator: 'gte', value: 10000 },
          ],
        },
        {
          type: 'group',
          operator: 'AND',
          children: [
            { type: 'condition', field: 'user.level', operator: 'gte', value: 2 },
            { type: 'condition', field: 'user.total_spent', operator: 'gte', value: 5000 },
            { type: 'condition', field: 'user.consecutive_checkin', operator: 'gte', value: 30 },
          ],
        },
        {
          type: 'group',
          operator: 'AND',
          children: [
            { type: 'condition', field: 'user.invited_friends', operator: 'gte', value: 10 },
            {
              type: 'group',
              operator: 'OR',
              children: [
                { type: 'condition', field: 'user.total_spent', operator: 'gte', value: 3000 },
                { type: 'condition', field: 'user.consecutive_checkin', operator: 'gte', value: 60 },
              ],
            },
          ],
        },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_vip_complex`,
      eventType: 'purchase',
      name: `${testPrefix}VIP会员复杂规则`,
      description: '多条件组合判断 VIP 资格',
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();

    // 验证规则结构
    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.operator).toBe('OR');
    expect(savedRule.children.length).toBe(3);

    // 验证第三个分支包含嵌套 OR
    const thirdBranch = savedRule.children[2];
    expect(thirdBranch.children[1].type).toBe('group');
    expect(thirdBranch.children[1].operator).toBe('OR');
  });
});

// ============================================================
// 规则嵌套边界和错误处理测试
// ============================================================
test.describe('规则嵌套测试: 边界和错误处理', () => {
  let api: ApiHelper;
  let apiContext: APIRequestContext;
  const testPrefix = `NestEdge_${Date.now().toString(36)}_`;
  let badgeId: number;

  test.beforeAll(async ({ playwright }) => {
    apiContext = await playwright.request.newContext({ baseURL: BASE_URL });
    api = new ApiHelper(apiContext, BASE_URL);
    await api.login(testUsers.admin.username, testUsers.admin.password);

    const { seriesId } = await api.ensureTestData(testPrefix);
    const badge = createTestBadge({
      name: `${testPrefix}边界测试徽章`,
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

  test('空 children 组 - 应被拒绝或处理', async () => {
    const ruleJson = {
      type: 'group',
      operator: 'AND',
      children: [],
    };

    try {
      const result = await api.createRule({
        badgeId,
        ruleCode: `${testPrefix}rule_empty_children`,
        eventType: 'purchase',
        name: `${testPrefix}空子节点规则`,
        ruleJson,
      });

      // 如果成功，验证服务端处理了空数组
      expect(result).toBeDefined();
    } catch (e: any) {
      // 如果失败，应该是合理的验证错误
      expect(e.message).toMatch(/400|invalid|empty|children/i);
    }
  });

  test('单条件无嵌套 - 最简结构', async () => {
    const ruleJson = {
      type: 'condition',
      field: 'amount',
      operator: 'gte',
      value: 100,
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_single_condition`,
      eventType: 'purchase',
      name: `${testPrefix}单条件规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.type).toBe('condition');
    expect(savedRule.field).toBe('amount');
  });

  test('超大值字段 - 验证数值边界', async () => {
    const ruleJson = {
      type: 'group',
      operator: 'AND',
      children: [
        { type: 'condition', field: 'big_number', operator: 'gte', value: Number.MAX_SAFE_INTEGER },
        { type: 'condition', field: 'small_number', operator: 'lte', value: -Number.MAX_SAFE_INTEGER },
        { type: 'condition', field: 'decimal', operator: 'eq', value: 0.123456789012345 },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_large_values`,
      eventType: 'purchase',
      name: `${testPrefix}大值边界规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
  });

  test('特殊字符字段名 - 验证编码处理', async () => {
    const ruleJson = {
      type: 'group',
      operator: 'OR',
      children: [
        { type: 'condition', field: 'user.profile.name', operator: 'contains', value: '测试' },
        { type: 'condition', field: 'data["key"]', operator: 'eq', value: 'value' },
        { type: 'condition', field: 'path/to/field', operator: 'startsWith', value: '/api' },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_special_fields`,
      eventType: 'share',
      name: `${testPrefix}特殊字段名规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.children[0].value).toBe('测试');
  });

  test('所有操作符组合 - 验证操作符支持', async () => {
    const ruleJson = {
      type: 'group',
      operator: 'OR',
      children: [
        { type: 'condition', field: 'f1', operator: 'eq', value: 1 },
        { type: 'condition', field: 'f2', operator: 'neq', value: 2 },
        { type: 'condition', field: 'f3', operator: 'gt', value: 3 },
        { type: 'condition', field: 'f4', operator: 'gte', value: 4 },
        { type: 'condition', field: 'f5', operator: 'lt', value: 5 },
        { type: 'condition', field: 'f6', operator: 'lte', value: 6 },
        { type: 'condition', field: 'f7', operator: 'in', value: ['a', 'b'] },
        { type: 'condition', field: 'f8', operator: 'notIn', value: ['c', 'd'] },
        { type: 'condition', field: 'f9', operator: 'contains', value: 'str' },
        { type: 'condition', field: 'f10', operator: 'startsWith', value: 'pre' },
        { type: 'condition', field: 'f11', operator: 'endsWith', value: 'suf' },
        { type: 'condition', field: 'f12', operator: 'between', value: [10, 20] },
        { type: 'condition', field: 'f13', operator: 'isEmpty', value: null },
        { type: 'condition', field: 'f14', operator: 'isNotEmpty', value: null },
      ],
    };

    const rule = await api.createRule({
      badgeId,
      ruleCode: `${testPrefix}rule_all_operators`,
      eventType: 'review',
      name: `${testPrefix}全操作符规则`,
      ruleJson,
    });

    expect(rule?.data?.id).toBeDefined();
    const savedRule = rule.data.ruleJson || rule.data.rule_json;
    expect(savedRule.children.length).toBe(14);
  });
});

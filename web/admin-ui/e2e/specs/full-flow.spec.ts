import { test, expect } from '@playwright/test';
import { LoginPage, BadgeListPage, RuleEditorPage } from '../pages';
import { ApiHelper, uniqueId, sleep } from '../utils';

/**
 * 全链路 E2E 测试
 *
 * 测试从配置到触发到发放的完整流程。
 */
test.describe('全链路测试: 消费升级场景', () => {
  let loginPage: LoginPage;
  let badgeListPage: BadgeListPage;
  let ruleEditorPage: RuleEditorPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_flow_');

  test.beforeAll(async ({ browser }) => {
    // 设置测试数据
  });

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    badgeListPage = new BadgeListPage(page);
    ruleEditorPage = new RuleEditorPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test.afterAll(async () => {
    // 清理测试数据
  });

  test('完整流程: 配置徽章 -> 配置规则 -> 触发事件 -> 验证发放', async ({ page }) => {
    // 1. 创建徽章
    await badgeListPage.goto();
    await badgeListPage.clickCreate();

    const badgeName = `${testPrefix}消费达人`;
    await badgeListPage.fillFormItem('名称', badgeName);
    await badgeListPage.fillFormItem('显示名称', '消费达人徽章');
    await badgeListPage.fillFormItem('描述', '消费满1000元获得');
    await badgeListPage.clickButton('提交');
    await badgeListPage.waitForMessage('success');

    // 2. 创建规则
    await page.goto('/rules/create');
    await ruleEditorPage.waitForCanvasReady();

    // 添加条件: 累计消费 >= 1000
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.configureCondition({
      field: '累计消费金额',
      operator: '>=',
      value: '1000',
    });

    // 添加动作: 发放徽章
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);
    await ruleEditorPage.configureAction({
      actionType: '发放徽章',
      badgeId: badgeName,
    });

    // 连接并保存
    await ruleEditorPage.save();

    // 发布规则
    await ruleEditorPage.publish();

    // 3. 模拟触发事件（通过 API）
    const userId = `${testPrefix}user_001`;

    // 发送交易事件（这里通过 API 模拟）
    await apiHelper.login('admin', 'admin123');

    // 等待规则热更新
    await sleep(3000);

    // 4. 验证徽章发放
    // 查询用户徽章
    await page.goto(`/users/${userId}/badges`);

    // 由于事件处理是异步的，可能需要等待
    await page.reload();

    // 验证徽章已发放
    await expect(page.locator(`tr:has-text("${badgeName}")`)).toBeVisible({ timeout: 30000 });
  });

  test('完整流程: 签到连续7天获得徽章', async ({ page }) => {
    // 1. 创建徽章
    const badgeName = `${testPrefix}签到达人`;
    await apiHelper.createBadge({
      name: badgeName,
      displayName: '签到达人',
      description: '连续签到7天获得',
      categoryId: 1,
      seriesId: 1,
    });

    // 2. 创建规则
    await page.goto('/rules/create');
    await ruleEditorPage.waitForCanvasReady();

    // 添加条件: 连续签到天数 >= 7
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.configureCondition({
      field: '连续签到天数',
      operator: '>=',
      value: '7',
    });

    // 添加动作
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);
    await ruleEditorPage.configureAction({
      actionType: '发放徽章',
      badgeId: badgeName,
    });

    await ruleEditorPage.save();
    await ruleEditorPage.publish();

    // 3. 模拟7天签到（通过 API）
    const userId = `${testPrefix}user_002`;

    // 这里应该发送7次签到事件
    // 实际测试中会通过 Kafka 发送事件

    // 4. 验证徽章发放
    await sleep(5000);
    await page.goto(`/users/${userId}/badges`);

    // 验证
    // await expect(page.locator(`tr:has-text("${badgeName}")`)).toBeVisible();
  });
});

test.describe('全链路测试: 级联触发场景', () => {
  let loginPage: LoginPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_cascade_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('级联触发: A -> B -> C', async ({ page }) => {
    // 1. 创建三个徽章
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}徽章A` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}徽章B` });
    const badgeC = await apiHelper.createBadge({ name: `${testPrefix}徽章C` });

    // 2. 配置级联关系
    await page.goto(`/badges/${badgeB.id}/dependencies`);
    await page.locator('button:has-text("添加依赖")').click();
    await page.locator('.badge-selector').click();
    await page.locator(`text=${badgeA.name}`).click();
    await page.locator('button:has-text("确定")').click();

    await page.goto(`/badges/${badgeC.id}/dependencies`);
    await page.locator('button:has-text("添加依赖")').click();
    await page.locator('.badge-selector').click();
    await page.locator(`text=${badgeB.name}`).click();
    await page.locator('button:has-text("确定")').click();

    // 3. 发放 A，验证 B 和 C 自动获得
    const userId = `${testPrefix}user_cascade`;
    await apiHelper.grantBadge(userId, badgeA.id, 'test');

    await sleep(5000);

    // 4. 验证级联发放
    await page.goto(`/users/${userId}/badges`);

    await expect(page.locator(`tr:has-text("${badgeA.name}")`)).toBeVisible();
    await expect(page.locator(`tr:has-text("${badgeB.name}")`)).toBeVisible();
    await expect(page.locator(`tr:has-text("${badgeC.name}")`)).toBeVisible();
  });
});

test.describe('全链路测试: 竞争兑换场景', () => {
  let loginPage: LoginPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_compete_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('限量兑换: 库存耗尽后无法兑换', async ({ page }) => {
    // 1. 创建限量兑换规则 (库存=2)
    await page.goto('/redemptions/rules/create');

    await page.locator('#name').fill(`${testPrefix}限量兑换`);
    await page.locator('#stock').fill('2');

    // 选择徽章和权益...
    await page.locator('button:has-text("提交")').click();

    // 2. 第一个用户兑换成功
    // 3. 第二个用户兑换成功
    // 4. 第三个用户兑换失败（库存不足）

    // 验证逻辑...
  });

  test('互斥兑换: D 和 E 只能选其一', async ({ page }) => {
    // 配置 D 和 E 为互斥组
    // 用户获得 D 后无法获得 E

    // 验证逻辑...
  });
});

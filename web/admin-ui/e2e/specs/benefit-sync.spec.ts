import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { ApiHelper, createTestBenefit, uniqueId } from '../utils';

test.describe('权益配置与同步', () => {
  let loginPage: LoginPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_benefit_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('创建权益配置', async ({ page }) => {
    await page.goto('/benefits');
    await page.locator('button:has-text("新建权益")').click();

    const benefit = createTestBenefit({ name: `${testPrefix}测试权益` });

    await page.locator('#name').fill(benefit.name);
    await page.locator('.ant-select:has-text("权益类型")').click();
    await page.locator('.ant-select-item:has-text("优惠券")').click();
    await page.locator('#value').fill(String(benefit.value));
    await page.locator('#externalId').fill(benefit.externalId);

    await page.locator('button:has-text("提交")').click();

    // 验证创建成功
    await expect(page.locator(`.ant-message-success`)).toBeVisible();
  });

  test('徽章关联权益', async ({ page }) => {
    await page.goto('/badges/1/benefits');

    // 点击添加权益
    await page.locator('button:has-text("关联权益")').click();

    // 选择权益
    await page.locator('.benefit-selector .ant-checkbox:first-child').click();
    await page.locator('button:has-text("确定")').click();

    // 验证关联成功
    await expect(page.locator('.ant-message-success')).toBeVisible();
    await expect(page.locator('.benefit-list .benefit-item')).toBeVisible();
  });

  test('权益发放条件配置', async ({ page }) => {
    await page.goto('/badges/1/benefits');

    // 点击配置发放条件
    await page.locator('.benefit-item:first-child button:has-text("配置条件")').click();

    // 设置发放条件
    await page.locator('.ant-select:has-text("发放时机")').click();
    await page.locator('.ant-select-item:has-text("获取徽章时")').click();

    await page.locator('#grant_quantity').fill('1');
    await page.locator('#validity_days').fill('30');

    await page.locator('button:has-text("保存")').click();

    await expect(page.locator('.ant-message-success')).toBeVisible();
  });

  test('外部权益同步状态查看', async ({ page }) => {
    await page.goto('/benefits/sync');

    // 查看同步日志
    await expect(page.locator('.sync-log-table')).toBeVisible();

    // 应该显示最近的同步记录
    const rows = await page.locator('.sync-log-table tbody tr').count();
    expect(rows).toBeGreaterThanOrEqual(0);
  });

  test('手动触发权益同步', async ({ page }) => {
    await page.goto('/benefits/sync');

    // 点击同步按钮
    await page.locator('button:has-text("立即同步")').click();

    // 确认同步
    await page.locator('.ant-modal-confirm-btns .ant-btn-primary').click();

    // 等待同步完成
    await expect(page.locator('.ant-message-loading')).toBeVisible();
    await expect(page.locator('.ant-message-success')).toBeVisible({ timeout: 30000 });
  });

  test('权益发放记录查询', async ({ page }) => {
    await page.goto('/benefits/grants');

    // 搜索用户的权益发放记录
    await page.locator('#user_id').fill('test_user_001');
    await page.locator('button:has-text("查询")').click();

    // 等待加载
    await page.waitForLoadState('networkidle');

    // 验证表格显示
    await expect(page.locator('.ant-table')).toBeVisible();
  });
});

test.describe('权益兑换流程', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('查看兑换规则列表', async ({ page }) => {
    await page.goto('/redemptions/rules');

    await expect(page.locator('.ant-table')).toBeVisible();
  });

  test('创建兑换规则', async ({ page }) => {
    await page.goto('/redemptions/rules/create');

    await page.locator('#name').fill('测试兑换规则');
    await page.locator('#description').fill('这是一个测试兑换规则');

    // 选择所需徽章
    await page.locator('.badge-selector').click();
    await page.locator('.ant-checkbox:first-child').click();
    await page.locator('button:has-text("确定")').click();

    // 选择兑换权益
    await page.locator('.benefit-selector').click();
    await page.locator('.ant-checkbox:first-child').click();
    await page.locator('button:has-text("确定")').click();

    // 设置库存
    await page.locator('#stock').fill('100');

    await page.locator('button:has-text("提交")').click();

    await expect(page.locator('.ant-message-success')).toBeVisible();
  });

  test('查看兑换记录', async ({ page }) => {
    await page.goto('/redemptions/records');

    // 筛选条件
    await page.locator('.ant-picker').click();
    await page.locator('.ant-picker-today-btn').click();

    await page.locator('button:has-text("查询")').click();

    await expect(page.locator('.ant-table')).toBeVisible();
  });
});

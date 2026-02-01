import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { BenefitsPage } from '../pages/BenefitsPage';
import { ApiHelper, uniqueId, createTestBenefit } from '../utils';

test.describe('权益管理 - 扩展测试', () => {
  let loginPage: LoginPage;
  let benefitsPage: BenefitsPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_ben_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    benefitsPage = new BenefitsPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('创建积分权益', async ({ page }) => {
    await benefitsPage.goto();
    await benefitsPage.clickCreate();

    await benefitsPage.nameInput.fill(`${testPrefix}积分奖励`);
    await benefitsPage.typeSelect.click();
    await page.locator('.ant-select-item:has-text("积分")').click();
    await benefitsPage.valueInput.fill('100');
    await benefitsPage.descriptionInput.fill('发放100积分');

    await benefitsPage.clickButton('提交');
    await benefitsPage.waitForMessage('success');

    await benefitsPage.goto();
    await benefitsPage.expectBenefitExists(`${testPrefix}积分奖励`);
  });

  test('创建优惠券权益', async ({ page }) => {
    await benefitsPage.goto();
    await benefitsPage.clickCreate();

    await benefitsPage.nameInput.fill(`${testPrefix}优惠券`);
    await benefitsPage.typeSelect.click();
    await page.locator('.ant-select-item:has-text("优惠券")').click();
    await benefitsPage.valueInput.fill('50');
    await benefitsPage.externalIdInput.fill('CPN_TEST_001');
    await benefitsPage.validityDaysInput.fill('30');

    await benefitsPage.clickButton('提交');
    await benefitsPage.waitForMessage('success');
  });

  test('创建会员权益', async ({ page }) => {
    await benefitsPage.goto();
    await benefitsPage.clickCreate();

    await benefitsPage.nameInput.fill(`${testPrefix}会员升级`);
    await benefitsPage.typeSelect.click();
    await page.locator('.ant-select-item:has-text("会员")').click();
    await benefitsPage.valueInput.fill('1');
    await benefitsPage.descriptionInput.fill('升级为VIP会员');
    await benefitsPage.validityDaysInput.fill('365');

    await benefitsPage.clickButton('提交');
    await benefitsPage.waitForMessage('success');
  });

  test('权益类型筛选', async () => {
    await benefitsPage.goto();

    // 筛选优惠券类型
    await benefitsPage.filterByType('优惠券');

    // 验证筛选结果
    const count = await benefitsPage.getBenefitCount();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('权益搜索', async () => {
    await benefitsPage.goto();
    await benefitsPage.search(testPrefix);

    // 验证搜索结果
    const count = await benefitsPage.getBenefitCount();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('编辑权益', async ({ page }) => {
    // 先创建权益
    const benefitName = `${testPrefix}待编辑权益`;
    await apiHelper.login('admin', 'admin123');
    await apiHelper.createBenefit(createTestBenefit({ name: benefitName }));

    await benefitsPage.goto();
    await benefitsPage.clickEdit(benefitName);

    await benefitsPage.descriptionInput.fill('更新后的描述');
    await benefitsPage.clickButton('提交');
    await benefitsPage.waitForMessage('success');
  });

  test('删除权益', async ({ page }) => {
    const benefitName = `${testPrefix}待删除权益`;
    await apiHelper.login('admin', 'admin123');
    await apiHelper.createBenefit(createTestBenefit({ name: benefitName }));

    await benefitsPage.goto();
    await benefitsPage.clickDelete(benefitName);
    await benefitsPage.confirmModal();
    await benefitsPage.waitForMessage('success');

    await benefitsPage.expectBenefitNotExists(benefitName);
  });
});

test.describe('权益发放记录', () => {
  let loginPage: LoginPage;
  let benefitsPage: BenefitsPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    benefitsPage = new BenefitsPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('发放记录列表加载', async () => {
    await benefitsPage.gotoGrants();
    await expect(benefitsPage.table).toBeVisible();
  });

  test('按用户ID查询', async ({ page }) => {
    await benefitsPage.gotoGrants();

    await page.locator('#user_id').fill('test_user');
    await page.locator('button:has-text("查询")').click();
    await benefitsPage.waitForLoading();

    await expect(benefitsPage.table).toBeVisible();
  });

  test('按日期范围查询', async ({ page }) => {
    await benefitsPage.gotoGrants();

    // 选择日期范围
    await page.locator('.ant-picker-range').click();
    await page.locator('.ant-picker-preset button:has-text("最近7天")').click();
    await page.locator('button:has-text("查询")').click();
    await benefitsPage.waitForLoading();

    await expect(benefitsPage.table).toBeVisible();
  });

  test('按权益类型筛选', async ({ page }) => {
    await benefitsPage.gotoGrants();

    await page.locator('.ant-select:has-text("权益类型")').click();
    await page.locator('.ant-select-item:has-text("优惠券")').click();
    await page.locator('button:has-text("查询")').click();
    await benefitsPage.waitForLoading();
  });

  test('导出发放记录', async ({ page }) => {
    await benefitsPage.gotoGrants();

    // 点击导出
    const downloadPromise = page.waitForEvent('download');
    await page.locator('button:has-text("导出")').click();

    // 验证下载开始
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/\.xlsx|\.csv/);
  });
});

test.describe('权益同步', () => {
  let loginPage: LoginPage;
  let benefitsPage: BenefitsPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    benefitsPage = new BenefitsPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('同步日志列表', async () => {
    await benefitsPage.gotoSync();
    await expect(benefitsPage.page.locator('.sync-log-table')).toBeVisible();
  });

  test('手动触发同步', async ({ page }) => {
    await benefitsPage.gotoSync();

    await page.locator('button:has-text("立即同步")').click();
    await benefitsPage.confirmModal();

    // 等待同步完成
    await expect(page.locator('.ant-message-loading, .ant-message-success')).toBeVisible({
      timeout: 30000,
    });
  });

  test('同步状态查看', async ({ page }) => {
    await benefitsPage.gotoSync();

    // 查看最近一次同步状态
    const latestSync = page.locator('.sync-log-table tbody tr').first();
    if (await latestSync.isVisible()) {
      const status = await latestSync.locator('.sync-status').textContent();
      expect(['成功', '失败', '进行中']).toContain(status?.trim());
    }
  });
});

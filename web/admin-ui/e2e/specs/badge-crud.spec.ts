import { test, expect } from '@playwright/test';
import { LoginPage, BadgeListPage } from '../pages';
import { ApiHelper, createTestBadge, testUsers, uniqueId } from '../utils';

test.describe('徽章 CRUD 操作', () => {
  let loginPage: LoginPage;
  let badgeListPage: BadgeListPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_badge_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    badgeListPage = new BadgeListPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    // 登录
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test.afterEach(async () => {
    // 清理测试数据
    await apiHelper.cleanup(testPrefix);
  });

  test('创建新徽章', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.clickCreate();

    // 填写表单
    const badge = createTestBadge({ name: `${testPrefix}新徽章` });
    await badgeListPage.fillFormItem('名称', badge.name);
    await badgeListPage.fillFormItem('显示名称', badge.displayName);
    await badgeListPage.fillFormItem('描述', badge.description);
    await badgeListPage.selectOption('分类', '默认分类');
    await badgeListPage.selectOption('系列', '默认系列');

    // 提交
    await badgeListPage.clickButton('提交');
    await badgeListPage.waitForMessage('success');

    // 验证创建成功
    await badgeListPage.goto();
    await badgeListPage.expectBadgeExists(badge.name);
  });

  test('编辑徽章', async ({ page }) => {
    // 先通过 API 创建徽章
    const badge = createTestBadge({ name: `${testPrefix}待编辑` });
    await apiHelper.createBadge(badge);

    await badgeListPage.goto();
    await badgeListPage.clickEdit(badge.name);

    // 修改名称
    const newDisplayName = `${testPrefix}已修改`;
    await badgeListPage.fillFormItem('显示名称', newDisplayName);
    await badgeListPage.clickButton('提交');
    await badgeListPage.waitForMessage('success');

    // 验证修改成功
    await badgeListPage.goto();
    await expect(page.locator(`tr:has-text("${newDisplayName}")`)).toBeVisible();
  });

  test('删除徽章', async ({ page }) => {
    // 先通过 API 创建徽章
    const badge = createTestBadge({ name: `${testPrefix}待删除` });
    await apiHelper.createBadge(badge);

    await badgeListPage.goto();
    await badgeListPage.expectBadgeExists(badge.name);

    // 删除
    await badgeListPage.clickDelete(badge.name);
    await badgeListPage.confirmModal();
    await badgeListPage.waitForMessage('success');

    // 验证删除成功
    await badgeListPage.expectBadgeNotExists(badge.name);
  });

  test('搜索徽章', async ({ page }) => {
    // 创建多个徽章
    const badge1 = createTestBadge({ name: `${testPrefix}搜索测试A` });
    const badge2 = createTestBadge({ name: `${testPrefix}搜索测试B` });
    const badge3 = createTestBadge({ name: `${testPrefix}其他徽章` });

    await apiHelper.createBadge(badge1);
    await apiHelper.createBadge(badge2);
    await apiHelper.createBadge(badge3);

    await badgeListPage.goto();

    // 搜索 "搜索测试"
    await badgeListPage.search('搜索测试');

    // 应该显示 badge1 和 badge2
    await badgeListPage.expectBadgeExists(badge1.name);
    await badgeListPage.expectBadgeExists(badge2.name);
    // 不应该显示 badge3
    await badgeListPage.expectBadgeNotExists(badge3.name);
  });

  test('分页功能', async ({ page }) => {
    // 创建超过一页的徽章
    const badges = [];
    for (let i = 0; i < 15; i++) {
      badges.push(createTestBadge({ name: `${testPrefix}分页测试_${i}` }));
    }

    for (const badge of badges) {
      await apiHelper.createBadge(badge);
    }

    await badgeListPage.goto();

    // 验证分页存在
    await expect(badgeListPage.pagination).toBeVisible();

    // 点击第二页
    await page.locator('.ant-pagination-item-2').click();
    await badgeListPage.waitForLoading();

    // 验证第二页有数据
    const count = await badgeListPage.getBadgeCount();
    expect(count).toBeGreaterThan(0);
  });
});

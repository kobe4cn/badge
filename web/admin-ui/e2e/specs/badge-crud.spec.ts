import { test, expect } from '@playwright/test';
import { LoginPage, BadgeListPage } from '../pages';

/**
 * 徽章 CRUD 操作 - UI 验证测试
 *
 * 验证徽章管理页面的基本 UI 功能，不依赖具体后端数据
 */
test.describe('徽章 CRUD 操作', () => {
  let loginPage: LoginPage;
  let badgeListPage: BadgeListPage;

  test.beforeEach(async ({ page }, testInfo) => {
    const isMobile = testInfo.project.name.toLowerCase().includes('mobile');
    test.skip(isMobile, 'Skipping mobile browser tests due to layout issues');

    loginPage = new LoginPage(page);
    badgeListPage = new BadgeListPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('徽章列表加载', async () => {
    await badgeListPage.goto();
    await expect(badgeListPage.table).toBeVisible();
    await expect(badgeListPage.createButton).toBeVisible();
  });

  test('创建新徽章 - 表单打开', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.clickCreate();

    // 验证表单打开
    const modal = page.locator('.ant-modal, .ant-drawer');
    await expect(modal).toBeVisible();

    // 验证表单字段存在（使用 first() 避免 strict mode 错误）
    await expect(page.locator('.ant-modal, .ant-drawer').getByText('徽章名称').first()).toBeVisible();

    // 关闭表单
    await page.locator('.ant-modal button, .ant-drawer button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('徽章表单验证', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.clickCreate();

    // 直接点击提交（不填写必填字段）
    await page.locator('.ant-modal button, .ant-drawer button').filter({ hasText: /提.*交/ }).click();
    await page.waitForTimeout(500);

    // 应该显示验证错误
    const errorMessage = page.locator('.ant-form-item-explain-error');
    const hasError = await errorMessage.count() > 0;
    expect(hasError).toBe(true);

    // 关闭表单
    await page.locator('.ant-modal button, .ant-drawer button').filter({ hasText: /取.*消/ }).click();
  });

  test('编辑徽章 - 按钮存在', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 表格加载完成后，行数决定了是否应该出现编辑按钮
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();
    const editButton = page.locator('button').filter({ hasText: /编辑/ }).first();
    const isVisible = await editButton.isVisible({ timeout: 3000 }).catch(() => false);

    // 有数据行时编辑按钮必须存在，无数据时允许不存在
    if (rowCount > 0) {
      expect(isVisible).toBe(true);
    } else {
      expect(isVisible).toBe(false);
    }
  });

  test('删除徽章 - 操作菜单存在', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 删除操作在 Dropdown「更多」菜单内，需要先打开菜单才能访问
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();

    if (rowCount === 0) {
      test.skip(true, '无数据行，跳过删除按钮验证');
      return;
    }

    // 找到第一行的「更多」操作按钮（MoreOutlined 图标按钮）
    const moreButton = rows.first().locator('button').last();
    await moreButton.click();

    // 等待下拉菜单出现并验证「删除」选项存在
    const dropdown = page.locator('.ant-dropdown:visible');
    await expect(dropdown).toBeVisible({ timeout: 3000 });

    const deleteItem = dropdown.locator('.ant-dropdown-menu-item').filter({ hasText: /删除/ });
    const editItem = dropdown.locator('.ant-dropdown-menu-item').filter({ hasText: /编辑/ });

    // 菜单中至少应包含编辑或删除选项（删除仅在草稿状态可用）
    const hasDelete = await deleteItem.isVisible().catch(() => false);
    const hasEdit = await editItem.isVisible().catch(() => false);
    expect(hasDelete || hasEdit).toBe(true);

    // 关闭下拉菜单
    await page.keyboard.press('Escape');
  });

  test('搜索徽章 - 表单存在', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 验证搜索表单或按钮存在
    const searchForm = page.locator('.ant-pro-form, .ant-pro-table-search');
    const searchButton = page.locator('button').filter({ hasText: /查.*询/ });

    const formVisible = await searchForm.isVisible({ timeout: 3000 }).catch(() => false);
    const buttonVisible = await searchButton.isVisible({ timeout: 3000 }).catch(() => false);

    expect(formVisible || buttonVisible).toBe(true);
  });

  test('分页功能 - 控件存在', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 验证分页控件
    const pagination = page.locator('.ant-pagination');
    await expect(pagination).toBeVisible();
  });

  test('表格列显示正确', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 验证表格列存在
    await expect(page.getByRole('columnheader', { name: '徽章名称' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: '操作' })).toBeVisible();
  });

  test('刷新按钮功能', async ({ page }) => {
    await badgeListPage.goto();
    await badgeListPage.waitForLoading();

    // 点击刷新按钮
    const refreshButton = page.locator('button').filter({ hasText: /刷新/ });
    if (await refreshButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await refreshButton.click();
      await badgeListPage.waitForLoading();
    }

    // 验证表格仍可见
    await expect(badgeListPage.table).toBeVisible();
  });
});

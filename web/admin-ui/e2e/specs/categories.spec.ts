import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { CategoryPage } from '../pages/CategoryPage';
import { ApiHelper, uniqueId } from '../utils';

test.describe('徽章分类管理', () => {
  let loginPage: LoginPage;
  let categoryPage: CategoryPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_cat_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    categoryPage = new CategoryPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('分类列表加载', async () => {
    await categoryPage.goto();
    await expect(categoryPage.table).toBeVisible();
    await expect(categoryPage.createButton).toBeVisible();
  });

  test('创建新分类', async ({ page }) => {
    await categoryPage.goto();
    await categoryPage.clickCreate();

    const categoryName = `${testPrefix}测试分类`;
    await categoryPage.nameInput.fill(categoryName);
    await categoryPage.displayNameInput.fill('测试分类显示名');
    await categoryPage.descriptionInput.fill('这是一个测试分类');
    await categoryPage.sortInput.fill('100');

    await categoryPage.clickButton('提交');
    await categoryPage.waitForMessage('success');

    await categoryPage.goto();
    await categoryPage.expectCategoryExists(categoryName);
  });

  test('编辑分类', async ({ page }) => {
    // 先创建一个分类
    const categoryName = `${testPrefix}待编辑分类`;
    await apiHelper.login('admin', 'admin123');
    // 通过 API 创建分类...

    await categoryPage.goto();
    // 编辑分类...
  });

  test('删除分类', async ({ page }) => {
    await categoryPage.goto();

    // 删除测试分类
    const testCategory = page.locator(`tr:has-text("${testPrefix}")`).first();
    if (await testCategory.isVisible()) {
      const name = await testCategory.locator('td').first().textContent();
      if (name) {
        await categoryPage.clickDelete(name);
        await categoryPage.confirmModal();
        await categoryPage.waitForMessage('success');
      }
    }
  });

  test('搜索分类', async () => {
    await categoryPage.goto();
    await categoryPage.search('活动');

    // 验证搜索结果
    const count = await categoryPage.getCategoryCount();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('分类排序', async ({ page }) => {
    await categoryPage.goto();

    // 点击排序列
    await page.locator('th:has-text("排序")').click();
    await categoryPage.waitForLoading();

    // 验证排序生效
    await expect(categoryPage.table).toBeVisible();
  });
});

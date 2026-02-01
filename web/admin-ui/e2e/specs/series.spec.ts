import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { SeriesPage } from '../pages/SeriesPage';
import { ApiHelper, uniqueId } from '../utils';

test.describe('徽章系列管理', () => {
  let loginPage: LoginPage;
  let seriesPage: SeriesPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_ser_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    seriesPage = new SeriesPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('系列列表加载', async () => {
    await seriesPage.goto();
    await expect(seriesPage.table).toBeVisible();
    await expect(seriesPage.createButton).toBeVisible();
  });

  test('创建新系列', async ({ page }) => {
    await seriesPage.goto();
    await seriesPage.clickCreate();

    const seriesName = `${testPrefix}测试系列`;
    await seriesPage.nameInput.fill(seriesName);
    await seriesPage.displayNameInput.fill('测试系列显示名');
    await seriesPage.descriptionInput.fill('这是一个测试系列');

    // 选择分类
    await seriesPage.categorySelect.click();
    await page.locator('.ant-select-item').first().click();

    // 选择主题
    await seriesPage.themeSelect.click();
    await page.locator('.ant-select-item:has-text("blue")').click();

    await seriesPage.clickButton('提交');
    await seriesPage.waitForMessage('success');

    await seriesPage.goto();
    await seriesPage.expectSeriesExists(seriesName);
  });

  test('按分类筛选系列', async ({ page }) => {
    await seriesPage.goto();

    // 选择分类筛选
    await seriesPage.categoryFilter.click();
    const firstCategory = page.locator('.ant-select-item').first();
    if (await firstCategory.isVisible()) {
      await firstCategory.click();
      await seriesPage.waitForLoading();
    }
  });

  test('编辑系列', async ({ page }) => {
    await seriesPage.goto();

    const testSeries = page.locator(`tr:has-text("${testPrefix}")`).first();
    if (await testSeries.isVisible()) {
      const name = await testSeries.locator('td').first().textContent();
      if (name) {
        await seriesPage.clickEdit(name);
        await seriesPage.displayNameInput.fill('更新后的显示名');
        await seriesPage.clickButton('提交');
        await seriesPage.waitForMessage('success');
      }
    }
  });

  test('删除系列', async ({ page }) => {
    await seriesPage.goto();

    const testSeries = page.locator(`tr:has-text("${testPrefix}")`).first();
    if (await testSeries.isVisible()) {
      const name = await testSeries.locator('td').first().textContent();
      if (name) {
        await seriesPage.clickDelete(name);
        await seriesPage.confirmModal();
        await seriesPage.waitForMessage('success');
      }
    }
  });
});

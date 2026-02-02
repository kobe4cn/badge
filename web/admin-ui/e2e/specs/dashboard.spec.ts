import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { DashboardPage } from '../pages/DashboardPage';

test.describe('数据看板', () => {
  let loginPage: LoginPage;
  let dashboardPage: DashboardPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dashboardPage = new DashboardPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('看板页面加载正常', async () => {
    await dashboardPage.goto();

    // 验证今日统计卡片
    await expect(dashboardPage.todayGrantsCard).toBeVisible();
    await expect(dashboardPage.newHoldersCard).toBeVisible();
    await expect(dashboardPage.todayRedemptionsCard).toBeVisible();

    // 验证总量统计卡片
    await expect(dashboardPage.totalGrantsCard).toBeVisible();
    await expect(dashboardPage.activeBadgesCard).toBeVisible();
    await expect(dashboardPage.badgeHoldersCard).toBeVisible();
  });

  test('图表渲染正常', async () => {
    await dashboardPage.goto();
    await dashboardPage.waitForChartsLoad();

    await expect(dashboardPage.grantTrendChart).toBeVisible();
    await expect(dashboardPage.categoryDistChart).toBeVisible();
    await expect(dashboardPage.topBadgesChart).toBeVisible();
  });

  test('日期范围筛选', async ({ page }) => {
    await dashboardPage.goto();

    // 使用 Segmented 切换时间范围
    await page.locator('.ant-segmented-item:has-text("30天")').first().click();

    // 验证图表刷新
    await dashboardPage.waitForChartsLoad();
  });

  test('刷新功能', async () => {
    await dashboardPage.goto();

    await dashboardPage.refreshButton.click();
    await dashboardPage.waitForLoading();

    // 刷新后数据应该加载完成
    await expect(dashboardPage.totalGrantsCard).toBeVisible();
  });

  test('统计数据格式正确', async () => {
    await dashboardPage.goto();

    const totalGrants = await dashboardPage.getStatValue('总发放数');
    // 应该是数字格式、带千分位的数字、或"-"（无数据时）
    expect(totalGrants).toMatch(/^[\d,]+$|^-$/);
  });

  test('响应式布局', async ({ page }) => {
    await dashboardPage.goto();

    // 缩小视口
    await page.setViewportSize({ width: 768, height: 1024 });
    await expect(dashboardPage.todayGrantsCard).toBeVisible();

    // 移动端视口
    await page.setViewportSize({ width: 375, height: 667 });
    await expect(dashboardPage.todayGrantsCard).toBeVisible();
  });
});

test.describe('实时数据更新', () => {
  let loginPage: LoginPage;
  let dashboardPage: DashboardPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dashboardPage = new DashboardPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('自动刷新开关', async ({ page }) => {
    await dashboardPage.goto();

    // 开启自动刷新
    const autoRefreshToggle = page.locator('.auto-refresh-toggle');
    if (await autoRefreshToggle.isVisible()) {
      await autoRefreshToggle.click();
      await expect(autoRefreshToggle).toHaveClass(/active/);
    }
  });
});

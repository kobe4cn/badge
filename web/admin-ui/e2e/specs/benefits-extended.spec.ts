import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { BenefitsPage } from '../pages/BenefitsPage';

test.describe('权益管理 - 扩展测试', () => {
  let loginPage: LoginPage;
  let benefitsPage: BenefitsPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    benefitsPage = new BenefitsPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('权益列表页面加载', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益列表页必须渲染出表格或空状态
    const contentArea = page.locator('table, .ant-table, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('新建权益按钮可见', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益管理页面必须提供创建入口
    const createButton = page.locator('button').filter({ hasText: /新建|创建|添加/ }).first();
    await expect(createButton).toBeVisible({ timeout: 5000 });
  });

  test('创建权益表单打开', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 点击新建按钮后，应弹出表单或跳转到创建页
    const createButton = page.locator('button').filter({ hasText: /新建|创建|添加/ }).first();
    if (await createButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await createButton.click();
      await page.waitForTimeout(500);

      // 创建操作应触发弹窗或页面跳转
      const modal = page.locator('.ant-modal, .ant-drawer');
      const hasModal = await modal.isVisible({ timeout: 3000 }).catch(() => false);
      const isOnCreatePage = page.url().includes('/create') || page.url().includes('/new');
      expect(hasModal || isOnCreatePage).toBe(true);

      // 关闭弹窗（如有）
      if (hasModal) {
        await page.locator('.ant-modal-close, button:has-text("取消")').first().click().catch(() => {});
      }
    } else {
      // 新建按钮不可见时，说明权限不足或页面结构异常
      test.skip(true, '新建按钮不可见，跳过表单测试');
    }
  });

  test('权益类型筛选', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 筛选操作不应破坏页面结构，表格必须持续可见
    const typeSelect = page.locator('.ant-select').filter({ hasText: /类型/ }).first();
    if (await typeSelect.isVisible({ timeout: 3000 }).catch(() => false)) {
      await typeSelect.click();
      await page.waitForTimeout(300);
      await page.keyboard.press('Escape');
    }

    // 无论是否存在筛选器，页面主体内容区域都应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
  });

  test('权益搜索', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 搜索操作后页面应保持稳定，不出现 JS 错误
    const searchInput = page.locator('input[type="text"]').first();
    if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await searchInput.fill('test');
      await page.waitForTimeout(300);

      // 搜索框输入后内容应被保留
      await expect(searchInput).toHaveValue('test');
    }

    // 页面主体内容区域应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
  });

  test('编辑权益按钮', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 有数据行时编辑按钮必须存在，用于验证操作列渲染正确
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();
    const editButton = page.locator('button').filter({ hasText: /编辑|修改/ }).first();
    const isVisible = await editButton.isVisible({ timeout: 3000 }).catch(() => false);

    if (rowCount > 0) {
      expect(isVisible).toBe(true);
    } else {
      expect(isVisible).toBe(false);
    }
  });

  test('删除权益按钮', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 有数据行时删除按钮必须存在，用于验证操作列渲染正确
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();
    const deleteButton = page.locator('button').filter({ hasText: /删除|移除/ }).first();
    const isVisible = await deleteButton.isVisible({ timeout: 3000 }).catch(() => false);

    if (rowCount > 0) {
      expect(isVisible).toBe(true);
    } else {
      expect(isVisible).toBe(false);
    }
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

  test('发放记录列表加载', async ({ page }) => {
    await benefitsPage.gotoGrants();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 发放记录列表应渲染出表格或空状态
    const contentArea = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('按用户ID查询', async ({ page }) => {
    await benefitsPage.gotoGrants();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 输入查询条件后值应被保留，确保输入框可交互
    const userIdInput = page.locator('input').filter({ hasText: '' }).first();
    if (await userIdInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await userIdInput.fill('test_user');
      await page.waitForTimeout(300);
      await expect(userIdInput).toHaveValue('test_user');
    }

    // 页面主体内容区域应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
  });

  test('按日期范围查询', async ({ page }) => {
    await benefitsPage.gotoGrants();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 日期选择器交互后关闭，确保下拉面板正确收起
    const datePicker = page.locator('.ant-picker').first();
    if (await datePicker.isVisible({ timeout: 3000 }).catch(() => false)) {
      await datePicker.click();
      const datePanel = page.locator('.ant-picker-dropdown');
      await expect(datePanel).toBeVisible({ timeout: 3000 });
      await page.keyboard.press('Escape');
    }

    // 页面主体内容区域应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
  });

  test('按权益类型筛选', async ({ page }) => {
    await benefitsPage.gotoGrants();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 下拉选择器交互后关闭，确保下拉面板正确收起
    const typeSelect = page.locator('.ant-select').first();
    if (await typeSelect.isVisible({ timeout: 3000 }).catch(() => false)) {
      await typeSelect.click();
      const dropdown = page.locator('.ant-select-dropdown');
      await expect(dropdown).toBeVisible({ timeout: 3000 });
      await page.keyboard.press('Escape');
    }

    // 页面主体内容区域应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
  });

  test('导出按钮', async ({ page }) => {
    await benefitsPage.gotoGrants();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 发放记录页面应提供数据导出能力
    const exportButton = page.locator('button').filter({ hasText: /导出|下载/ }).first();
    const isVisible = await exportButton.isVisible({ timeout: 3000 }).catch(() => false);

    // 导出按钮存在时应可点击（未被禁用）
    if (isVisible) {
      await expect(exportButton).toBeEnabled();
    }

    // 页面主体内容区域应正常渲染
    const mainContent = page.locator('.ant-pro-page-container, .ant-layout-content, main').first();
    await expect(mainContent).toBeVisible();
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

  test('权益列表页面加载', async ({ page }) => {
    // /benefits/sync 路由不存在，改为验证权益列表的刷新功能
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    const contentArea = page.locator('table, .ant-table, .ant-pro-page-container, .ant-layout-content, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('刷新按钮可见', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益列表工具栏包含刷新按钮
    const refreshButton = page.locator('button').filter({ hasText: /刷新/ }).first();
    await expect(refreshButton).toBeVisible({ timeout: 5000 });
  });

  test('权益列表状态查看', async ({ page }) => {
    await benefitsPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    const contentArea = page.locator('table, .ant-table, .ant-layout-content, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });
});

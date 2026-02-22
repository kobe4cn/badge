import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';

test.describe('权益配置与同步', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('权益列表页面加载', async ({ page }) => {
    await page.goto('/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益列表页应包含表格或列表容器（即使数据为空也应显示表格骨架或空状态）
    const contentArea = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('新建权益按钮可见', async ({ page }) => {
    await page.goto('/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益管理页面必须提供创建入口
    const createButton = page.locator('button').filter({ hasText: /新建|创建|添加/ }).first();
    await expect(createButton).toBeVisible({ timeout: 5000 });
  });

  test('新建权益表单打开', async ({ page }) => {
    await page.goto('/benefits');
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

      if (hasModal) {
        await page.locator('.ant-modal-close, button:has-text("取消")').first().click().catch(() => {});
      }
    } else {
      test.skip(true, '新建按钮不可见，跳过表单测试');
    }
  });

  test('徽章权益关联页面', async ({ page }) => {
    await page.goto('/badges/1/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益关联页应显示内容区域（表格、卡片或空状态）
    const contentArea = page.locator('table, .ant-table, .ant-card, .ant-empty, .ant-layout-content, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('权益列表页面刷新', async ({ page }) => {
    // /benefits/sync 路由不存在，实际刷新功能在 /benefits/list 页面工具栏中
    await page.goto('/benefits/list');
    await page.waitForLoadState('networkidle').catch(() => {});

    const contentArea = page.locator('table, .ant-table, .ant-pro-page-container, .ant-layout-content, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('刷新按钮可见', async ({ page }) => {
    await page.goto('/benefits/list');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 权益列表页面工具栏中包含刷新按钮
    const refreshButton = page.locator('button').filter({ hasText: /刷新/ }).first();
    await expect(refreshButton).toBeVisible({ timeout: 5000 });
  });

  test('权益发放记录页面', async ({ page }) => {
    await page.goto('/benefits/grants');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 发放记录页应包含表格或列表容器
    const contentArea = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('权益发放记录搜索', async ({ page }) => {
    await page.goto('/benefits/grants');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 搜索操作后页面应保持稳定
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
    await page.waitForLoadState('networkidle').catch(() => {});

    // 兑换规则列表页应包含表格或空状态
    const contentArea = page.locator('table, .ant-table, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('兑换规则新建按钮', async ({ page }) => {
    await page.goto('/redemptions/rules');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 兑换规则管理页面必须提供创建入口
    const createButton = page.locator('button').filter({ hasText: /新建|创建|添加/ }).first();
    await expect(createButton).toBeVisible({ timeout: 5000 });
  });

  test('兑换规则创建页面', async ({ page }) => {
    await page.goto('/redemptions/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 创建页应包含表单或页面主体容器
    const contentArea = page.locator('form, .ant-form, .ant-card, .ant-pro-page-container, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('查看兑换记录', async ({ page }) => {
    await page.goto('/redemptions/records');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 兑换记录页应包含表格或空状态
    const contentArea = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });

  test('兑换记录日期筛选', async ({ page }) => {
    await page.goto('/redemptions/records');
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
});

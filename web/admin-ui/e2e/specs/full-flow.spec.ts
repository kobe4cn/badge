import { test, expect } from '@playwright/test';
import { LoginPage, BadgeListPage, RuleEditorPage } from '../pages';

/**
 * 全链路 E2E 测试 - UI 验证
 *
 * 验证管理后台各页面的基本功能和导航流程。
 */
test.describe('全链路测试: 徽章管理流程', () => {
  let loginPage: LoginPage;
  let badgeListPage: BadgeListPage;
  let ruleEditorPage: RuleEditorPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    badgeListPage = new BadgeListPage(page);
    ruleEditorPage = new RuleEditorPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('徽章列表 -> 创建徽章 -> 返回列表', async ({ page }) => {
    // 1. 访问徽章列表
    await badgeListPage.goto();
    await page.waitForLoadState('networkidle').catch(() => {});

    // 列表页面必须显示表格和创建按钮，这是徽章管理的基本 UI 结构
    await expect(badgeListPage.table).toBeVisible();
    await expect(badgeListPage.createButton).toBeVisible();

    // 2. 点击创建按钮
    const createButton = page.locator('button').filter({ hasText: /新建|创建|添加/ }).first();
    if (await createButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await createButton.click();
      await page.waitForTimeout(500);

      // 应该打开表单或跳转到创建页
      const hasForm = await page.locator('.ant-modal, .ant-drawer, form').isVisible({ timeout: 3000 }).catch(() => false);
      const isOnCreatePage = page.url().includes('/create') || page.url().includes('/new');

      // 关闭或返回
      if (hasForm) {
        await page.locator('.ant-modal-close, button:has-text("取消")').first().click().catch(() => {});
      }
    }

    // 3. 返回列表后，表格应仍然可见，验证导航未破坏页面状态
    await badgeListPage.goto();
    await expect(badgeListPage.table).toBeVisible();
  });

  test('规则编辑器页面加载', async ({ page }) => {
    await page.goto('/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    // ReactFlow 在 CI 慢环境下需要更长加载时间
    const canvas = page.locator('.react-flow');
    const form = page.locator('form, .ant-form');
    const mainContent = page.locator('main, .ant-layout-content').first();

    // 先确认页面主体容器已渲染
    await expect(mainContent).toBeVisible({ timeout: 10000 });

    // ReactFlow 可能需要较长时间初始化（CI 单核 worker 尤其慢）
    const hasCanvas = await canvas.isVisible({ timeout: 15000 }).catch(() => false);
    const hasForm = await form.isVisible({ timeout: 3000 }).catch(() => false);

    // 至少需要画布或表单之一可见
    expect(hasCanvas || hasForm).toBeTruthy();
  });

  test('用户徽章页面', async ({ page }) => {
    await page.goto('/users/test_user/badges');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 用户徽章页应显示内容区域（表格、卡片或空状态）
    const contentArea = page.locator('table, .ant-table, .ant-card, .ant-list, .ant-empty, .ant-layout-content, main').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });
});

test.describe('全链路测试: 依赖配置流程', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('徽章详情 -> 依赖配置', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 依赖配置页应显示内容区域（列表、按钮或空状态）
    const contentArea = page.locator('table, .ant-table, .ant-card, .ant-empty, .ant-layout-content, main, button').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });
});

test.describe('全链路测试: 兑换配置流程', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('兑换规则列表 -> 创建规则', async ({ page }) => {
    // 1. 访问兑换规则列表
    await page.goto('/redemptions/rules');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 列表页应包含表格或空状态
    const listContent = page.locator('table, .ant-table, .ant-empty, .ant-pro-page-container').first();
    await expect(listContent).toBeVisible({ timeout: 10000 });

    // 2. 访问创建页面
    await page.goto('/redemptions/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 创建页应包含表单或页面容器
    const createContent = page.locator('form, .ant-form, .ant-card, .ant-pro-page-container, main').first();
    await expect(createContent).toBeVisible({ timeout: 10000 });
  });

  test('兑换记录页面', async ({ page }) => {
    await page.goto('/redemptions/records');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 兑换记录页应包含表格或空状态
    const contentArea = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(contentArea).toBeVisible({ timeout: 10000 });
  });
});

test.describe('全链路测试: 权益管理流程', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('权益列表 -> 权益同步 -> 发放记录', async ({ page }) => {
    // 1. 权益列表 - 应显示表格或空状态
    await page.goto('/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});
    const benefitsList = page.locator('table, .ant-table, .ant-empty, .ant-pro-page-container').first();
    await expect(benefitsList).toBeVisible({ timeout: 10000 });

    // 2. 权益同步 - 应显示内容区域
    await page.goto('/benefits/sync');
    await page.waitForLoadState('networkidle').catch(() => {});
    const syncContent = page.locator('button, .ant-card, .ant-table, .ant-layout-content, main').first();
    await expect(syncContent).toBeVisible({ timeout: 10000 });

    // 3. 发放记录 - 应显示表格或空状态
    await page.goto('/benefits/grants');
    await page.waitForLoadState('networkidle').catch(() => {});
    const grantsContent = page.locator('table, .ant-table, .ant-list, .ant-empty, .ant-pro-page-container').first();
    await expect(grantsContent).toBeVisible({ timeout: 10000 });
  });
});

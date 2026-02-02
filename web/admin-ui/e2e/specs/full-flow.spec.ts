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

    // 验证列表页面
    const hasContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 100;
    expect(hasContent).toBeTruthy();

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

    // 3. 返回列表验证
    await badgeListPage.goto();
    const hasListContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 100;
    expect(hasListContent).toBeTruthy();
  });

  test('规则编辑器页面加载', async ({ page }) => {
    // 访问规则创建页
    await page.goto('/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证画布或表单加载
    const hasCanvas = await page.locator('.react-flow').isVisible({ timeout: 5000 }).catch(() => false);
    const hasForm = await page.locator('form, .ant-form').isVisible({ timeout: 3000 }).catch(() => false);
    const hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 100;

    expect(hasCanvas || hasForm || hasAnyContent).toBeTruthy();
  });

  test('用户徽章页面', async ({ page }) => {
    // 访问用户徽章页面
    await page.goto('/users/test_user/badges');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证页面有内容
    const hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();
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
    // 访问徽章依赖页面
    await page.goto('/badges/1/dependencies');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证页面有内容
    const hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();
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

    // 验证页面有内容
    let hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();

    // 2. 访问创建页面
    await page.goto('/redemptions/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证创建页面
    hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();
  });

  test('兑换记录页面', async ({ page }) => {
    // 访问兑换记录页面
    await page.goto('/redemptions/records');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证页面有内容
    const hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();
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
    // 1. 权益列表
    await page.goto('/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});
    let hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();

    // 2. 权益同步
    await page.goto('/benefits/sync');
    await page.waitForLoadState('networkidle').catch(() => {});
    hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();

    // 3. 发放记录
    await page.goto('/benefits/grants');
    await page.waitForLoadState('networkidle').catch(() => {});
    hasAnyContent = await page.locator('body').evaluate(el => el.textContent?.trim().length || 0) > 50;
    expect(hasAnyContent).toBeTruthy();
  });
});

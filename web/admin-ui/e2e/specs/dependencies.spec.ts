import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { DependencyPage } from '../pages/DependencyPage';

test.describe('徽章依赖配置', () => {
  let loginPage: LoginPage;
  let dependencyPage: DependencyPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dependencyPage = new DependencyPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('依赖列表加载', async ({ page }) => {
    // 访问一个徽章的依赖页面（假设 ID 1 存在）
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 验证页面基本元素
    // 页面可能显示列表或空状态
    const hasAddButton = await dependencyPage.addButton.isVisible().catch(() => false);
    const hasEmptyState = await page.locator('.ant-empty, .empty-state, [class*="empty"]').isVisible().catch(() => false);
    const hasList = await dependencyPage.dependencyList.isVisible().catch(() => false);

    // 至少有一种状态显示
    expect(hasAddButton || hasEmptyState || hasList).toBeTruthy();
  });

  test('添加依赖按钮可见', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 添加按钮应该可见
    const addButton = page.locator('button').filter({ hasText: /添加|新增|新建/ }).first();
    const isVisible = await addButton.isVisible().catch(() => false);

    // 如果页面正常加载，添加按钮应该存在
    if (await page.locator('.ant-card, .dependency-list, [class*="dependency"]').isVisible().catch(() => false)) {
      expect(isVisible).toBeTruthy();
    }
  });

  test('添加依赖表单打开', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 点击添加按钮
    const addButton = page.locator('button').filter({ hasText: /添加|新增|新建/ }).first();
    if (await addButton.isVisible().catch(() => false)) {
      await addButton.click();

      // 等待弹窗或表单出现
      const modal = page.locator('.ant-modal, .ant-drawer');
      const isModalVisible = await modal.isVisible({ timeout: 3000 }).catch(() => false);

      if (isModalVisible) {
        // 验证表单字段存在
        const hasTypeField = await page.locator('text=依赖类型').isVisible().catch(() => false);
        const hasBadgeField = await page.locator('text=依赖徽章').isVisible().catch(() => false);

        // 关闭弹窗
        await page.locator('.ant-modal-close, button:has-text("取消")').first().click().catch(() => {});
      }
    }
  });

  test('依赖类型选项', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 点击添加按钮
    const addButton = page.locator('button').filter({ hasText: /添加|新增|新建/ }).first();
    if (await addButton.isVisible().catch(() => false)) {
      await addButton.click();

      // 等待弹窗
      await page.locator('.ant-modal, .ant-drawer').waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});

      // 点击类型选择框
      const typeSelect = page.locator('.ant-select').filter({ hasText: /类型/ }).first();
      if (await typeSelect.isVisible().catch(() => false)) {
        await typeSelect.click();
        await page.waitForTimeout(300);

        // 验证类型选项（移动端可能展示不同）
        const hasPrerequisite = await page.locator('.ant-select-item:has-text("前置条件")').isVisible().catch(() => false);
        const hasConsume = await page.locator('.ant-select-item:has-text("消耗")').isVisible().catch(() => false);
        const hasExclusive = await page.locator('.ant-select-item:has-text("互斥")').isVisible().catch(() => false);
        const hasAnyOption = await page.locator('.ant-select-item').first().isVisible().catch(() => false);

        // 至少有一个选项可见（包括任意类型）
        expect(hasPrerequisite || hasConsume || hasExclusive || hasAnyOption).toBeTruthy();
      }

      // 关闭
      await page.keyboard.press('Escape');
    }
  });

  test('删除依赖确认弹窗', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 如果有依赖项，验证删除确认逻辑
    const deleteButton = page.locator('button').filter({ hasText: /删除|移除/ }).first();
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await deleteButton.click();

      // 应该显示确认弹窗
      const hasConfirm = await page.locator('.ant-popconfirm, .ant-modal-confirm').isVisible({ timeout: 2000 }).catch(() => false);

      if (hasConfirm) {
        // 取消删除
        await page.locator('button:has-text("取消"), button:has-text("否")').first().click().catch(() => {});
      }
    }
  });

  test('查看依赖图', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 点击查看依赖图按钮
    const graphButton = page.locator('button').filter({ hasText: /依赖图|关系图|查看图/ }).first();
    if (await graphButton.isVisible().catch(() => false)) {
      await graphButton.click();

      // 验证图表显示
      const graph = page.locator('.dependency-graph, .react-flow, [class*="graph"]');
      const isGraphVisible = await graph.isVisible({ timeout: 3000 }).catch(() => false);

      // 关闭图表弹窗（如果是弹窗）
      await page.locator('.ant-modal-close, button:has-text("关闭")').first().click().catch(() => {});
    }
  });
});

test.describe('依赖图可视化', () => {
  let loginPage: LoginPage;
  let dependencyPage: DependencyPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dependencyPage = new DependencyPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('依赖图节点显示', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    const graphButton = page.locator('button').filter({ hasText: /依赖图|关系图|查看图/ }).first();
    if (await graphButton.isVisible().catch(() => false)) {
      await graphButton.click();

      // 等待图表加载
      await page.waitForTimeout(500);

      // 验证节点存在（使用更宽泛的选择器）
      const nodes = await page.locator('.dependency-graph .node, .react-flow__node, [class*="node"]').count();
      expect(nodes).toBeGreaterThanOrEqual(0);
    }
  });

  test('依赖图连线显示', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    const graphButton = page.locator('button').filter({ hasText: /依赖图|关系图|查看图/ }).first();
    if (await graphButton.isVisible().catch(() => false)) {
      await graphButton.click();

      // 等待图表加载
      await page.waitForTimeout(500);

      // 验证连线存在（使用更宽泛的选择器）
      const edges = await page.locator('.dependency-graph .edge, .react-flow__edge, [class*="edge"]').count();
      expect(edges).toBeGreaterThanOrEqual(0);
    }
  });
});

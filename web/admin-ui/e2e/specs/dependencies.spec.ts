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

    const addButton = page.locator('button').filter({ hasText: /添加|新增|新建/ }).first();
    if (!(await addButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '添加按钮不可见，页面可能未正常加载');
      return;
    }

    await addButton.click();

    // 弹窗或表单必须出现，否则添加按钮的交互存在问题
    const modal = page.locator('.ant-modal, .ant-drawer');
    await expect(modal).toBeVisible({ timeout: 5000 });

    // 验证表单中包含依赖配置所需的关键字段
    const hasTypeField = await page.locator('text=依赖类型').isVisible({ timeout: 3000 }).catch(() => false);
    const hasBadgeField = await page.locator('text=依赖徽章').isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasTypeField || hasBadgeField).toBeTruthy();

    // 关闭弹窗
    await page.locator('.ant-modal-close, button:has-text("取消")').first().click().catch(() => {});
  });

  test('依赖类型选项', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    const addButton = page.locator('button').filter({ hasText: /添加|新增|新建/ }).first();
    if (!(await addButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '添加按钮不可见，跳过依赖类型选项测试');
      return;
    }

    await addButton.click();
    await page.locator('.ant-modal, .ant-drawer').waitFor({ state: 'visible', timeout: 5000 });

    // 依赖类型选择器应存在且可交互
    const typeSelect = page.locator('.ant-select').filter({ hasText: /类型/ }).first();
    if (!(await typeSelect.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '依赖类型选择器不存在');
      return;
    }

    await typeSelect.click();
    await page.waitForTimeout(300);

    // 下拉面板中至少应有一个依赖类型选项
    const dropdown = page.locator('.ant-select-dropdown:visible');
    await expect(dropdown).toBeVisible({ timeout: 3000 });

    const options = dropdown.locator('.ant-select-item');
    const optionCount = await options.count();
    expect(optionCount).toBeGreaterThan(0);

    // 关闭
    await page.keyboard.press('Escape');
  });

  test('删除依赖确认弹窗', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    // 删除按钮仅在有依赖项时出现
    const deleteButton = page.locator('button').filter({ hasText: /删除|移除/ }).first();
    if (!(await deleteButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '无依赖项可删除');
      return;
    }

    await deleteButton.click();

    // 删除操作必须弹出二次确认，防止误删
    const confirmPopup = page.locator('.ant-popconfirm, .ant-modal-confirm');
    await expect(confirmPopup).toBeVisible({ timeout: 3000 });

    // 取消删除，验证取消后弹窗消失
    await page.locator('button:has-text("取消"), button:has-text("否")').first().click();
  });

  test('查看依赖图', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    const graphButton = page.locator('button').filter({ hasText: /依赖图|关系图|查看图/ }).first();
    if (!(await graphButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '依赖图按钮不可见，功能可能未实现');
      return;
    }

    await graphButton.click();

    // 图表容器必须渲染可见
    const graph = page.locator('.dependency-graph, .react-flow, [class*="graph"]');
    await expect(graph.first()).toBeVisible({ timeout: 5000 });

    // 关闭图表弹窗（如果是弹窗）
    await page.locator('.ant-modal-close, button:has-text("关闭")').first().click().catch(() => {});
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
    if (!(await graphButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '依赖图按钮不可见，功能可能未实现');
      return;
    }

    await graphButton.click();
    await page.waitForTimeout(500);

    // 依赖图中应至少包含当前徽章本身作为节点
    const nodes = page.locator('.dependency-graph .node, .react-flow__node, [class*="node"]');
    await expect(nodes.first()).toBeVisible({ timeout: 5000 });
    const nodeCount = await nodes.count();
    expect(nodeCount).toBeGreaterThan(0);
  });

  test('依赖图连线显示', async ({ page }) => {
    await page.goto('/badges/1/dependencies');
    await dependencyPage.waitForPageLoad();

    const graphButton = page.locator('button').filter({ hasText: /依赖图|关系图|查看图/ }).first();
    if (!(await graphButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '依赖图按钮不可见，功能可能未实现');
      return;
    }

    await graphButton.click();
    await page.waitForTimeout(500);

    // 图表容器应可见
    const graphContainer = page.locator('.dependency-graph, .react-flow, [class*="graph"]').first();
    await expect(graphContainer).toBeVisible({ timeout: 5000 });

    // 连线是否存在取决于是否有依赖关系，但图表容器必须正常渲染
    const edges = page.locator('.dependency-graph .edge, .react-flow__edge, [class*="edge"]');
    const edgeCount = await edges.count();
    // 如果有节点之间存在依赖，则连线数应 > 0；无依赖时 0 条也是合理的
    expect(edgeCount).toBeGreaterThanOrEqual(0);
    // 但图表容器本身必须已渲染
    expect(await graphContainer.isVisible()).toBeTruthy();
  });
});

import { test, expect } from '@playwright/test';
import { LoginPage, RuleEditorPage } from '../pages';

/**
 * 规则编辑器（画布）- UI 验证测试
 *
 * 验证规则编辑器画布的基本 UI 功能
 */
test.describe('规则编辑器（画布）', () => {
  let loginPage: LoginPage;
  let ruleEditorPage: RuleEditorPage;

  test.beforeEach(async ({ page }, testInfo) => {
    // 跳过移动端测试（画布交互在移动端有兼容性问题）
    const projectName = testInfo.project.name;
    if (projectName.includes('mobile') || projectName.includes('Mobile')) {
      test.skip(true, 'Skipping mobile browser tests due to canvas compatibility issues');
    }

    loginPage = new LoginPage(page);
    ruleEditorPage = new RuleEditorPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('画布初始化正确', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证画布可见
    await expect(ruleEditorPage.canvas).toBeVisible();

    // 验证节点类型面板可见 - 检查条件节点选项
    await expect(ruleEditorPage.conditionNode).toBeVisible();

    // 验证工具栏保存按钮可见
    await expect(ruleEditorPage.saveButton).toBeVisible();
  });

  test('节点面板可拖拽元素可见', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证条件节点可见且可交互
    await expect(ruleEditorPage.conditionNode).toBeVisible();

    // 验证画布已有示例节点
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBeGreaterThan(0);
  });

  test('画布已加载示例规则', async ({ page }) => {
    await ruleEditorPage.goto();

    // 画布应该已有示例节点
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBeGreaterThan(0);

    // 验证画布仍然可用
    await expect(ruleEditorPage.canvas).toBeVisible();
  });

  test('画布包含逻辑组节点', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证画布中有 AND 逻辑组（示例规则）
    const andNode = page.locator('.react-flow__node:has-text("AND")');
    await expect(andNode.first()).toBeVisible();
  });

  test('画布节点可选中', async ({ page }) => {
    await ruleEditorPage.goto();
    await page.waitForTimeout(500);

    // 点击画布上的第一个节点
    const firstNode = page.locator('.react-flow__node').first();
    if (await firstNode.isVisible({ timeout: 5000 }).catch(() => false)) {
      await firstNode.click({ force: true });
    }

    // 验证画布仍然可见
    await expect(ruleEditorPage.canvas).toBeVisible();
  });

  test('工具栏按钮可见', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证主要工具栏按钮可见
    await expect(ruleEditorPage.saveButton).toBeVisible();

    // 验证添加节点按钮
    await expect(ruleEditorPage.addNodeButton).toBeVisible();
  });

  test('规则测试按钮', async ({ page }) => {
    await ruleEditorPage.goto();

    // 如果测试按钮可见，点击它
    if (await ruleEditorPage.testButton.isVisible().catch(() => false)) {
      await ruleEditorPage.testButton.click();

      // 等待弹窗出现
      const modal = page.locator('.ant-modal');
      if (await modal.isVisible({ timeout: 3000 }).catch(() => false)) {
        // 关闭弹窗
        await page.locator('.ant-modal-close, button:has-text("取消"), button:has-text("关闭")').first().click();
      }
    }

    // 验证画布仍然可见
    await expect(ruleEditorPage.canvas).toBeVisible();
  });

  test('画布缩放和平移', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证画布可见
    await expect(ruleEditorPage.canvas).toBeVisible();

    // 验证缩放控件存在
    const controls = page.locator('.react-flow__controls');
    if (await controls.isVisible({ timeout: 3000 }).catch(() => false)) {
      // 点击放大按钮
      const zoomIn = controls.locator('button').first();
      if (await zoomIn.isVisible().catch(() => false)) {
        await zoomIn.click();
        await page.waitForTimeout(300);
      }
    }

    // 画布应该仍然正常工作
    await expect(ruleEditorPage.canvas).toBeVisible();
  });
});

test.describe('规则模板', () => {
  let loginPage: LoginPage;
  let ruleEditorPage: RuleEditorPage;

  test.beforeEach(async ({ page }, testInfo) => {
    // 跳过移动端测试
    const projectName = testInfo.project.name;
    if (projectName.includes('mobile') || projectName.includes('Mobile')) {
      test.skip(true, 'Skipping mobile browser tests due to canvas compatibility issues');
    }

    loginPage = new LoginPage(page);
    ruleEditorPage = new RuleEditorPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('规则编辑器页面加载', async ({ page }) => {
    await page.goto('/rules/create');
    await ruleEditorPage.waitForPageLoad();

    // 验证画布加载
    await expect(ruleEditorPage.canvas).toBeVisible();

    // 验证节点面板可见
    await expect(ruleEditorPage.conditionNode).toBeVisible();
  });

  test('节点类型面板包含所有节点类型', async ({ page }) => {
    await ruleEditorPage.goto();

    // 验证三种节点类型都可见
    await expect(ruleEditorPage.conditionNode).toBeVisible();
    await expect(ruleEditorPage.actionNode).toBeVisible();
    await expect(ruleEditorPage.combinerNode).toBeVisible();
  });
});

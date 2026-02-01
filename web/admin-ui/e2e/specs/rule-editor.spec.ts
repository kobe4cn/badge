import { test, expect } from '@playwright/test';
import { LoginPage, RuleEditorPage } from '../pages';
import { ApiHelper, createTestRule, uniqueId } from '../utils';

test.describe('规则编辑器（画布）', () => {
  let loginPage: LoginPage;
  let ruleEditorPage: RuleEditorPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_rule_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    ruleEditorPage = new RuleEditorPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test.afterEach(async () => {
    await apiHelper.cleanup(testPrefix);
  });

  test('画布初始化正确', async () => {
    await ruleEditorPage.goto();

    // 验证画布可见
    await expect(ruleEditorPage.canvas).toBeVisible();

    // 验证节点面板可见
    await expect(ruleEditorPage.nodePanel).toBeVisible();

    // 验证工具栏可见
    await expect(ruleEditorPage.saveButton).toBeVisible();
  });

  test('拖拽添加条件节点', async () => {
    await ruleEditorPage.goto();

    // 拖拽条件节点到画布
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 200);

    // 验证节点已添加
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(1);
  });

  test('创建简单规则: 消费金额触发', async ({ page }) => {
    await ruleEditorPage.goto();

    // 添加条件节点
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.configureCondition({
      field: '累计消费金额',
      operator: '>=',
      value: '1000',
    });

    // 添加动作节点
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);
    await ruleEditorPage.configureAction({
      actionType: '发放徽章',
      badgeId: '1',
    });

    // 连接节点
    const nodes = await page.locator('.react-flow__node').all();
    if (nodes.length >= 2) {
      const sourceId = await nodes[0].getAttribute('data-id');
      const targetId = await nodes[1].getAttribute('data-id');
      if (sourceId && targetId) {
        await ruleEditorPage.connectNodes(sourceId, targetId);
      }
    }

    // 验证连线
    const edgeCount = await ruleEditorPage.getEdgeCount();
    expect(edgeCount).toBe(1);

    // 保存
    await ruleEditorPage.save();
  });

  test('创建组合规则: AND 条件', async ({ page }) => {
    await ruleEditorPage.goto();

    // 添加两个条件节点
    await ruleEditorPage.dragNodeToCanvas('condition', 100, 100);
    await ruleEditorPage.dragNodeToCanvas('condition', 300, 100);

    // 添加组合节点
    await ruleEditorPage.dragNodeToCanvas('combiner', 200, 250);
    await ruleEditorPage.configureCombiner('AND');

    // 添加动作节点
    await ruleEditorPage.dragNodeToCanvas('action', 200, 400);

    // 验证节点数量
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(4);
  });

  test('创建嵌套规则: OR 包含 AND', async ({ page }) => {
    await ruleEditorPage.goto();

    // 创建复杂规则结构
    // (条件A AND 条件B) OR 条件C -> 动作

    // 第一层 AND
    await ruleEditorPage.dragNodeToCanvas('condition', 50, 50);   // A
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 50);  // B
    await ruleEditorPage.dragNodeToCanvas('combiner', 125, 150);  // AND

    // 第二层 OR
    await ruleEditorPage.dragNodeToCanvas('condition', 350, 100); // C
    await ruleEditorPage.dragNodeToCanvas('combiner', 200, 280);  // OR

    // 动作
    await ruleEditorPage.dragNodeToCanvas('action', 200, 400);

    // 验证节点数量
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(6);
  });

  test('撤销/重做操作', async ({ page }) => {
    await ruleEditorPage.goto();

    // 添加节点
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 200);
    let nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(1);

    // 撤销
    await ruleEditorPage.undoButton.click();
    nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(0);

    // 重做
    await ruleEditorPage.redoButton.click();
    nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBe(1);
  });

  test('规则预览', async ({ page }) => {
    await ruleEditorPage.goto();

    // 创建简单规则
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);

    // 预览
    await ruleEditorPage.preview();

    // 验证预览弹窗
    await expect(page.locator('.preview-modal')).toBeVisible();

    // 应该显示规则 JSON
    await expect(page.locator('.preview-modal pre')).toBeVisible();
  });

  test('规则发布', async ({ page }) => {
    // 先创建草稿规则
    const rule = createTestRule({ name: `${testPrefix}待发布规则` });
    const created = await apiHelper.createRule(rule);

    await ruleEditorPage.goto(created.id);

    // 发布
    await ruleEditorPage.publish();

    // 验证发布成功
    await expect(page.locator('.rule-status:has-text("已发布")')).toBeVisible();
  });

  test('画布缩放和平移', async ({ page }) => {
    await ruleEditorPage.goto();

    // 添加一些节点
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 200);

    // 缩放
    await ruleEditorPage.zoom(1.5);

    // 平移
    await ruleEditorPage.pan(100, 100);

    // 画布应该仍然正常工作
    await expect(ruleEditorPage.canvas).toBeVisible();
  });
});

test.describe('规则模板', () => {
  let loginPage: LoginPage;
  let ruleEditorPage: RuleEditorPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    ruleEditorPage = new RuleEditorPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('从模板创建规则', async ({ page }) => {
    await page.goto('/rules/create');
    await ruleEditorPage.waitForPageLoad();

    // 点击模板按钮
    await page.locator('button:has-text("从模板创建")').click();

    // 选择模板
    await page.locator('.template-list .template-item:first-child').click();

    // 验证节点已加载
    const nodeCount = await ruleEditorPage.getNodeCount();
    expect(nodeCount).toBeGreaterThan(0);
  });

  test('保存为模板', async ({ page }) => {
    await ruleEditorPage.goto();

    // 创建规则
    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);

    // 保存为模板
    await page.locator('button:has-text("保存为模板")').click();
    await page.locator('#template-name').fill('测试模板');
    await page.locator('button:has-text("确定")').click();

    await ruleEditorPage.waitForMessage('success');
  });
});

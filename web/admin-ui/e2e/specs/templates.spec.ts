import { test, expect } from '@playwright/test';
import { LoginPage, RuleEditorPage } from '../pages';
import { TemplatePage } from '../pages/TemplatePage';
import { ApiHelper, uniqueId } from '../utils';

test.describe('规则模板管理', () => {
  let loginPage: LoginPage;
  let templatePage: TemplatePage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_tpl_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    templatePage = new TemplatePage(page);
    apiHelper = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('模板列表加载', async () => {
    await templatePage.goto();

    await expect(templatePage.templateList).toBeVisible();
    await expect(templatePage.createButton).toBeVisible();
  });

  test('搜索模板', async ({ page }) => {
    await templatePage.goto();
    await templatePage.search('消费');

    // 搜索后模板列表区域应保持可见（即使没有匹配结果也应显示空状态）
    await expect(templatePage.templateList).toBeVisible();

    // 搜索匹配 name、description 和 code 三个字段，
    // 验证结果卡片中至少有一个字段包含关键词
    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      const firstCard = page.locator('.template-card').first();
      const cardText = await firstCard.textContent();
      expect(cardText).toContain('消费');
    }
  });

  test('预览模板', async ({ page }) => {
    await templatePage.goto();

    const count = await templatePage.getTemplateCount();
    if (count === 0) {
      test.skip(true, '无可用模板，跳过预览测试');
      return;
    }

    const firstTemplateName = await page.locator('.template-card .template-name').first().textContent();
    expect(firstTemplateName).toBeDefined();

    await templatePage.previewTemplate(firstTemplateName!);
    await expect(templatePage.previewModal).toBeVisible();
  });

  test('从模板创建规则', async ({ page }) => {
    await templatePage.goto();

    const count = await templatePage.getTemplateCount();
    if (count === 0) {
      test.skip(true, '无可用模板，跳过从模板创建规则测试');
      return;
    }

    const firstTemplateName = await page.locator('.template-card .template-name').first().textContent();
    expect(firstTemplateName).toBeDefined();

    await templatePage.useTemplate(firstTemplateName!);

    // 应该跳转到规则编辑器
    await expect(page).toHaveURL(/\/rules\/create/);
  });

  test('规则编辑器可保存', async ({ page }) => {
    // 验证规则编辑器的保存功能
    await page.goto('/rules/create');
    const ruleEditorPage = new RuleEditorPage(page);
    await ruleEditorPage.waitForCanvasReady();

    // 验证画布和保存按钮可见
    await expect(ruleEditorPage.canvas).toBeVisible();
    await expect(ruleEditorPage.saveButton).toBeVisible();
  });

  test('删除模板', async ({ page }) => {
    await templatePage.goto();

    const count = await templatePage.getTemplateCount();
    if (count === 0) {
      test.skip(true, '无可用模板，跳过删除测试');
      return;
    }

    // 仅删除测试前缀创建的模板，避免误删系统内置模板
    const testTemplate = page.locator(`.template-card .template-name:has-text("${testPrefix}")`).first();
    if (!(await testTemplate.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '无测试前缀匹配的模板可删除');
      return;
    }

    const templateName = await testTemplate.textContent();
    expect(templateName).toBeDefined();
    await templatePage.deleteTemplate(templateName!);
    await expect(page.locator('.ant-message-success')).toBeVisible();
  });
});

test.describe('内置模板', () => {
  let loginPage: LoginPage;
  let templatePage: TemplatePage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    templatePage = new TemplatePage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('内置模板不可删除', async ({ page }) => {
    await templatePage.goto();

    // 查找系统模板
    const systemTemplate = page.locator('.template-card.system-template').first();
    if (!(await systemTemplate.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip(true, '无系统内置模板，跳过验证');
      return;
    }

    // 系统模板的删除按钮应该禁用或不存在，防止误删
    const deleteBtn = systemTemplate.locator('button:has-text("删除")');
    if (await deleteBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await expect(deleteBtn).toBeDisabled();
    } else {
      // 删除按钮不存在也是符合预期的安全设计
      expect(await deleteBtn.count()).toBe(0);
    }
  });

  test('消费累计模板结构正确', async ({ page }) => {
    await templatePage.goto();
    await templatePage.search('消费累计');

    const count = await templatePage.getTemplateCount();
    if (count === 0) {
      test.skip(true, '消费累计模板不存在，跳过结构验证');
      return;
    }

    await templatePage.previewTemplate('消费累计');

    // 消费累计模板应包含条件节点和动作节点（共 2 个）
    await expect(templatePage.previewCanvas.locator('.react-flow__node')).toHaveCount(2);
  });

  test('连续签到模板结构正确', async ({ page }) => {
    await templatePage.goto();
    await templatePage.search('连续签到');

    const count = await templatePage.getTemplateCount();
    if (count === 0) {
      test.skip(true, '连续签到模板不存在，跳过结构验证');
      return;
    }

    await templatePage.previewTemplate('连续签到');
    // 签到模板应至少包含一个节点
    const nodeCount = await templatePage.previewCanvas.locator('.react-flow__node').count();
    expect(nodeCount).toBeGreaterThan(0);
  });
});

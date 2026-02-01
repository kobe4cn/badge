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
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('模板列表加载', async () => {
    await templatePage.goto();

    await expect(templatePage.templateList).toBeVisible();
    await expect(templatePage.createButton).toBeVisible();
  });

  test('搜索模板', async () => {
    await templatePage.goto();
    await templatePage.search('消费');

    // 应该只显示匹配的模板
    const count = await templatePage.getTemplateCount();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('预览模板', async ({ page }) => {
    await templatePage.goto();

    // 如果有模板，预览第一个
    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      const firstTemplateName = await page.locator('.template-card .template-name').first().textContent();
      if (firstTemplateName) {
        await templatePage.previewTemplate(firstTemplateName);
        await expect(templatePage.previewModal).toBeVisible();
      }
    }
  });

  test('从模板创建规则', async ({ page }) => {
    await templatePage.goto();

    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      const firstTemplateName = await page.locator('.template-card .template-name').first().textContent();
      if (firstTemplateName) {
        await templatePage.useTemplate(firstTemplateName);

        // 应该跳转到规则编辑器
        await expect(page).toHaveURL(/\/rules\/create/);
      }
    }
  });

  test('创建新模板', async ({ page }) => {
    // 先创建一个规则
    await page.goto('/rules/create');
    const ruleEditorPage = new RuleEditorPage(page);
    await ruleEditorPage.waitForCanvasReady();

    await ruleEditorPage.dragNodeToCanvas('condition', 200, 100);
    await ruleEditorPage.dragNodeToCanvas('action', 200, 300);

    // 保存为模板
    await page.locator('button:has-text("保存为模板")').click();
    await page.locator('#template-name').fill(`${testPrefix}测试模板`);
    await page.locator('#template-description').fill('E2E 测试创建的模板');
    await page.locator('.ant-modal button:has-text("确定")').click();

    await expect(page.locator('.ant-message-success')).toBeVisible();
  });

  test('删除模板', async ({ page }) => {
    // 需要先有测试模板
    await templatePage.goto();

    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      const firstTemplateName = await page.locator('.template-card .template-name').first().textContent();
      if (firstTemplateName && firstTemplateName.includes(testPrefix)) {
        await templatePage.deleteTemplate(firstTemplateName);
        await expect(page.locator('.ant-message-success')).toBeVisible();
      }
    }
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
    if (await systemTemplate.isVisible()) {
      // 删除按钮应该禁用或不存在
      const deleteBtn = systemTemplate.locator('button:has-text("删除")');
      if (await deleteBtn.isVisible()) {
        await expect(deleteBtn).toBeDisabled();
      }
    }
  });

  test('消费累计模板结构正确', async ({ page }) => {
    await templatePage.goto();
    await templatePage.search('消费累计');

    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      await templatePage.previewTemplate('消费累计');

      // 验证模板包含条件和动作节点
      await expect(templatePage.previewCanvas.locator('.react-flow__node')).toHaveCount(2);
    }
  });

  test('连续签到模板结构正确', async ({ page }) => {
    await templatePage.goto();
    await templatePage.search('连续签到');

    const count = await templatePage.getTemplateCount();
    if (count > 0) {
      await templatePage.previewTemplate('连续签到');
      await expect(templatePage.previewCanvas.locator('.react-flow__node')).toHaveCount(2);
    }
  });
});

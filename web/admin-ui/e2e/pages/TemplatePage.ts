import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 规则模板管理页面对象
 */
export class TemplatePage extends BasePage {
  // 列表元素
  readonly templateList = this.page.locator('.template-list');
  readonly templateCard = this.page.locator('.template-card');
  readonly createButton = this.page.locator('button:has-text("创建模板")');
  readonly searchInput = this.page.locator('input[placeholder*="搜索"]');

  // 表单元素
  readonly nameInput = this.page.locator('#name');
  readonly descriptionInput = this.page.locator('#description');
  readonly categorySelect = this.page.locator('.ant-select:has-text("分类")');
  readonly submitButton = this.page.locator('button:has-text("提交")');

  // 预览
  readonly previewModal = this.page.locator('.template-preview-modal');
  readonly previewCanvas = this.page.locator('.template-preview-modal .react-flow');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/templates');
    await this.waitForPageLoad();
  }

  async search(keyword: string): Promise<void> {
    await this.searchInput.fill(keyword);
    await this.page.keyboard.press('Enter');
    await this.waitForLoading();
  }

  async clickCreate(): Promise<void> {
    await this.createButton.click();
    await this.waitForPageLoad();
  }

  async selectTemplate(name: string): Promise<void> {
    await this.page.locator(`.template-card:has-text("${name}")`).click();
  }

  async previewTemplate(name: string): Promise<void> {
    await this.page.locator(`.template-card:has-text("${name}") button:has-text("预览")`).click();
    await this.previewModal.waitFor({ state: 'visible' });
  }

  async useTemplate(name: string): Promise<void> {
    await this.page.locator(`.template-card:has-text("${name}") button:has-text("使用")`).click();
  }

  async deleteTemplate(name: string): Promise<void> {
    await this.page.locator(`.template-card:has-text("${name}") button:has-text("删除")`).click();
    await this.confirmModal();
  }

  async getTemplateCount(): Promise<number> {
    return await this.templateCard.count();
  }

  async expectTemplateExists(name: string): Promise<void> {
    await expect(this.page.locator(`.template-card:has-text("${name}")`)).toBeVisible();
  }

  async expectTemplateNotExists(name: string): Promise<void> {
    await expect(this.page.locator(`.template-card:has-text("${name}")`)).not.toBeVisible();
  }
}

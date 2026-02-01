import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章分类管理页面对象
 */
export class CategoryPage extends BasePage {
  readonly table = this.page.locator('.ant-table');
  readonly createButton = this.page.locator('button:has-text("新建分类")');
  readonly searchInput = this.page.locator('input[placeholder*="搜索"]');

  readonly nameInput = this.page.locator('#name');
  readonly displayNameInput = this.page.locator('#display_name');
  readonly descriptionInput = this.page.locator('#description');
  readonly iconUpload = this.page.locator('.ant-upload');
  readonly sortInput = this.page.locator('#sort_order');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/categories');
    await this.waitForPageLoad();
  }

  async clickCreate(): Promise<void> {
    await this.createButton.click();
    await this.waitForPageLoad();
  }

  async search(keyword: string): Promise<void> {
    await this.searchInput.fill(keyword);
    await this.page.keyboard.press('Enter');
    await this.waitForLoading();
  }

  async clickEdit(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("编辑")`).click();
    await this.waitForPageLoad();
  }

  async clickDelete(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("删除")`).click();
  }

  async getCategoryCount(): Promise<number> {
    return await this.page.locator('.ant-table-row').count();
  }

  async expectCategoryExists(name: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${name}")`)).toBeVisible();
  }

  async expectCategoryNotExists(name: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${name}")`)).not.toBeVisible();
  }
}

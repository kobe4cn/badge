import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章系列管理页面对象
 */
export class SeriesPage extends BasePage {
  readonly table = this.page.locator('.ant-table');
  readonly createButton = this.page.locator('button:has-text("新建系列")');
  readonly categoryFilter = this.page.locator('.ant-select:has-text("分类")');

  readonly nameInput = this.page.locator('#name');
  readonly displayNameInput = this.page.locator('#display_name');
  readonly descriptionInput = this.page.locator('#description');
  readonly categorySelect = this.page.locator('#category_id');
  readonly themeSelect = this.page.locator('#theme');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/series');
    await this.waitForPageLoad();
  }

  async clickCreate(): Promise<void> {
    await this.createButton.click();
    await this.waitForPageLoad();
  }

  async filterByCategory(categoryName: string): Promise<void> {
    await this.categoryFilter.click();
    await this.page.locator(`.ant-select-item:has-text("${categoryName}")`).click();
    await this.waitForLoading();
  }

  async clickEdit(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("编辑")`).click();
    await this.waitForPageLoad();
  }

  async clickDelete(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("删除")`).click();
  }

  async getSeriesCount(): Promise<number> {
    return await this.page.locator('.ant-table-row').count();
  }

  async expectSeriesExists(name: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${name}")`)).toBeVisible();
  }
}

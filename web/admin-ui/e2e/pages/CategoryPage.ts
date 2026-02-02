import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章分类管理页面对象
 */
export class CategoryPage extends BasePage {
  readonly table = this.page.locator('.ant-table');
  readonly createButton = this.page.locator('button:has-text("新建分类")');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/badges/categories');
    await this.waitForPageLoad();
    await this.waitForLoading();
  }

  async clickCreate(): Promise<void> {
    await this.createButton.click();
    // 等待抽屉或模态框打开
    await this.page.locator('.ant-drawer, .ant-modal').waitFor({ state: 'visible', timeout: 10000 });
  }

  /**
   * 搜索分类 - 使用表单中的名称输入框和查询按钮
   */
  async search(keyword: string): Promise<void> {
    // 使用页面搜索表单（非抽屉/模态框）
    const searchInput = this.page.locator('.ant-pro-form').locator('.ant-form-item').filter({ hasText: '名称' }).locator('input');
    await searchInput.fill(keyword);
    await this.clickButton('查询');
    await this.waitForLoading();
  }

  async clickEdit(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await row.getByRole('button', { name: /编辑/ }).click();
    await this.page.locator('.ant-drawer, .ant-modal').waitFor({ state: 'visible', timeout: 10000 });
  }

  async clickDelete(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await row.getByRole('button', { name: /删/ }).click();
    // 等待 Popconfirm 出现
    await this.page.locator('.ant-popconfirm, .ant-popover').waitFor({ state: 'visible', timeout: 5000 });
  }

  async getCategoryCount(): Promise<number> {
    const rows = this.page.locator('.ant-table-tbody tr[class*="ant-table-row"]');
    return await rows.count();
  }

  async expectCategoryExists(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await expect(row.first()).toBeVisible({ timeout: 10000 });
  }

  async expectCategoryNotExists(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await expect(row).toHaveCount(0, { timeout: 5000 });
  }
}

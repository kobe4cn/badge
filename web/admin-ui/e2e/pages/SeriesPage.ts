import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章系列管理页面对象
 */
export class SeriesPage extends BasePage {
  readonly table = this.page.locator('.ant-table');
  readonly createButton = this.page.locator('button:has-text("新建系列")');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/badges/series');
    await this.waitForPageLoad();
    await this.waitForLoading();
  }

  /**
   * 搜索系列 - 使用表单中的名称输入框和查询按钮
   */
  async search(keyword: string): Promise<void> {
    const searchInput = this.page.locator('.ant-pro-form').locator('.ant-form-item').filter({ hasText: '名称' }).locator('input');
    await searchInput.fill(keyword);
    await this.clickButton('查询');
    await this.waitForLoading();
  }

  async clickCreate(): Promise<void> {
    await this.createButton.click();
    // 等待抽屉或模态框打开
    await this.page.locator('.ant-drawer, .ant-modal').waitFor({ state: 'visible', timeout: 10000 });
  }

  /**
   * 按分类筛选 - 使用搜索表单
   */
  async filterByCategory(categoryName: string): Promise<void> {
    // 在搜索表单中找分类下拉框
    const categoryFormItem = this.page.locator('.ant-pro-form .ant-form-item').filter({ hasText: '分类' });
    await categoryFormItem.locator('.ant-select-selector').click();
    await this.page.locator('.ant-select-dropdown').waitFor({ state: 'visible' });
    await this.page.locator(`.ant-select-item-option:has-text("${categoryName}")`).first().click();
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

  async getSeriesCount(): Promise<number> {
    const rows = this.page.locator('.ant-table-tbody tr[class*="ant-table-row"]');
    return await rows.count();
  }

  async expectSeriesExists(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await expect(row.first()).toBeVisible({ timeout: 10000 });
  }

  async expectSeriesNotExists(name: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(name) });
    await expect(row).toHaveCount(0, { timeout: 5000 });
  }
}

import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章列表页面对象
 */
export class BadgeListPage extends BasePage {
  readonly createButton = this.page.locator('button:has-text("新建徽章")');
  readonly searchInput = this.page.locator('.ant-pro-table-search input[placeholder*="搜索"]');
  readonly table = this.page.locator('.ant-table');
  readonly tableRows = this.page.locator('.ant-table-tbody tr');
  readonly pagination = this.page.locator('.ant-pagination');

  constructor(page: Page) {
    super(page);
  }

  /**
   * 导航到徽章列表
   */
  async goto(): Promise<void> {
    await this.page.goto('/badges');
    await this.waitForPageLoad();
    await this.waitForLoading();
  }

  /**
   * 搜索徽章
   */
  async search(keyword: string): Promise<void> {
    await this.searchInput.fill(keyword);
    await this.page.keyboard.press('Enter');
    await this.waitForLoading();
  }

  /**
   * 点击新建徽章
   */
  async clickCreate(): Promise<void> {
    await this.createButton.click();
  }

  /**
   * 点击编辑按钮
   */
  async clickEdit(badgeName: string): Promise<void> {
    const row = this.page.locator(`tr:has-text("${badgeName}")`);
    await row.locator('button:has-text("编辑"), a:has-text("编辑")').click();
  }

  /**
   * 点击删除按钮
   */
  async clickDelete(badgeName: string): Promise<void> {
    const row = this.page.locator(`tr:has-text("${badgeName}")`);
    await row.locator('button:has-text("删除")').click();
  }

  /**
   * 获取徽章数量
   */
  async getBadgeCount(): Promise<number> {
    const rows = await this.tableRows.count();
    // 排除空状态行
    const emptyRow = this.page.locator('.ant-table-placeholder');
    if (await emptyRow.isVisible()) {
      return 0;
    }
    return rows;
  }

  /**
   * 验证徽章存在
   */
  async expectBadgeExists(badgeName: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${badgeName}")`)).toBeVisible();
  }

  /**
   * 验证徽章不存在
   */
  async expectBadgeNotExists(badgeName: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${badgeName}")`)).not.toBeVisible();
  }
}

import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章列表页面对象
 */
export class BadgeListPage extends BasePage {
  readonly createButton = this.page.locator('button:has-text("新建徽章")');
  readonly table = this.page.locator('.ant-table');
  readonly tableRows = this.page.locator('.ant-table-tbody tr[class*="ant-table-row"]');
  readonly pagination = this.page.locator('.ant-pagination');

  constructor(page: Page) {
    super(page);
  }

  /**
   * 导航到徽章列表
   */
  async goto(): Promise<void> {
    await this.page.goto('/badges/definitions');
    await this.waitForPageLoad();
    await this.waitForLoading();
  }

  /**
   * 搜索徽章 - 使用表单中的徽章名称输入框和查询按钮
   */
  async search(keyword: string): Promise<void> {
    // 找到搜索表单中的徽章名称输入框
    const searchInput = this.page.locator('.ant-form-item').filter({ hasText: '徽章名称' }).locator('input');
    await searchInput.fill(keyword);

    // 等待 API 响应完成
    const responsePromise = this.page.waitForResponse(
      (response) => response.url().includes('/api/admin/badges') && response.status() === 200,
      { timeout: 30000 }
    );

    // 点击查询按钮
    await this.clickButton('查询');

    // 等待 API 响应
    await responsePromise;

    // 等待表格加载完成
    await this.waitForLoading();

    // 额外等待一下确保数据刷新
    await this.page.waitForTimeout(500);
  }

  /**
   * 点击新建徽章
   */
  async clickCreate(): Promise<void> {
    await this.createButton.click();
    // 等待抽屉打开
    await this.page.locator('.ant-drawer').waitFor({ state: 'visible', timeout: 10000 });
  }

  /**
   * 点击编辑按钮
   */
  async clickEdit(badgeName: string): Promise<void> {
    const row = this.page.locator(`tr:has-text("${badgeName}")`);
    await row.locator('button:has-text("编辑"), a:has-text("编辑")').click();
  }

  /**
   * 点击删除按钮 - 删除在 more 下拉菜单中
   */
  async clickDelete(badgeName: string): Promise<void> {
    const row = this.page.locator(`tr:has-text("${badgeName}")`);
    // 点击 more 按钮打开下拉菜单
    await row.locator('button[class*="ant-btn"]').filter({ has: this.page.locator('span[aria-label="more"]') }).click();
    // 等待下拉菜单出现
    await this.page.locator('.ant-dropdown').waitFor({ state: 'visible' });
    // 点击删除选项
    await this.page.locator('.ant-dropdown-menu-item:has-text("删除")').click();
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
    // 使用 getByRole 匹配表格行，更可靠
    const row = this.page.getByRole('row', { name: new RegExp(badgeName) });
    await expect(row.first()).toBeVisible({ timeout: 15000 });
  }

  /**
   * 验证徽章不存在
   */
  async expectBadgeNotExists(badgeName: string): Promise<void> {
    const row = this.page.getByRole('row', { name: new RegExp(badgeName) });
    await expect(row).toHaveCount(0, { timeout: 5000 });
  }
}

import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 数据看板页面对象
 */
export class DashboardPage extends BasePage {
  // 统计卡片
  readonly totalBadgesCard = this.page.locator('.stat-card:has-text("徽章总数")');
  readonly activeBadgesCard = this.page.locator('.stat-card:has-text("生效徽章")');
  readonly totalGrantsCard = this.page.locator('.stat-card:has-text("发放次数")');
  readonly activeUsersCard = this.page.locator('.stat-card:has-text("活跃用户")');

  // 图表
  readonly grantTrendChart = this.page.locator('.chart-container:has-text("发放趋势")');
  readonly categoryDistChart = this.page.locator('.chart-container:has-text("分类分布")');
  readonly topBadgesChart = this.page.locator('.chart-container:has-text("热门徽章")');

  // 筛选器
  readonly dateRangePicker = this.page.locator('.ant-picker-range');
  readonly categoryFilter = this.page.locator('.filter-category');
  readonly refreshButton = this.page.locator('button:has-text("刷新")');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/dashboard');
    await this.waitForPageLoad();
  }

  async selectDateRange(start: string, end: string): Promise<void> {
    await this.dateRangePicker.click();
    await this.page.locator('.ant-picker-cell-inner').filter({ hasText: start }).first().click();
    await this.page.locator('.ant-picker-cell-inner').filter({ hasText: end }).first().click();
  }

  async getStatValue(cardName: string): Promise<string> {
    const card = this.page.locator(`.stat-card:has-text("${cardName}") .stat-value`);
    return await card.textContent() || '0';
  }

  async waitForChartsLoad(): Promise<void> {
    await this.grantTrendChart.locator('canvas, svg').waitFor({ state: 'visible' });
  }
}

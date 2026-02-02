import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 数据看板页面对象
 *
 * 统计卡片标题映射：
 * - 总发放数 (totalGrantsCard)
 * - 活跃徽章 (activeBadgesCard)
 * - 持有用户 (badgeHoldersCard)
 * - 用户覆盖率 (coverageCard)
 * - 今日发放 (todayGrantsCard)
 * - 新增持有者 (newHoldersCard)
 * - 今日兑换 (todayRedemptionsCard)
 */
export class DashboardPage extends BasePage {
  // 今日统计卡片
  readonly todayGrantsCard = this.page.locator('.stat-card:has-text("今日发放")');
  readonly newHoldersCard = this.page.locator('.stat-card:has-text("新增持有者")');
  readonly todayRedemptionsCard = this.page.locator('.stat-card:has-text("今日兑换")');

  // 总量统计卡片
  readonly totalGrantsCard = this.page.locator('.stat-card:has-text("总发放数")');
  readonly activeBadgesCard = this.page.locator('.stat-card:has-text("活跃徽章")');
  readonly badgeHoldersCard = this.page.locator('.stat-card:has-text("持有用户")');
  readonly coverageCard = this.page.locator('.stat-card:has-text("用户覆盖率")');

  // 兼容旧测试的别名
  readonly totalBadgesCard = this.totalGrantsCard;
  readonly activeUsersCard = this.badgeHoldersCard;

  // 图表（使用 Card title 定位）
  readonly grantTrendChart = this.page.locator('.ant-card:has-text("发放趋势")');
  readonly categoryDistChart = this.page.locator('.ant-card:has-text("徽章类型分布")');
  readonly topBadgesChart = this.page.locator('.ant-card:has-text("热门徽章")');

  // 筛选器
  readonly dateRangePicker = this.page.locator('.ant-segmented');
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
    // 统计卡片的值在 .ant-statistic-content-value 或直接在卡片内的大字体数字
    const card = this.page.locator(`.stat-card:has-text("${cardName}")`);
    const statValue = card.locator('.ant-statistic-content-value-int, .ant-statistic-content-value');
    const directValue = card.locator('div[style*="font-size: 28px"], div[style*="fontSize: 28"]');

    if (await statValue.isVisible()) {
      return await statValue.textContent() || '0';
    }
    if (await directValue.isVisible()) {
      return await directValue.textContent() || '0';
    }
    return '0';
  }

  async waitForChartsLoad(): Promise<void> {
    // 等待任一图表加载完成
    await this.grantTrendChart.waitFor({ state: 'visible' });
  }

  async expectStatsVisible(): Promise<void> {
    // 验证今日统计卡片可见
    await expect(this.todayGrantsCard).toBeVisible();
    await expect(this.newHoldersCard).toBeVisible();
    await expect(this.todayRedemptionsCard).toBeVisible();

    // 验证总量统计卡片可见
    await expect(this.totalGrantsCard).toBeVisible();
    await expect(this.activeBadgesCard).toBeVisible();
    await expect(this.badgeHoldersCard).toBeVisible();
    await expect(this.coverageCard).toBeVisible();
  }
}

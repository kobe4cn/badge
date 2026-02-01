import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 权益管理页面对象
 *
 * 封装权益相关的页面操作，包括权益 CRUD、发放记录、同步等功能。
 */
export class BenefitsPage extends BasePage {
  readonly table = this.page.locator('.ant-table');
  readonly createButton = this.page.locator('button:has-text("新建权益")');
  readonly searchInput = this.page.locator('input[placeholder*="搜索"]');
  readonly typeFilter = this.page.locator('.ant-select:has-text("类型")');

  // 表单元素
  readonly nameInput = this.page.locator('#name');
  readonly typeSelect = this.page.locator('#benefit_type');
  readonly valueInput = this.page.locator('#value');
  readonly externalIdInput = this.page.locator('#external_id');
  readonly descriptionInput = this.page.locator('#description');
  readonly validityDaysInput = this.page.locator('#validity_days');

  // 关联徽章模态框
  readonly badgeLinkModal = this.page.locator('.badge-link-modal');
  readonly badgeSelector = this.page.locator('.badge-selector');

  constructor(page: Page) {
    super(page);
  }

  async goto(): Promise<void> {
    await this.page.goto('/benefits');
    await this.waitForPageLoad();
  }

  async gotoGrants(): Promise<void> {
    await this.page.goto('/benefits/grants');
    await this.waitForPageLoad();
  }

  async gotoSync(): Promise<void> {
    await this.page.goto('/benefits/sync');
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

  async filterByType(type: string): Promise<void> {
    await this.typeFilter.click();
    await this.page.locator(`.ant-select-item:has-text("${type}")`).click();
    await this.waitForLoading();
  }

  async clickEdit(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("编辑")`).click();
    await this.waitForPageLoad();
  }

  async clickDelete(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("删除")`).click();
  }

  async clickLinkBadge(name: string): Promise<void> {
    await this.page.locator(`tr:has-text("${name}") button:has-text("关联徽章")`).click();
    await this.badgeLinkModal.waitFor({ state: 'visible' });
  }

  async selectBadges(badgeNames: string[]): Promise<void> {
    for (const name of badgeNames) {
      await this.page.locator(`.badge-selector-item:has-text("${name}")`).click();
    }
  }

  async getBenefitCount(): Promise<number> {
    return await this.page.locator('.ant-table-row').count();
  }

  async expectBenefitExists(name: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${name}")`)).toBeVisible();
  }

  async expectBenefitNotExists(name: string): Promise<void> {
    await expect(this.page.locator(`tr:has-text("${name}")`)).not.toBeVisible();
  }
}

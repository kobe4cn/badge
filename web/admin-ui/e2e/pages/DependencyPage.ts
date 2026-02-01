import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 徽章依赖配置页面对象
 */
export class DependencyPage extends BasePage {
  readonly dependencyList = this.page.locator('.dependency-list');
  readonly addButton = this.page.locator('button:has-text("添加依赖")');
  readonly dependencyGraph = this.page.locator('.dependency-graph');

  // 添加依赖表单
  readonly badgeSelect = this.page.locator('#depends_on_badge_id');
  readonly typeSelect = this.page.locator('#dependency_type');
  readonly quantityInput = this.page.locator('#required_quantity');
  readonly autoTriggerSwitch = this.page.locator('#auto_trigger');
  readonly exclusiveGroupInput = this.page.locator('#exclusive_group_id');

  constructor(page: Page) {
    super(page);
  }

  async goto(badgeId: number): Promise<void> {
    await this.page.goto(`/badges/${badgeId}/dependencies`);
    await this.waitForPageLoad();
  }

  async addDependency(options: {
    badgeName: string;
    type: 'prerequisite' | 'consume' | 'exclusive';
    quantity?: number;
    autoTrigger?: boolean;
    exclusiveGroup?: string;
  }): Promise<void> {
    await this.addButton.click();

    // 选择依赖徽章
    await this.badgeSelect.click();
    await this.page.locator(`.ant-select-item:has-text("${options.badgeName}")`).click();

    // 选择依赖类型
    await this.typeSelect.click();
    const typeLabel = {
      prerequisite: '前置条件',
      consume: '消耗',
      exclusive: '互斥',
    };
    await this.page.locator(`.ant-select-item:has-text("${typeLabel[options.type]}")`).click();

    // 设置数量
    if (options.quantity) {
      await this.quantityInput.fill(String(options.quantity));
    }

    // 自动触发
    if (options.autoTrigger) {
      await this.autoTriggerSwitch.check();
    }

    // 互斥组
    if (options.exclusiveGroup) {
      await this.exclusiveGroupInput.fill(options.exclusiveGroup);
    }

    await this.clickButton('确定');
    await this.waitForMessage('success');
  }

  async removeDependency(badgeName: string): Promise<void> {
    await this.page.locator(`.dependency-item:has-text("${badgeName}") button:has-text("删除")`).click();
    await this.confirmModal();
    await this.waitForMessage('success');
  }

  async getDependencyCount(): Promise<number> {
    return await this.page.locator('.dependency-item').count();
  }

  async expectDependencyExists(badgeName: string): Promise<void> {
    await expect(this.page.locator(`.dependency-item:has-text("${badgeName}")`)).toBeVisible();
  }

  async expectDependencyNotExists(badgeName: string): Promise<void> {
    await expect(this.page.locator(`.dependency-item:has-text("${badgeName}")`)).not.toBeVisible();
  }

  async viewGraph(): Promise<void> {
    await this.page.locator('button:has-text("查看依赖图")').click();
    await this.dependencyGraph.waitFor({ state: 'visible' });
  }
}

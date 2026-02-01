import { Page, Locator, expect } from '@playwright/test';

/**
 * 页面对象基类
 *
 * 提供通用的页面操作和等待方法。
 */
export abstract class BasePage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  /**
   * 等待页面加载完成
   */
  async waitForPageLoad(): Promise<void> {
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * 等待 API 请求完成
   */
  async waitForApi(urlPattern: string | RegExp): Promise<void> {
    await this.page.waitForResponse(
      (response) =>
        (typeof urlPattern === 'string'
          ? response.url().includes(urlPattern)
          : urlPattern.test(response.url())) && response.status() === 200
    );
  }

  /**
   * 等待加载状态消失
   */
  async waitForLoading(): Promise<void> {
    const spinner = this.page.locator('.ant-spin-spinning');
    if (await spinner.isVisible()) {
      await spinner.waitFor({ state: 'hidden', timeout: 30000 });
    }
  }

  /**
   * 等待消息提示
   */
  async waitForMessage(type: 'success' | 'error' | 'warning' | 'info'): Promise<string> {
    const message = this.page.locator(`.ant-message-${type}`);
    await message.waitFor({ state: 'visible' });
    const text = await message.textContent();
    return text || '';
  }

  /**
   * 确认模态框
   */
  async confirmModal(): Promise<void> {
    await this.page.locator('.ant-modal-confirm-btns .ant-btn-primary').click();
    await this.page.locator('.ant-modal').waitFor({ state: 'hidden' });
  }

  /**
   * 取消模态框
   */
  async cancelModal(): Promise<void> {
    await this.page.locator('.ant-modal-confirm-btns .ant-btn:not(.ant-btn-primary)').click();
    await this.page.locator('.ant-modal').waitFor({ state: 'hidden' });
  }

  /**
   * 填写表单项
   */
  async fillFormItem(label: string, value: string): Promise<void> {
    const formItem = this.page.locator(`.ant-form-item:has(.ant-form-item-label:has-text("${label}"))`);
    const input = formItem.locator('input, textarea').first();
    await input.fill(value);
  }

  /**
   * 选择下拉框选项
   */
  async selectOption(label: string, optionText: string): Promise<void> {
    const formItem = this.page.locator(`.ant-form-item:has(.ant-form-item-label:has-text("${label}"))`);
    await formItem.locator('.ant-select-selector').click();
    await this.page.locator(`.ant-select-item-option:has-text("${optionText}")`).click();
  }

  /**
   * 点击按钮
   */
  async clickButton(text: string): Promise<void> {
    await this.page.locator(`button:has-text("${text}")`).click();
  }

  /**
   * 获取表格数据
   */
  async getTableData(): Promise<string[][]> {
    const rows = await this.page.locator('.ant-table-tbody tr').all();
    const data: string[][] = [];

    for (const row of rows) {
      const cells = await row.locator('td').all();
      const rowData: string[] = [];
      for (const cell of cells) {
        rowData.push((await cell.textContent()) || '');
      }
      data.push(rowData);
    }

    return data;
  }

  /**
   * 截图
   */
  async screenshot(name: string): Promise<void> {
    await this.page.screenshot({
      path: `../test-results/screenshots/${name}.png`,
      fullPage: true,
    });
  }
}

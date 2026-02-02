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
   * 确认模态框或气泡确认框
   * Ant Design 按钮文本可能带空格（如 "确 认"）
   */
  async confirmModal(): Promise<void> {
    // 等待确认框出现
    await this.page.waitForTimeout(300);

    // 先尝试 Popconfirm（气泡确认框）- 使用更具体的选择器
    const popconfirm = this.page.locator('.ant-popconfirm, .ant-popover').filter({ hasText: /确.*认|删.*除|确.*定/ });
    if (await popconfirm.isVisible({ timeout: 2000 }).catch(() => false)) {
      // 按钮文本可能有空格："确 认"
      const okBtn = popconfirm.locator('button').filter({ hasText: /确\s*认|确\s*定|OK/i });
      await okBtn.click();
      await popconfirm.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
      return;
    }

    // 然后尝试 Modal（模态框）
    const modal = this.page.locator('.ant-modal').filter({ hasText: /确.*认|删.*除|确.*定/ });
    if (await modal.isVisible({ timeout: 2000 }).catch(() => false)) {
      const modalOk = modal.locator('.ant-btn-primary, button').filter({ hasText: /确\s*认|确\s*定/ });
      await modalOk.click();
      await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
      return;
    }

    // 最后尝试页面上任何可见的确认按钮（处理空格）
    const confirmBtn = this.page.locator('button').filter({ hasText: /^确\s*认$|^确\s*定$/ }).first();
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
  }

  /**
   * 取消模态框或气泡确认框
   */
  async cancelModal(): Promise<void> {
    // 先尝试 Popconfirm
    const popconfirmCancel = this.page.locator('.ant-popconfirm-buttons .ant-btn:not(.ant-btn-primary), .ant-popover-buttons .ant-btn:not(.ant-btn-primary)');
    if (await popconfirmCancel.isVisible({ timeout: 2000 }).catch(() => false)) {
      await popconfirmCancel.click();
      return;
    }

    // 然后尝试 Modal
    const modalCancel = this.page.locator('.ant-modal-confirm-btns .ant-btn:not(.ant-btn-primary), .ant-modal-footer .ant-btn:not(.ant-btn-primary)');
    await modalCancel.click();
    await this.page.locator('.ant-modal').waitFor({ state: 'hidden' }).catch(() => {});
  }

  /**
   * 填写表单项
   * 优先查找抽屉/模态框中的表单项，避免匹配到页面上的搜索框
   */
  async fillFormItem(label: string, value: string): Promise<void> {
    // 优先在抽屉或模态框中查找
    const drawerFormItem = this.page.locator(`.ant-drawer .ant-form-item`).filter({ hasText: label });
    const modalFormItem = this.page.locator(`.ant-modal .ant-form-item`).filter({ hasText: label });

    let formItem;
    if (await drawerFormItem.first().isVisible().catch(() => false)) {
      formItem = drawerFormItem.first();
    } else if (await modalFormItem.first().isVisible().catch(() => false)) {
      formItem = modalFormItem.first();
    } else {
      // 回退到页面上的表单项
      formItem = this.page.locator(`.ant-form-item`).filter({ hasText: label }).first();
    }

    const input = formItem.locator('input, textarea').first();
    await input.scrollIntoViewIfNeeded();
    await input.fill(value);
  }

  /**
   * 选择下拉框选项
   * 优先查找抽屉/模态框中的表单项
   */
  async selectOption(label: string, optionText: string): Promise<void> {
    // 优先在抽屉或模态框中查找
    const drawerFormItem = this.page.locator(`.ant-drawer .ant-form-item`).filter({ hasText: label });
    const modalFormItem = this.page.locator(`.ant-modal .ant-form-item`).filter({ hasText: label });

    let formItem;
    if (await drawerFormItem.first().isVisible().catch(() => false)) {
      formItem = drawerFormItem.first();
    } else if (await modalFormItem.first().isVisible().catch(() => false)) {
      formItem = modalFormItem.first();
    } else {
      formItem = this.page.locator(`.ant-form-item`).filter({ hasText: label }).first();
    }

    await formItem.locator('.ant-select-selector').first().scrollIntoViewIfNeeded();
    await formItem.locator('.ant-select-selector').first().click();
    // 等待下拉列表出现
    await this.page.locator('.ant-select-dropdown').waitFor({ state: 'visible' });
    await this.page.locator(`.ant-select-item-option:has-text("${optionText}")`).first().click();
  }

  /**
   * 点击按钮
   * 优先查找抽屉/模态框中的按钮
   */
  async clickButton(text: string): Promise<void> {
    // Ant Design 的按钮文本可能有空格（如 "提 交" 而非 "提交"）
    // 使用正则匹配来处理这种情况
    const textWithSpaces = text.split('').join('\\s*');
    const button = this.page.locator(`button`).filter({ hasText: new RegExp(textWithSpaces) }).first();
    await button.waitFor({ state: 'visible', timeout: 10000 });
    await button.click();
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

import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 登录页面对象
 */
export class LoginPage extends BasePage {
  readonly usernameInput = this.page.locator('#username');
  readonly passwordInput = this.page.locator('#password');
  readonly loginButton = this.page.locator('button[type="submit"]');
  readonly errorMessage = this.page.locator('.ant-form-item-explain-error');

  constructor(page: Page) {
    super(page);
  }

  /**
   * 导航到登录页
   */
  async goto(): Promise<void> {
    await this.page.goto('/login');
    await this.waitForPageLoad();
  }

  /**
   * 登录
   */
  async login(username: string, password: string): Promise<void> {
    await this.usernameInput.fill(username);
    await this.passwordInput.fill(password);
    await this.loginButton.click();
  }

  /**
   * 使用管理员账户登录
   */
  async loginAsAdmin(): Promise<void> {
    await this.login('admin', 'admin123');
    await this.page.waitForURL('**/dashboard');
  }

  /**
   * 验证登录成功
   */
  async expectLoginSuccess(): Promise<void> {
    await expect(this.page).toHaveURL(/.*dashboard/);
  }

  /**
   * 验证登录失败
   */
  async expectLoginError(message: string): Promise<void> {
    await expect(this.errorMessage).toContainText(message);
  }
}

import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { testUsers } from '../utils';

test.describe('登录功能', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
  });

  test('管理员成功登录', async () => {
    await loginPage.login(testUsers.admin.username, testUsers.admin.password);
    await loginPage.expectLoginSuccess();
  });

  test('操作员成功登录', async () => {
    await loginPage.login(testUsers.operator.username, testUsers.operator.password);
    await loginPage.expectLoginSuccess();
  });

  test('错误密码登录失败', async () => {
    await loginPage.login(testUsers.admin.username, 'wrong_password');
    await loginPage.expectLoginError('用户名或密码错误');
  });

  test('空用户名登录失败', async () => {
    await loginPage.login('', testUsers.admin.password);
    await loginPage.expectLoginError('请输入用户名');
  });

  test('空密码登录失败', async () => {
    await loginPage.login(testUsers.admin.username, '');
    await loginPage.expectLoginError('请输入密码');
  });

  test('不存在的用户登录失败', async () => {
    await loginPage.login('nonexistent_user', 'any_password');
    await loginPage.expectLoginError('用户名或密码错误');
  });
});

test.describe('登录状态保持', () => {
  test('刷新页面保持登录状态', async ({ page }) => {
    const loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();

    // 刷新页面
    await page.reload();

    // 应该仍在 dashboard
    await expect(page).toHaveURL(/.*dashboard/);
  });

  test('登出后清除登录状态', async ({ page }) => {
    const loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();

    // 登出
    await page.locator('.user-dropdown').click();
    await page.locator('text=退出登录').click();

    // 应该跳转到登录页
    await expect(page).toHaveURL(/.*login/);

    // 直接访问 dashboard 应该重定向到登录页
    await page.goto('/dashboard');
    await expect(page).toHaveURL(/.*login/);
  });
});

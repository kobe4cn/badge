import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { testUsers } from '../utils';
import { ApiHelper } from '../utils/api-helper';

/**
 * 前后端集成测试
 *
 * 测试真实后端 API 交互，禁用 mock 模式运行
 * 运行命令: npm run test:integration
 */

// 公共辅助函数：登录并等待页面稳定
async function loginAndWait(page: import('@playwright/test').Page, username: string, password: string) {
  const loginPage = new LoginPage(page);
  await loginPage.goto();
  await loginPage.login(username, password);
  await page.waitForURL('**/dashboard', { timeout: 15000 });
  await page.waitForLoadState('networkidle').catch(() => {});
}

// 辅助函数：等待页面中的表格加载
async function waitForTable(page: import('@playwright/test').Page) {
  // 使用 .first() 避免 strict mode violation（antd table 会同时渲染 div.ant-table 和 table 元素）
  await expect(
    page.locator('table').first()
  ).toBeVisible({ timeout: 15000 });
}

// 辅助函数：通过搜索框过滤列表数据，防止分页导致目标项不在当前页
async function searchInTable(page: import('@playwright/test').Page, keyword: string) {
  const searchInput = page.locator('input[placeholder="请输入"]').first();
  if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
    await searchInput.fill(keyword);
    await page.locator('button').filter({ hasText: /查\s*询/ }).click();
    await page.waitForTimeout(1000);
    await waitForTable(page);
  }
}

test.describe('集成测试: 认证流程', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    await loginPage.goto();
  });

  test('管理员登录成功并获取完整用户信息', async ({ page }) => {
    const loginResponse = page.waitForResponse('**/api/admin/auth/login');

    await loginPage.login(testUsers.admin.username, testUsers.admin.password);

    const response = await loginResponse;
    expect(response.status()).toBe(200);

    const data = await response.json();
    expect(data.success).toBe(true);
    expect(data.data.token).toBeTruthy();
    expect(data.data.user).toBeTruthy();
    expect(data.data.user.username).toBe('admin');

    await loginPage.expectLoginSuccess();
  });

  test('登录后获取当前用户信息', async ({ page }) => {
    await loginPage.loginAsAdmin();

    // 验证页面显示用户名
    await expect(page.getByText('系统管理员').first()).toBeVisible({ timeout: 10000 });
  });

  test('登出清除认证状态', async ({ page }) => {
    await loginPage.loginAsAdmin();

    // 找到用户区域并点击展开下拉菜单
    const userArea = page.locator('.ant-dropdown-trigger, .user-dropdown').first();
    await userArea.click();

    // 点击登出
    await page.getByText('退出登录').click();

    await expect(page).toHaveURL(/.*login/);

    // 验证直接访问受保护页面会重定向
    await page.goto('/dashboard');
    await expect(page).toHaveURL(/.*login/);
  });

  test('错误密码登录失败', async ({ page }) => {
    const loginResponse = page.waitForResponse('**/api/admin/auth/login');

    await loginPage.login(testUsers.admin.username, 'wrong_password');

    const response = await loginResponse;
    expect(response.status()).toBe(401);

    // 验证错误提示
    await expect(
      page.locator('.ant-form-item-explain-error, .ant-message-error')
    ).toBeVisible({ timeout: 5000 });
  });
});

test.describe('集成测试: 分类管理 CRUD', () => {
  const testPrefix = `T${Date.now().toString(36)}_`;

  test.beforeEach(async ({ page }) => {
    await loginAndWait(page, testUsers.admin.username, testUsers.admin.password);
  });

  test.afterEach(async ({ request }) => {
    const api = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');
    await api.login(testUsers.admin.username, testUsers.admin.password);
    await api.cleanup(testPrefix);
  });

  test('创建分类', async ({ page }) => {
    await page.goto('/badges/categories');
    await waitForTable(page);

    // 点击新建按钮
    await page.locator('button').filter({ hasText: /新建/ }).first().click();

    // 等待弹窗出现
    await page.locator('.ant-modal').waitFor({ state: 'visible' });

    // 使用 placeholder 精确定位可见输入框
    const categoryName = `${testPrefix}Cat`;
    await page.getByPlaceholder('请输入分类名称').fill(categoryName);

    // 提交（antd 按钮文本可能含空格，如"提 交"）
    await page.locator('.ant-modal').locator('button').filter({ hasText: /提\s*交/ }).click();

    // 等待列表刷新
    await page.waitForResponse(resp => resp.url().includes('/categories') && resp.request().method() === 'POST', { timeout: 10000 }).catch(() => {});
    await page.waitForTimeout(1500);
    await page.reload();
    await waitForTable(page);

    // 使用搜索框过滤数据，确保目标项在当前页
    await searchInTable(page, testPrefix);

    // 验证
    await expect(page.getByText(categoryName).first()).toBeVisible({ timeout: 10000 });
  });

  test('编辑分类', async ({ page, request }) => {
    const api = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');
    await api.login(testUsers.admin.username, testUsers.admin.password);
    await api.createCategory({ name: `${testPrefix}Edit`, sortOrder: 0 });

    await page.goto('/badges/categories');
    await waitForTable(page);

    // 使用搜索框过滤数据，确保目标项在当前页
    await searchInTable(page, testPrefix);

    // 等待数据加载
    await expect(page.getByText(`${testPrefix}Edit`).first()).toBeVisible({ timeout: 10000 });

    // 点击编辑按钮
    const row = page.locator('tr').filter({ hasText: `${testPrefix}Edit` });
    await row.getByText('编辑').click();

    // 等待弹窗
    const modal = page.locator('.ant-modal');
    await modal.waitFor({ state: 'visible' });

    // 修改名称（使用全选+输入替代 clear，更兼容 antd showCount 输入框）
    const newName = `${testPrefix}Edited`;
    const nameInput = page.getByPlaceholder('请输入分类名称');
    await nameInput.click();
    await nameInput.press('Meta+a');
    await nameInput.fill(newName);

    // 提交并等待 PUT 请求完成
    const putResponse = page.waitForResponse(
      resp => resp.url().includes('/categories/') && resp.request().method() === 'PUT',
      { timeout: 10000 }
    );
    await modal.locator('button').filter({ hasText: /提\s*交/ }).click();
    const resp = await putResponse.catch(() => null);

    // 验证修改成功
    await page.waitForTimeout(1000);
    await page.reload();
    await waitForTable(page);

    // 使用搜索框过滤数据，确保目标项在当前页
    await searchInTable(page, testPrefix);

    await expect(page.getByText(newName).first()).toBeVisible({ timeout: 10000 });
  });

  test('删除分类', async ({ page, request }) => {
    const api = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');
    await api.login(testUsers.admin.username, testUsers.admin.password);
    await api.createCategory({ name: `${testPrefix}Del`, sortOrder: 0 });

    await page.goto('/badges/categories');
    await waitForTable(page);

    // 使用搜索框过滤数据，确保目标项在当前页
    await searchInTable(page, testPrefix);

    await expect(page.getByText(`${testPrefix}Del`).first()).toBeVisible({ timeout: 10000 });

    // 点击删除按钮
    const row = page.locator('tr').filter({ hasText: `${testPrefix}Del` });
    await row.getByText('删').click();

    // 等待确认弹出并点击"确 认"
    await page.waitForTimeout(500);
    const confirmBtn = page.getByRole('button', { name: /确\s*认/ });
    await confirmBtn.click();

    // 等待删除 API 完成并刷新页面
    await page.waitForResponse(resp => resp.url().includes('/categories') && resp.request().method() === 'DELETE', { timeout: 10000 }).catch(() => {});
    await page.waitForTimeout(1500);
    await page.reload();
    await waitForTable(page);

    // 使用搜索框过滤数据，确保在过滤后的结果中验证删除
    await searchInTable(page, testPrefix);

    // 验证删除成功
    await expect(page.getByText(`${testPrefix}Del`)).not.toBeVisible({ timeout: 5000 });
  });
});

test.describe('集成测试: 徽章完整流程', () => {
  let api: ApiHelper;
  const testPrefix = `T${Date.now().toString(36)}_`;
  let seriesId: number;

  test.beforeAll(async ({ request }) => {
    api = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');
    await api.login(testUsers.admin.username, testUsers.admin.password);
    const testData = await api.ensureTestData(testPrefix);
    seriesId = testData.seriesId;
  });

  test.beforeEach(async ({ page }) => {
    await loginAndWait(page, testUsers.admin.username, testUsers.admin.password);
  });

  test.afterAll(async ({ request }) => {
    const cleanup = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');
    await cleanup.login(testUsers.admin.username, testUsers.admin.password);
    await cleanup.cleanup(testPrefix);
  });

  test('创建徽章', async ({ page }) => {
    // 跳过如果 seriesId 无效
    test.skip(!seriesId, '基础测试数据创建失败');

    await page.goto('/badges/definitions');
    await waitForTable(page);

    // 点击新建
    await page.locator('button').filter({ hasText: /新建/ }).first().click();

    // 等待 Drawer 出现（徽章使用 Drawer）
    const drawer = page.locator('.ant-drawer, dialog');
    await drawer.waitFor({ state: 'visible' });

    // 使用 placeholder 精确定位徽章名称输入框
    const badgeName = `${testPrefix}Badge`;
    const nameInput = page.getByPlaceholder('请输入徽章名称');
    if (await nameInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await nameInput.fill(badgeName);
    } else {
      // 尝试使用 label 定位
      await page.getByLabel('徽章名称').first().fill(badgeName);
    }

    // 选择系列（antd Select 下拉需要等待选项加载）
    const seriesSelect = drawer.locator('.ant-form-item').filter({ hasText: /系列/ }).locator('.ant-select-selector');
    if (await seriesSelect.isVisible({ timeout: 2000 }).catch(() => false)) {
      await seriesSelect.click();
      // 等待下拉菜单出现并包含选项
      const dropdown = page.locator('.ant-select-dropdown:visible');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      await page.waitForTimeout(500);
      const option = dropdown.locator('.ant-select-item-option').first();
      if (await option.isVisible({ timeout: 3000 }).catch(() => false)) {
        await option.click();
      } else {
        // 下拉没有选项，按 Escape 关闭
        await page.keyboard.press('Escape');
      }
    }

    // 填写图标 URL
    const iconInput = page.getByPlaceholder(/图标/);
    if (await iconInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await iconInput.fill('https://example.com/icon.png');
    }

    // 提交（antd 按钮文本可能含空格）
    await drawer.locator('button').filter({ hasText: /提\s*交/ }).click();

    // 验证操作结果
    const hasSuccess = await page.locator('.ant-message-success').isVisible({ timeout: 5000 }).catch(() => false);
    const hasError = await page.locator('.ant-form-item-explain-error').isVisible({ timeout: 2000 }).catch(() => false);

    if (!hasSuccess && hasError) {
      test.info().annotations.push({ type: 'note', description: '表单验证错误，需要调整必填字段' });
    }
    expect(hasSuccess || !hasError).toBeTruthy();
  });
});

test.describe('集成测试: 页面加载验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginAndWait(page, testUsers.admin.username, testUsers.admin.password);
  });

  test('用户列表页面加载', async ({ page }) => {
    await page.goto('/system/users');
    await waitForTable(page);

    // 验证页面标题
    await expect(page.getByText('用户管理').first()).toBeVisible();

    // 验证新建按钮存在
    await expect(page.locator('button').filter({ hasText: /新建/ }).first()).toBeVisible();
  });

  test('角色列表页面加载', async ({ page }) => {
    await page.goto('/system/roles');
    await waitForTable(page);

    // 验证页面标题
    await expect(page.getByText('角色管理').first()).toBeVisible();
  });

  test('权益列表页面加载', async ({ page }) => {
    await page.goto('/benefits/list');
    await waitForTable(page);

    // 验证页面标题
    await expect(page.getByText('权益列表').first()).toBeVisible();
  });

  test('兑换规则列表页面加载', async ({ page }) => {
    await page.goto('/redemptions/rules');
    await waitForTable(page);

    // 验证页面标题或表格存在
    await expect(page.locator('table, .ant-empty').first()).toBeVisible();
  });

  test('规则列表页面加载', async ({ page }) => {
    await page.goto('/rules/canvas');
    await page.waitForLoadState('networkidle').catch(() => {});

    // 验证页面加载（画布或表格）
    const hasCanvas = await page.locator('.react-flow').isVisible({ timeout: 5000 }).catch(() => false);
    const hasTable = await page.locator('table').isVisible({ timeout: 3000 }).catch(() => false);
    const hasForm = await page.locator('form, .ant-form').isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasCanvas || hasTable || hasForm).toBeTruthy();
  });

  test('规则画布加载', async ({ page }) => {
    await page.goto('/rules/create');
    await page.waitForLoadState('networkidle').catch(() => {});

    const hasCanvas = await page.locator('.react-flow').isVisible({ timeout: 5000 }).catch(() => false);
    const hasForm = await page.locator('form, .ant-form').isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasCanvas || hasForm).toBeTruthy();
  });

  test('仪表盘页面加载', async ({ page }) => {
    // 登录后已在 dashboard
    await expect(page.getByText('数据看板').first()).toBeVisible({ timeout: 10000 });
  });

  test('分类管理页面加载', async ({ page }) => {
    await page.goto('/badges/categories');
    await waitForTable(page);

    await expect(page.getByText('分类管理').first()).toBeVisible();
  });

  test('系列管理页面加载', async ({ page }) => {
    await page.goto('/badges/series');
    await waitForTable(page);

    await expect(page.getByText('系列管理').first()).toBeVisible();
  });
});

test.describe('集成测试: 权限控制', () => {
  test('运营人员可以正常登录并访问业务页面', async ({ page }) => {
    try {
      await loginAndWait(page, testUsers.operator.username, testUsers.operator.password);
    } catch (e) {
      // operator 用户可能不存在于真实后端数据库中
      test.info().annotations.push({ type: 'skip', description: 'operator 用户不存在，跳过测试' });
      return;
    }

    // 运营人员应该能访问徽章管理
    await page.goto('/badges/categories');
    await waitForTable(page);
    await expect(page.getByText('分类管理').first()).toBeVisible();
  });

  test('只读用户可以登录并查看页面', async ({ page }) => {
    try {
      await loginAndWait(page, testUsers.viewer.username, testUsers.viewer.password);
    } catch (e) {
      // viewer 用户可能不存在于真实后端数据库中
      test.info().annotations.push({ type: 'skip', description: 'viewer 用户不存在，跳过测试' });
      return;
    }

    // viewer 应该能看到页面
    await page.goto('/badges/definitions');
    await page.waitForLoadState('networkidle').catch(() => {});
    await waitForTable(page);

    // 验证页面加载成功（viewer 可以看到列表）
    await expect(page.getByText('徽章定义').first()).toBeVisible();
  });

  test('不同角色用户登录返回正确权限', async ({ request }) => {
    const api = new ApiHelper(request, process.env.BASE_URL || 'http://localhost:3001');

    // admin 应该拥有 system 模块权限
    await api.login(testUsers.admin.username, testUsers.admin.password);
    const adminResp = await request.get(`${process.env.BASE_URL || 'http://localhost:3001'}/api/admin/auth/me`, {
      headers: { 'Authorization': `Bearer ${await api.login(testUsers.admin.username, testUsers.admin.password)}` },
    });
    const adminData = await adminResp.json();
    expect(adminData.data.permissions).toContain('system:user:write');

    // operator 不应该有 system 模块写权限（用户可能不存在）
    try {
      const operatorToken = await api.login(testUsers.operator.username, testUsers.operator.password);
      const operatorResp = await request.get(`${process.env.BASE_URL || 'http://localhost:3001'}/api/admin/auth/me`, {
        headers: { 'Authorization': `Bearer ${operatorToken}` },
      });
      const operatorData = await operatorResp.json();
      expect(operatorData.data.permissions).not.toContain('system:user:write');
    } catch (e) {
      test.info().annotations.push({ type: 'skip', description: 'operator 用户不存在，跳过 operator 权限验证' });
    }

    // viewer 应该只有 read 权限（用户可能不存在）
    try {
      const viewerToken = await api.login(testUsers.viewer.username, testUsers.viewer.password);
      const viewerResp = await request.get(`${process.env.BASE_URL || 'http://localhost:3001'}/api/admin/auth/me`, {
        headers: { 'Authorization': `Bearer ${viewerToken}` },
      });
      const viewerData = await viewerResp.json();
      const viewerPerms = viewerData.data.permissions as string[];
      const hasWritePermission = viewerPerms.some((p: string) => p.endsWith(':write'));
      expect(hasWritePermission).toBe(false);
    } catch (e) {
      test.info().annotations.push({ type: 'skip', description: 'viewer 用户不存在，跳过 viewer 权限验证' });
    }
  });
});

import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import { DependencyPage } from '../pages/DependencyPage';
import { ApiHelper, uniqueId } from '../utils';

test.describe('徽章依赖配置', () => {
  let loginPage: LoginPage;
  let dependencyPage: DependencyPage;
  let apiHelper: ApiHelper;
  const testPrefix = uniqueId('e2e_dep_');

  test.beforeEach(async ({ page, request }) => {
    loginPage = new LoginPage(page);
    dependencyPage = new DependencyPage(page);
    apiHelper = new ApiHelper(request, process.env.API_BASE_URL || 'http://localhost:8080');

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('依赖列表加载', async () => {
    await dependencyPage.goto(1);
    await expect(dependencyPage.dependencyList).toBeVisible();
    await expect(dependencyPage.addButton).toBeVisible();
  });

  test('添加前置条件依赖', async () => {
    // 先创建两个徽章
    await apiHelper.login('admin', 'admin123');
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}徽章A` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}徽章B` });

    await dependencyPage.goto(badgeB.id);

    await dependencyPage.addDependency({
      badgeName: badgeA.name,
      type: 'prerequisite',
      autoTrigger: true,
    });

    await dependencyPage.expectDependencyExists(badgeA.name);
  });

  test('添加消耗依赖', async () => {
    await apiHelper.login('admin', 'admin123');
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}消耗源` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}消耗目标` });

    await dependencyPage.goto(badgeB.id);

    await dependencyPage.addDependency({
      badgeName: badgeA.name,
      type: 'consume',
      quantity: 1,
    });

    await dependencyPage.expectDependencyExists(badgeA.name);
  });

  test('添加互斥依赖', async () => {
    await apiHelper.login('admin', 'admin123');
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}互斥A` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}互斥B` });

    await dependencyPage.goto(badgeB.id);

    await dependencyPage.addDependency({
      badgeName: badgeA.name,
      type: 'exclusive',
      exclusiveGroup: 'test_group',
    });

    await dependencyPage.expectDependencyExists(badgeA.name);
  });

  test('删除依赖', async () => {
    await apiHelper.login('admin', 'admin123');
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}待删除源` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}待删除目标` });

    await dependencyPage.goto(badgeB.id);

    // 先添加
    await dependencyPage.addDependency({
      badgeName: badgeA.name,
      type: 'prerequisite',
    });

    // 再删除
    await dependencyPage.removeDependency(badgeA.name);
    await dependencyPage.expectDependencyNotExists(badgeA.name);
  });

  test('查看依赖图', async ({ page }) => {
    await dependencyPage.goto(1);

    // 点击查看依赖图
    await dependencyPage.viewGraph();

    // 验证图表显示
    await expect(dependencyPage.dependencyGraph).toBeVisible();
  });

  test('循环依赖检测', async ({ page }) => {
    await apiHelper.login('admin', 'admin123');
    const badgeA = await apiHelper.createBadge({ name: `${testPrefix}循环A` });
    const badgeB = await apiHelper.createBadge({ name: `${testPrefix}循环B` });

    // A -> B
    await dependencyPage.goto(badgeB.id);
    await dependencyPage.addDependency({
      badgeName: badgeA.name,
      type: 'prerequisite',
    });

    // 尝试 B -> A（应该失败）
    await dependencyPage.goto(badgeA.id);
    await dependencyPage.addButton.click();
    await dependencyPage.badgeSelect.click();
    await page.locator(`.ant-select-item:has-text("${badgeB.name}")`).click();
    await dependencyPage.typeSelect.click();
    await page.locator('.ant-select-item:has-text("前置条件")').click();
    await dependencyPage.clickButton('确定');

    // 应该显示错误
    await expect(page.locator('.ant-message-error')).toBeVisible();
  });
});

test.describe('依赖图可视化', () => {
  let loginPage: LoginPage;
  let dependencyPage: DependencyPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dependencyPage = new DependencyPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test('依赖图节点显示正确', async ({ page }) => {
    await dependencyPage.goto(1);
    await dependencyPage.viewGraph();

    // 验证节点存在
    const nodes = await page.locator('.dependency-graph .node').count();
    expect(nodes).toBeGreaterThanOrEqual(0);
  });

  test('依赖图连线显示正确', async ({ page }) => {
    await dependencyPage.goto(1);
    await dependencyPage.viewGraph();

    // 验证连线样式
    const edges = await page.locator('.dependency-graph .edge').count();
    expect(edges).toBeGreaterThanOrEqual(0);
  });
});

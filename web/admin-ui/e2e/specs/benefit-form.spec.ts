import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';

/**
 * 权益表单（BenefitForm）交互测试
 *
 * 验证权益新建/编辑弹窗的字段渲染、表单校验、下拉交互等行为，
 * 不依赖后端数据的具体内容，聚焦于表单组件本身的可用性。
 */
test.describe('权益表单', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }, testInfo) => {
    const isMobile = testInfo.project.name.toLowerCase().includes('mobile');
    test.skip(isMobile, 'Skipping mobile browser tests due to layout issues');

    loginPage = new LoginPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();

    // 所有用例均从权益列表页开始操作
    await page.goto('/benefits');
    await page.waitForLoadState('networkidle').catch(() => {});
  });

  test('新建权益弹窗正确打开', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await expect(createButton).toBeVisible({ timeout: 5000 });
    await createButton.click();
    await page.waitForTimeout(500);

    // 弹窗必须可见且标题为"新建权益"，确认打开的是新建模式
    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });
    await expect(modal.getByText('新建权益').first()).toBeVisible();

    // 关闭弹窗，还原页面状态
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('新建表单包含所有必要字段', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await createButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 必填字段标签必须全部渲染，缺少任何一个都说明表单结构损坏
    await expect(modal.getByText('权益编码').first()).toBeVisible();
    await expect(modal.getByText('权益名称').first()).toBeVisible();
    await expect(modal.getByText('权益类型').first()).toBeVisible();

    // 关闭弹窗
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('表单验证 - 空提交显示错误', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await createButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 不填写任何字段直接提交，触发客户端校验
    await page.locator('.ant-modal button').filter({ hasText: /提.*交|确.*定/ }).click();
    await page.waitForTimeout(500);

    // 必填字段未填时必须出现校验错误提示，防止无效数据提交到后端
    const errorMessage = page.locator('.ant-form-item-explain-error');
    const hasError = await errorMessage.count() > 0;
    expect(hasError).toBe(true);

    // 关闭弹窗
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
  });

  test('权益类型下拉框可交互', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await createButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 点击权益类型选择器，触发下拉面板展开
    const typeFormItem = modal.locator('.ant-form-item').filter({ hasText: '权益类型' });
    await typeFormItem.locator('.ant-select-selector').click();
    await page.waitForTimeout(300);

    // 下拉面板必须包含预设的权益类型选项，保证枚举值正确传递到前端
    const dropdown = page.locator('.ant-select-dropdown');
    await expect(dropdown).toBeVisible({ timeout: 3000 });

    const hasPoints = await dropdown.getByText('积分').isVisible({ timeout: 2000 }).catch(() => false);
    const hasCoupon = await dropdown.getByText('优惠券').isVisible({ timeout: 2000 }).catch(() => false);
    expect(hasPoints || hasCoupon).toBe(true);

    // 收起下拉面板，避免影响后续操作
    await page.keyboard.press('Escape');

    // 关闭弹窗
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('取消按钮关闭弹窗', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await createButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 取消按钮必须能正确关闭弹窗，而不是残留在页面上遮挡操作
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 });
  });

  test('编辑按钮打开弹窗（有数据时）', async ({ page }) => {
    // 只有表格中存在数据行时才能测试编辑流程
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();

    if (rowCount === 0) {
      test.skip(true, '表格无数据，跳过编辑弹窗测试');
      return;
    }

    const editButton = page.locator('button').filter({ hasText: /编辑/ }).first();
    await editButton.click();
    await page.waitForTimeout(500);

    // 编辑模式的弹窗标题应明确区分于新建模式
    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });
    await expect(modal.getByText('编辑权益').first()).toBeVisible();

    // 关闭弹窗
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('编辑模式下编码字段禁用', async ({ page }) => {
    // 编码是权益的唯一标识，编辑时不允许修改以保证数据一致性
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();

    if (rowCount === 0) {
      test.skip(true, '表格无数据，跳过编码禁用测试');
      return;
    }

    const editButton = page.locator('button').filter({ hasText: /编辑/ }).first();
    await editButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 定位编码所在的表单项，验证其 input 处于 disabled 状态
    const codeFormItem = modal.locator('.ant-form-item').filter({ hasText: '权益编码' });
    const codeInput = codeFormItem.locator('input').first();
    await expect(codeInput).toBeDisabled();

    // 关闭弹窗
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('表单字段可填写', async ({ page }) => {
    const createButton = page.locator('button').filter({ hasText: /新建权益/ });
    await createButton.click();
    await page.waitForTimeout(500);

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });

    // 填写编码字段
    const codeFormItem = modal.locator('.ant-form-item').filter({ hasText: '权益编码' });
    const codeInput = codeFormItem.locator('input').first();
    await codeInput.fill('test_benefit_code');
    await expect(codeInput).toHaveValue('test_benefit_code');

    // 填写名称字段
    const nameFormItem = modal.locator('.ant-form-item').filter({ hasText: '权益名称' });
    const nameInput = nameFormItem.locator('input').first();
    await nameInput.fill('测试权益');
    await expect(nameInput).toHaveValue('测试权益');

    // 选择权益类型，验证下拉选择器能正确赋值
    const typeFormItem = modal.locator('.ant-form-item').filter({ hasText: '权益类型' });
    await typeFormItem.locator('.ant-select-selector').click();
    await page.waitForTimeout(300);

    const dropdown = page.locator('.ant-select-dropdown');
    await expect(dropdown).toBeVisible({ timeout: 3000 });

    // 选中第一个可用选项
    const firstOption = dropdown.locator('.ant-select-item-option').first();
    await firstOption.click();
    await page.waitForTimeout(300);

    // 选择后下拉框应有已选中的值（selection-item 出现表示选中成功）
    const selectedValue = typeFormItem.locator('.ant-select-selection-item');
    const hasSelection = await selectedValue.isVisible({ timeout: 2000 }).catch(() => false);
    expect(hasSelection).toBe(true);

    // 关闭弹窗但不提交，避免产生脏数据
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });
});

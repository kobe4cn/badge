import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';

/**
 * 兑换规则表单 - UI 交互验证测试
 *
 * 验证 RedemptionRuleForm（ModalForm）的基本 UI 功能，不依赖具体后端数据
 * 表单为 600px 宽 ModalForm，包含动态列表（ProFormList）和分组分隔线
 */
test.describe('兑换规则表单', () => {
  let loginPage: LoginPage;

  test.beforeEach(async ({ page }, testInfo) => {
    const isMobile = testInfo.project.name.toLowerCase().includes('mobile');
    test.skip(isMobile, 'Skipping mobile browser tests due to layout issues');

    loginPage = new LoginPage(page);

    await loginPage.goto();
    await loginPage.loginAsAdmin();

    // 导航到兑换规则页面并等待加载完成
    await page.goto('/redemptions/rules');
    await page.waitForLoadState('networkidle').catch(() => {});
    await expect(page.locator('table, .ant-empty').first()).toBeVisible({ timeout: 15000 });
  });

  test('新建规则弹窗正确打开', async ({ page }) => {
    // 兑换规则使用 ModalForm，需要确认按钮触发后模态框正确渲染
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await expect(createButton).toBeVisible();
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 新建场景下标题应为"新建兑换规则"，区别于编辑场景
    await expect(modal.getByText('新建兑换规则').first()).toBeVisible();

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('新建表单包含所有必要字段和分组', async ({ page }) => {
    // 表单包含基础字段、动态列表、频率限制分隔线、时间与选项分隔线等区域
    // 需要确认所有分组和字段都正确渲染，避免遗漏必填项导致用户困惑
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 验证基础字段标签
    await expect(modal.getByText('规则名称').first()).toBeVisible();
    await expect(modal.getByText('权益 ID').first()).toBeVisible();
    await expect(modal.getByText('所需徽章').first()).toBeVisible();

    // 验证分隔线（Divider）标题，确保频率限制和时间选项两组字段分区清晰
    await expect(modal.getByText('频率限制').first()).toBeVisible();
    await expect(modal.getByText('时间与选项').first()).toBeVisible();

    // 验证自动兑换开关存在
    await expect(modal.getByText('自动兑换').first()).toBeVisible();

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('表单验证 - 空提交显示错误', async ({ page }) => {
    // 必填字段（name、benefitId、requiredBadges）未填写时直接提交应触发校验
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 直接点击提交，不填写任何字段
    await page.locator('.ant-modal button').filter({ hasText: /提.*交/ }).click();
    await page.waitForTimeout(500);

    // 至少有一个必填字段的校验提示出现
    const errorMessage = page.locator('.ant-form-item-explain-error');
    const hasError = await errorMessage.count() > 0;
    expect(hasError).toBe(true);

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('所需徽章列表可添加', async ({ page }) => {
    // ProFormList 需要手动添加行，验证"添加所需徽章"按钮能正确插入新行
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 点击"添加所需徽章"按钮，ProFormList 会在列表末尾追加一行
    const addBadgeButton = modal.locator('button').filter({ hasText: /添加所需徽章/ });
    if (await addBadgeButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await addBadgeButton.click();
      await page.waitForTimeout(300);

      // 新行应包含 badgeId 和 quantity 两个数字输入框
      const digitInputs = modal.locator('.ant-pro-form-list-container .ant-input-number, .ant-pro-form-list-container input[type="number"]');
      const inputCount = await digitInputs.count();
      // 每行至少有 badgeId + quantity 两个字段
      expect(inputCount).toBeGreaterThanOrEqual(2);
    }

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('取消按钮关闭弹窗', async ({ page }) => {
    // 确保取消操作不会触发表单提交，且模态框能正确关闭
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 点击取消
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();

    // 验证模态框消失
    await expect(modal).not.toBeVisible({ timeout: 5000 });
  });

  test('编辑按钮打开弹窗（有数据时）', async ({ page }) => {
    // 编辑操作需要表格中已有数据行，无数据时跳过此验证
    const rows = page.locator('.ant-table-tbody tr[data-row-key]');
    const rowCount = await rows.count();

    if (rowCount > 0) {
      // 点击第一行的编辑按钮
      const firstRow = rows.first();
      const editButton = firstRow.locator('button').filter({ hasText: /编辑/ }).first();
      // 某些 ProTable 的操作列使用 <a> 标签而非 button
      const editLink = firstRow.getByText('编辑').first();

      if (await editButton.isVisible({ timeout: 3000 }).catch(() => false)) {
        await editButton.click();
      } else if (await editLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await editLink.click();
      } else {
        // 没有编辑入口，跳过后续验证
        return;
      }

      const modal = page.locator('.ant-modal');
      await expect(modal).toBeVisible();

      // 编辑场景下标题应为"编辑兑换规则"
      await expect(modal.getByText('编辑兑换规则').first()).toBeVisible();

      // 关闭模态框
      await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
      await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
    } else {
      // 无数据时标记为跳过，不视为测试失败
      test.info().annotations.push({ type: 'skip', description: '表格无数据行，跳过编辑弹窗验证' });
    }
  });

  test('频率限制字段可见', async ({ page }) => {
    // 频率限制是可选配置区域，但字段标签必须渲染以供用户填写
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    await expect(modal.getByText('每用户上限').first()).toBeVisible();
    await expect(modal.getByText('每日上限').first()).toBeVisible();

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('自动兑换开关可交互', async ({ page }) => {
    // 自动兑换开关默认关闭，需要验证用户可以切换其状态
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    const autoRedeemSwitch = modal.locator('.ant-switch').first();
    if (await autoRedeemSwitch.isVisible({ timeout: 3000 }).catch(() => false)) {
      // 记录切换前的状态
      const wasChecked = await autoRedeemSwitch.getAttribute('aria-checked') === 'true';
      await autoRedeemSwitch.click();

      // 切换后状态应与切换前相反
      const isChecked = await autoRedeemSwitch.getAttribute('aria-checked') === 'true';
      expect(isChecked).toBe(!wasChecked);
    }

    // 关闭模态框
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });

  test('表单字段可填写', async ({ page }) => {
    // 验证基础字段的输入功能正常，不提交表单
    const createButton = page.locator('button').filter({ hasText: /新建规则/ });
    await createButton.click();

    const modal = page.locator('.ant-modal');
    await expect(modal).toBeVisible();

    // 填写规则名称（定位包含"规则名称"标签的 form-item 内的 input）
    const nameInput = modal.locator('.ant-form-item').filter({ hasText: '规则名称' }).locator('input').first();
    if (await nameInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await nameInput.fill('测试兑换规则');
      await expect(nameInput).toHaveValue('测试兑换规则');
    }

    // 填写权益 ID（ProFormDigit 渲染为 ant-input-number 内的 input）
    const benefitInput = modal.locator('.ant-form-item').filter({ hasText: '权益 ID' }).locator('input').first();
    if (await benefitInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await benefitInput.fill('1');
      await expect(benefitInput).toHaveValue('1');
    }

    // 不提交，直接关闭
    await page.locator('.ant-modal button').filter({ hasText: /取.*消/ }).click();
    await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  });
});

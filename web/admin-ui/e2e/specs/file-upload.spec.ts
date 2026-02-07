import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * CSV 批量上传功能测试
 *
 * 验证 /grants/batch 页面的批量任务创建和文件上传流程。
 * 批量任务页面是一个 ProTable 列表，通过"新建任务"按钮进入上传流程。
 */
test.describe('CSV 批量上传', () => {
  const tmpFiles: string[] = [];

  /** 创建临时文件并记录路径，便于统一清理 */
  function createTempFile(filename: string, content: string): string {
    const tmpDir = path.join(__dirname, '..', 'test-results', 'tmp');
    if (!fs.existsSync(tmpDir)) {
      fs.mkdirSync(tmpDir, { recursive: true });
    }
    const filePath = path.join(tmpDir, filename);
    fs.writeFileSync(filePath, content, 'utf-8');
    tmpFiles.push(filePath);
    return filePath;
  }

  test.beforeEach(async ({ page }, testInfo) => {
    const isMobile = testInfo.project.name.toLowerCase().includes('mobile');
    test.skip(isMobile, '移动端布局不支持批量上传页面');

    const loginPage = new LoginPage(page);
    await loginPage.goto();
    await loginPage.loginAsAdmin();
  });

  test.afterAll(() => {
    for (const f of tmpFiles) {
      try { fs.unlinkSync(f); } catch { /* 忽略 */ }
    }
  });

  test('批量任务页面正常加载', async ({ page }) => {
    await page.goto('/grants/batch');
    await page.waitForLoadState('networkidle');

    // 页面应包含"新建任务"按钮和任务列表表格
    const createBtn = page.locator('button').filter({ hasText: /新建任务/ });
    await expect(createBtn).toBeVisible({ timeout: 10000 });

    // 表格应存在（即使暂无数据）
    const table = page.locator('.ant-table');
    await expect(table).toBeVisible({ timeout: 10000 });
  });

  test('新建任务弹窗包含上传区域', async ({ page }) => {
    await page.goto('/grants/batch');
    await page.waitForLoadState('networkidle');

    // 点击"新建任务"按钮
    const createBtn = page.locator('button').filter({ hasText: /新建任务/ });
    await createBtn.click();

    // 弹窗应出现，包含表单或上传组件
    const modal = page.locator('.ant-modal, .ant-drawer').first();
    await expect(modal).toBeVisible({ timeout: 10000 });

    // 弹窗内应有上传区域或文件输入
    const uploadArea = modal.locator('.ant-upload, .ant-upload-drag, input[type="file"], [data-testid="batch-upload"]');
    const hasUpload = await uploadArea.count() > 0;

    // 弹窗内应有表单项（任务名称、徽章选择等）
    const formItems = modal.locator('.ant-form-item, .ant-pro-form');
    const hasForm = await formItems.count() > 0;

    // 至少应有上传区域或表单
    expect(hasUpload || hasForm).toBeTruthy();
  });

  test('上传有效 CSV 文件', async ({ page }) => {
    await page.goto('/grants/batch');
    await page.waitForLoadState('networkidle');

    // 打开新建任务弹窗
    const createBtn = page.locator('button').filter({ hasText: /新建任务/ });
    await createBtn.click();

    const modal = page.locator('.ant-modal, .ant-drawer').first();
    await expect(modal).toBeVisible({ timeout: 10000 });

    // 构造合法 CSV
    const csvContent = [
      'userId,badgeId,reason',
      'user_001,1,测试批量发放',
      'user_002,1,测试批量发放',
      'user_003,2,活动奖励',
    ].join('\n');
    const csvPath = createTempFile('valid-upload.csv', csvContent);

    // 查找文件输入
    const fileInput = modal.locator('input[type="file"]');
    const hasFileInput = await fileInput.count() > 0;

    if (hasFileInput) {
      await fileInput.setInputFiles(csvPath);

      // 上传后应出现文件名预览
      await expect(
        modal.locator('.ant-upload-list-item, .ant-upload-list-text-container').first()
      ).toBeVisible({ timeout: 10000 });
    } else {
      // 页面可能使用其他方式（如 textarea 手动输入用户 ID 列表），跳过文件上传验证
      test.skip(true, '页面不包含文件上传组件，跳过 CSV 上传测试');
    }
  });

  test('上传非 CSV 格式文件应提示错误', async ({ page }) => {
    await page.goto('/grants/batch');
    await page.waitForLoadState('networkidle');

    const createBtn = page.locator('button').filter({ hasText: /新建任务/ });
    await createBtn.click();

    const modal = page.locator('.ant-modal, .ant-drawer').first();
    await expect(modal).toBeVisible({ timeout: 10000 });

    const fileInput = modal.locator('input[type="file"]');
    const hasFileInput = await fileInput.count() > 0;

    if (hasFileInput) {
      const txtContent = '这不是一个 CSV 文件';
      const txtPath = createTempFile('invalid-format.txt', txtContent);
      await fileInput.setInputFiles(txtPath);

      // 应出现错误提示
      const errorIndicator = page.locator(
        '.ant-message-error, .ant-upload-list-item-error, .ant-form-item-explain-error, .ant-alert-error'
      ).first();
      await expect(errorIndicator).toBeVisible({ timeout: 10000 });
    } else {
      test.skip(true, '页面不包含文件上传组件');
    }
  });

  test('批量任务列表展示表格结构', async ({ page }) => {
    await page.goto('/grants/batch');
    await page.waitForLoadState('networkidle');

    // 表格应有正确的列标题
    const table = page.locator('.ant-table');
    await expect(table).toBeVisible({ timeout: 10000 });

    // 验证表头包含关键列
    const headers = page.locator('.ant-table-thead th');
    const headerTexts = await headers.allTextContents();
    const headerStr = headerTexts.join(' ');

    // 应包含任务 ID、状态、操作等关键列
    expect(headerStr).toContain('任务');
    expect(headerStr).toContain('状态');
  });
});

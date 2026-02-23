import { expect, test } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const aicTomlPath = fileURLToPath(new URL('../../aic.toml', import.meta.url));

test('捕获浏览器控制台报错', async ({ page }) => {
  const errors: Array<{ type: string; text: string; location?: string }> = [];
  const warnings: Array<{ type: string; text: string }> = [];

  const printCollectedLogs = async (phase: string) => {
    const currentUrl = page.url();
    const fileInputCount = await page.locator('input[type="file"]').count();
    const headingCount = await page.getByRole('heading').count();

    console.log(`\n========== 浏览器日志报告 (${phase}) ==========`);
    console.log(`当前 URL: ${currentUrl}`);
    console.log(`heading 数量: ${headingCount}`);
    console.log(`file input 数量: ${fileInputCount}`);

    if (warnings.length > 0) {
      console.log(`\n⚠️  发现 ${warnings.length} 个警告:`);
      warnings.forEach((warn, i) => {
        console.log(`  ${i + 1}. ${warn.text.substring(0, 200)}${warn.text.length > 200 ? '...' : ''}`);
      });
    }

    if (errors.length > 0) {
      console.log(`\n❌ 发现 ${errors.length} 个错误:`);
      errors.forEach((err, i) => {
        console.log(`\n  [${i + 1}] ${err.type}:`);
        console.log(`      ${err.text.substring(0, 300)}${err.text.length > 300 ? '...' : ''}`);
        if (err.location) {
          console.log(`      Location: ${err.location.substring(0, 200)}...`);
        }
      });
    }

    if (errors.length === 0 && warnings.length === 0) {
      console.log('✅ 未发现任何错误或警告');
    }

    console.log('=====================================\n');
  };

  // 监听控制台消息
  page.on('console', msg => {
    const type = msg.type();
    const text = msg.text();
    
    if (type === 'error') {
      errors.push({ type: 'console.error', text });
    } else if (type === 'warning') {
      warnings.push({ type: 'console.warn', text });
    }
  });

  // 监听页面 JavaScript 错误
  page.on('pageerror', err => {
    errors.push({ type: 'pageerror', text: err.message, location: err.stack });
  });

  // 监听请求失败
  page.on('requestfailed', request => {
    errors.push({ 
      type: 'requestfailed', 
      text: `请求失败: ${request.url()} - ${request.failure()?.errorText || '未知错误'}` 
    });
  });

  let testFailure: unknown = null;

  try {
    // 访问页面
    await page.goto('/');

    // 等待页面完全加载
    await page.waitForLoadState('networkidle');

    // 先确认上传入口存在，避免直接卡死在 setInputFiles
    const fileInput = page.locator('input[type="file"]');
    await expect(fileInput).toHaveCount(1, { timeout: 8_000 });

    // 上传 aic.toml 文件
    await fileInput.setInputFiles(aicTomlPath);

    // 等待上传处理完成
    await page.waitForTimeout(2000);

    // 额外等待一段时间以捕获异步错误
    await page.waitForTimeout(10_000);
  } catch (error) {
    testFailure = error;
  } finally {
    await printCollectedLogs(testFailure ? '失败中断' : '正常结束');
  }

  if (testFailure) {
    throw testFailure;
  }

  // 断言：不应该有错误
  expect(errors).toHaveLength(0);
});

import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { deflateSync } from 'node:zlib';

function encodeToml(toml: string): string {
  const compressed = deflateSync(Buffer.from(toml, 'utf8'));
  return compressed.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

test('share param should auto solve without user edits', async ({ page }) => {
  const here = dirname(fileURLToPath(import.meta.url));
  const tomlPath = resolve(here, '../../crates/end_io/src/aic.toml');
  const toml = readFileSync(tomlPath, 'utf8');
  const encoded = encodeToml(toml);

  await page.goto(`/?s=${encoded}`);

  await expect(page.getByRole('heading', { name: /Plan Summary|方案评估/ })).toBeVisible();
  await expect(page.getByText(/Revenue \/ min|收益 \/ min/)).toBeVisible({ timeout: 20_000 });
});

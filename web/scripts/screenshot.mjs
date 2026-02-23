#!/usr/bin/env node
import { chromium, firefox, webkit } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';

const HELP = `Usage:
  npm run screenshot -- --url <url> [options]

Required:
  --url <url>                  Target page URL

Options:
  --out <path>                 Output image path (default: screenshots/screenshot-<timestamp>.png)
  --selector <css>             Capture only this element instead of full page
  --full-page                  Capture full page (default when no selector)
  --wait-ms <number>           Wait milliseconds before capture (default: 0)
  --wait-for <css>             Wait for selector before capture
  --timeout <number>           Timeout in ms (default: 30000)
  --viewport <WxH>             Viewport size, e.g. 1440x900 (default: 1440x900)
  --browser <name>             chromium | firefox | webkit (default: firefox)
  --no-headless                Run browser in headed mode
  --help                       Show help

Examples:
  npm run screenshot -- --url http://127.0.0.1:4173
  npm run screenshot -- --url http://127.0.0.1:4173 --out screenshots/home.png --full-page
  npm run screenshot -- --url http://127.0.0.1:4173 --selector '#app'
`;

function parseArgs(argv) {
  const args = {
    url: '',
    out: '',
    selector: '',
    fullPage: false,
    waitMs: 0,
    waitFor: '',
    timeout: 30_000,
    viewport: { width: 1440, height: 900 },
    browser: 'firefox',
    headless: true,
    help: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (token === '--help' || token === '-h') {
      args.help = true;
      continue;
    }

    if (token === '--full-page') {
      args.fullPage = true;
      continue;
    }

    if (token === '--no-headless') {
      args.headless = false;
      continue;
    }

    if (!token.startsWith('--')) {
      throw new Error(`Unexpected argument: ${token}`);
    }

    const value = argv[index + 1];
    if (value === undefined || value.startsWith('--')) {
      throw new Error(`Missing value for ${token}`);
    }
    index += 1;

    switch (token) {
      case '--url':
        args.url = value;
        break;
      case '--out':
        args.out = value;
        break;
      case '--selector':
        args.selector = value;
        break;
      case '--wait-ms':
        args.waitMs = Number(value);
        break;
      case '--wait-for':
        args.waitFor = value;
        break;
      case '--timeout':
        args.timeout = Number(value);
        break;
      case '--viewport': {
        const [width, height] = value.toLowerCase().split('x').map(Number);
        args.viewport = { width, height };
        break;
      }
      case '--browser':
        args.browser = value;
        break;
      default:
        throw new Error(`Unknown option: ${token}`);
    }
  }

  return args;
}

function assertValidArgs(args) {
  if (!args.url) {
    throw new Error('Missing --url');
  }

  try {
    new URL(args.url);
  } catch {
    throw new Error(`Invalid URL: ${args.url}`);
  }

  const validBrowsers = new Set(['chromium', 'firefox', 'webkit']);
  if (!validBrowsers.has(args.browser)) {
    throw new Error(`Invalid --browser: ${args.browser}`);
  }

  if (!Number.isFinite(args.waitMs) || args.waitMs < 0) {
    throw new Error(`Invalid --wait-ms: ${args.waitMs}`);
  }

  if (!Number.isFinite(args.timeout) || args.timeout <= 0) {
    throw new Error(`Invalid --timeout: ${args.timeout}`);
  }

  if (!Number.isInteger(args.viewport.width) || !Number.isInteger(args.viewport.height) || args.viewport.width <= 0 || args.viewport.height <= 0) {
    throw new Error(`Invalid --viewport: ${args.viewport.width}x${args.viewport.height}`);
  }
}

function resolveBrowserLauncher(name) {
  if (name === 'chromium') {
    return chromium;
  }
  if (name === 'webkit') {
    return webkit;
  }
  return firefox;
}

function defaultOutputPath() {
  const stamp = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
  return path.resolve('screenshots', `screenshot-${stamp}.png`);
}

async function run() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    process.stdout.write(HELP);
    return;
  }

  assertValidArgs(args);

  const outputPath = args.out ? path.resolve(args.out) : defaultOutputPath();
  await fs.mkdir(path.dirname(outputPath), { recursive: true });

  const launcher = resolveBrowserLauncher(args.browser);
  const browser = await launcher.launch({ headless: args.headless });

  try {
    const page = await browser.newPage({ viewport: args.viewport });
    await page.goto(args.url, { waitUntil: 'networkidle', timeout: args.timeout });

    await page.evaluate(async () => {
      if (typeof document !== 'undefined' && 'fonts' in document) {
        await document.fonts.ready;
      }
    });
    await page.waitForTimeout(50);

    if (args.waitFor) {
      await page.waitForSelector(args.waitFor, { timeout: args.timeout, state: 'visible' });
    }

    if (args.waitMs > 0) {
      await page.waitForTimeout(args.waitMs);
    }

    if (args.selector) {
      const element = page.locator(args.selector).first();
      await element.waitFor({ timeout: args.timeout, state: 'visible' });
      await element.screenshot({ path: outputPath });
    } else {
      await page.screenshot({ path: outputPath, fullPage: args.fullPage || true });
    }

    process.stdout.write(`${outputPath}\n`);
  } finally {
    await browser.close();
  }
}

run().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});

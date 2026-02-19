import { expect, test } from '@playwright/test';

const wasmShim = `
(() => {
  const encoder = new TextEncoder();
  const memory = new ArrayBuffer(2 * 1024 * 1024);
  const HEAPU8 = new Uint8Array(memory);
  const HEAPU32 = new Uint32Array(memory);
  let nextPtr = 8;
  const SLICE_SIZE = 12;

  const align4 = (value) => (value + 3) & ~3;
  const malloc = (size) => {
    const requested = Number(size) || 0;
    if (requested <= 0) {
      return 0;
    }

    const ptr = nextPtr;
    nextPtr = align4(nextPtr + requested);
    if (nextPtr > HEAPU8.length) {
      throw new Error('mock wasm heap exhausted');
    }
    return ptr;
  };

  const writeEnvelope = (envelope) => {
    const payload = JSON.stringify(envelope);
    const bytes = encoder.encode(payload);
    const strPtr = malloc(bytes.length);
    HEAPU8.set(bytes, strPtr);

    const slicePtr = malloc(SLICE_SIZE);
    const base = slicePtr >>> 2;
    HEAPU32[base] = strPtr;
    HEAPU32[base + 1] = bytes.length;
    HEAPU32[base + 2] = bytes.length;
    return slicePtr;
  };

  const bootstrap = {
    status: 'ok',
    data: {
      defaultAicToml: [
        'external_power_consumption_w = 120',
        '',
        '[supply_per_min]',
        '"Originium Ore" = 40',
        '',
        '[[outposts]]',
        'key = "Refugee Camp"',
        'en = "Refugee Camp"',
        'zh = "难民营"',
        'money_cap_per_hour = 12000',
        '[outposts.prices]',
        '"SC Valley Battery" = 30'
      ].join('\\n'),
      catalog: {
        items: [
          { key: 'Originium Ore', en: 'Originium Ore', zh: '源石矿' },
          { key: 'SC Valley Battery', en: 'SC Valley Battery', zh: 'SC 山谷电池' }
        ]
      }
    }
  };

  const solved = {
    status: 'ok',
    data: {
      reportText: 'ok',
      summary: {
        lang: 'en',
        stage1RevenuePerMin: 10,
        stage2RevenuePerMin: 12.34,
        stage2RevenuePerHour: 740,
        totalMachines: 9,
        totalThermalBanks: 1,
        powerGenW: 450,
        powerUseW: 420,
        powerMarginW: 30,
        outposts: [
          {
            key: 'Refugee Camp',
            name: 'Refugee Camp',
            valuePerMin: 12.34,
            capPerMin: 20,
            ratio: 0.617
          }
        ],
        topSales: [
          {
            outpostKey: 'Refugee Camp',
            outpostName: 'Refugee Camp',
            itemKey: 'SC Valley Battery',
            itemName: 'SC Valley Battery',
            valuePerMin: 12.34
          }
        ],
        facilities: [{ key: 'Assembler', name: 'Assembler', machines: 4 }],
        externalSupplySlack: []
      },
      logisticsGraph: {
        items: [],
        nodes: [],
        edges: []
      }
    }
  };

  globalThis.createEndWebModule = async () => ({
    HEAPU8,
    HEAPU32,
    ccall(ident, returnType, argTypes, args) {
      if (ident === 'malloc') {
        return malloc(args[0]);
      }

      if (ident === 'free' || ident === 'end_web_free_slice') {
        return undefined;
      }

      const envelope =
        ident === 'end_web_bootstrap'
          ? bootstrap
          : ident === 'end_web_solve_from_aic_toml'
            ? solved
            : { status: 'err', error: { message: 'unknown wasm call: ' + ident } };

      return writeEnvelope(envelope);
    }
  });
})();
`;

test('workspace boots and auto solve produces result panels', async ({ page }) => {
  await page.context().route('**/wasm/end_web.js*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/javascript',
      body: wasmShim
    });
  });

  await page.goto('/');

  await expect(page.getByRole('heading', { name: /Configuration Editor|配置编辑器/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Solver Output|求解结果/ })).toBeVisible();

  const externalPowerInput = page.locator('#external-power');
  await expect(externalPowerInput).toBeVisible();
  await externalPowerInput.fill('321');

  await expect(page.getByText(/Revenue \/ min|收益 \/ min/)).toBeVisible({ timeout: 60_000 });
  await expect(page.getByText(/Logistics Graph|物流图/)).toBeVisible();
});

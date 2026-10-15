import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: session estimates and visible chart ranges on touch and keyboard`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      let dense = false;
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          const readings: Record<string, number[]> = {
            controllerTempC: [25, 35],
            motorTempC: [20, 50],
            dutyCycle: [0, 0.42],
            motorLoad: [0, 0.6],
            batteryLoad: [0, 0.75],
            estimatedBatteryLevel: [0.61, 0.55],
          };
          for (const [field, column] of Object.entries(value.columns) as [string, { values: (number | null)[] }][]) {
            const finite = column.values.filter((v): v is number => v != null && Number.isFinite(v));
            Object.assign(column, {
              elapsedMs: [0, 60000],
              sequence: [0, 1],
              generations: [0, 0],
              values: readings[field] ?? [finite[0] ?? null, finite.at(-1) ?? null],
            });
          }
          if (dense && value.columns.motorLoad) {
            Object.assign(value.columns.motorLoad, {
              elapsedMs: [0, 1000, 2000, 60000],
              sequence: [0, 1, 2, 3],
              generations: [0, 0, 0, 0],
              values: [0, 0.75, 0, 0],
            });
          }
        }
        await route.fulfill({ response, json: value });
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      await page.getByRole('img', { name: 'Estimated battery level over elapsed time', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
      const mode = page.getByRole('button', { name: /^Chart mode:/ });
      const screenshots = process.env.FLOAT_VIEW_SCREENSHOTS;
      if (screenshots && name === 'chromium') await mkdir(screenshots, { recursive: true });
      for (const theme of ['light', 'dark']) {
        await page.evaluate((theme) => (document.documentElement.dataset.theme = theme), theme);
        const colours = new Set<string>();
        for (const current of ['info', 'zoom', 'pan']) {
          assert.equal(await mode.getAttribute('data-mode'), current);
          const appearance = await mode.evaluate((node) => {
            const style = getComputedStyle(node);
            const canvas = document.createElement('canvas');
            const context = canvas.getContext('2d')!;
            const luminance = (colour: string) => {
              context.fillStyle = colour;
              context.fillRect(0, 0, 1, 1);
              const rgb = Array.from(context.getImageData(0, 0, 1, 1).data)
                .slice(0, 3)
                .map((v) => {
                  const s = v / 255;
                  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
                });
              return rgb[0]! * 0.2126 + rgb[1]! * 0.7152 + rgb[2]! * 0.0722;
            };
            const foreground = luminance(style.color);
            const background = luminance(style.backgroundColor);
            return {
              colour: style.backgroundColor,
              contrast: (Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05),
              icon: getComputedStyle(node.querySelector('span')!).maskImage,
            };
          });
          assert.ok(appearance.contrast >= 4.5, `${theme} ${current} label has readable contrast`);
          assert.notEqual(appearance.icon, 'none', `${current} has a visible mode icon`);
          colours.add(appearance.colour);
          if (screenshots && name === 'chromium') {
            await page.screenshot({
              path: `${screenshots}/chart-mode-${theme}-${current}.png`,
              clip: { x: 270, y: 724, width: 120, height: 120 },
            });
            if (current === 'info') await page.screenshot({ path: `${screenshots}/chart-modes-${theme}.png` });
          }
          await mode.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 42 });
          await mode.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 42 });
          assert.equal(await mode.getAttribute('data-mode'), current, 'secondary activation preserves mode');
          if (current === 'zoom') {
            await mode.focus();
            await page.keyboard.press('Enter');
          } else {
            await mode.tap();
          }
        }
        assert.equal(colours.size, 3, `${theme} modes have distinct colours`);
        assert.equal(await mode.getAttribute('data-mode'), 'info', 'each activation changes mode exactly once');
      }
      const battery = page.locator('[aria-label="Estimated battery level range"]');
      await battery.getByText('61%', { exact: true }).waitFor();
      assert.match(await battery.innerText(), /Min 55%\s+Max 61%/);
      const temperature = page.locator('[aria-label="Temperature range"]');
      await temperature.scrollIntoViewIfNeeded();
      await temperature.getByText('50.0 °C', { exact: true }).waitFor();
      assert.match(await temperature.innerText(), /Min 20\.0 °C\s+Max 50\.0 °C/);
      const load = page.locator('[aria-label="Load range"]');
      assert.equal(await load.innerText(), 'Max 75%');
      const assertChartLayout = async () => {
        for (const chart of await page.getByRole('img', { name: /over elapsed time$/ }).all()) {
          const bounds = (await chart.boundingBox())!;
          assert.ok(bounds.height >= 200 && bounds.height <= 300, 'charts have readable, bounded vertical space');
          assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= page.viewportSize()!.width, 'chart fits the screen');
          const frame = chart.locator('..').locator('..');
          const labels = frame.locator('[aria-hidden="true"]').first();
          assert.ok(
            await labels.evaluate((node) => parseFloat(getComputedStyle(node).fontSize) >= 10),
            'axis labels remain legible on phones'
          );
        }
      };
      await assertChartLayout();
      if (screenshots && name === 'chromium') {
        await mkdir(screenshots, { recursive: true });
        await page
          .getByRole('img', { name: 'Estimated battery level over elapsed time', exact: true })
          .scrollIntoViewIfNeeded();
        await page.screenshot({ path: `${screenshots}/session-battery-mobile.png` });
        await load.scrollIntoViewIfNeeded();
        await page.screenshot({ path: `${screenshots}/session-load-mobile.png` });
        await temperature.scrollIntoViewIfNeeded();
        await page.screenshot({ path: `${screenshots}/session-ranges-all-lines.png` });
      }
      const motor = page.getByRole('button', { name: /^(Hide|Show) Motor temp line$/ });
      await motor.dispatchEvent('pointerdown', { pointerType: 'mouse', button: 2, isPrimary: true });
      assert.match(await temperature.innerText(), /Min 20\.0 °C\s+Max 50\.0 °C/);
      await motor.tap();
      assert.match(await temperature.innerText(), /Min 25\.0 °C\s+Max 35\.0 °C/);
      if (screenshots && name === 'chromium')
        await page.screenshot({ path: `${screenshots}/session-ranges-motor-hidden.png` });
      await motor.focus();
      await page.keyboard.press('Enter');
      assert.match(await temperature.innerText(), /Min 20\.0 °C\s+Max 50\.0 °C/);
      const batteryLoad = page.getByRole('button', { name: /^(Hide|Show) Battery load line$/ });
      await batteryLoad.tap();
      assert.equal(await load.innerText(), 'Max 60%');
      await page.getByRole('button', { name: /^(Hide|Show) Motor load line$/ }).tap();
      assert.equal(await load.innerText(), 'Max 42%');
      await page.getByRole('button', { name: /^(Hide|Show) Duty cycle line$/ }).tap();
      assert.equal(await load.innerText(), 'Max —');
      await page.getByRole('button', { name: /^(Hide|Show) Duty cycle line$/ }).tap();
      await page.getByRole('button', { name: /^(Hide|Show) Motor load line$/ }).tap();
      await page.getByRole('button', { name: /^(Hide|Show) Battery load line$/ }).tap();
      await page.setViewportSize({ width: 1024, height: 900 });
      await assertChartLayout();
      await temperature.scrollIntoViewIfNeeded();
      await page.locator('[aria-label="IMU range"]').getByText('1.4 deg', { exact: true }).waitFor();
      if (screenshots && name === 'chromium')
        await page.screenshot({ path: `${screenshots}/session-ranges-desktop.png` });
      dense = true;
      await page.setViewportSize({ width: 390, height: 844 });
      const loadChart = page.getByRole('img', { name: 'Load over elapsed time', exact: true });
      await loadChart.locator('[data-history-band]').waitFor({ state: 'attached' });
      assert.equal(await load.innerText(), 'Max 75%', 'smoothing keeps the recorded spike in the range label');
      await page
        .getByRole('button', { name: /Chart mode: info/ })
        .last()
        .click();
      await loadChart.scrollIntoViewIfNeeded();
      const bounds = (await loadChart.boundingBox())!;
      const zoomResponse = page.waitForResponse((response) => {
        if (!response.url().endsWith('/api/session-data')) return false;
        const request = response.request().postDataJSON();
        return request.kind === 'series' && request.range.startElapsedMs > 0 && request.range.endElapsedMs < 60000;
      });
      for (const [type, ratio] of [
        ['pointerdown', 0.1],
        ['pointermove', 0.5],
        ['pointerup', 0.5],
      ] as const) {
        await loadChart.dispatchEvent(type, {
          pointerType: 'touch',
          pointerId: 1,
          isPrimary: true,
          button: 0,
          clientX: bounds.x + bounds.width * ratio,
          clientY: bounds.y + bounds.height / 2,
        });
      }
      await zoomResponse;
      await loadChart.locator('[data-history-band]').waitFor({ state: 'detached' });
    } finally {
      for (const context of browser.contexts()) {
        for (const page of context.pages()) await page.unrouteAll({ behavior: 'ignoreErrors' });
      }
      await browser.close();
      await stopMockServer();
    }
  });
}

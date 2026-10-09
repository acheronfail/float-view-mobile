import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: live electrical readings preserve distance and animate signed power`,
    { timeout: 60_000 },
    async () => {
      const port = await startMockServer(11002);
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
        await client.request('device.connect', {
          deviceId: scanned.discoveredDevices[0]!.deviceId,
          trackingMode: 'saved',
        });
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
        page.setDefaultTimeout(5000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        let fw: number | null = 0.7;
        let amps: number | null = 12;
        let rejectSelection = false;
        let saves = 0;
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'settings.update' && message.params?.patch?.liveWidgetSlots) {
              saves++;
              if (rejectSelection) {
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Test save failure' } })
                );
                return;
              }
            }
            server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state =
              message.event === 'state.changed'
                ? message.data
                : message.result?.activeConnection
                  ? message.result
                  : null;
            if (state?.activeConnection?.latestSample)
              Object.assign(state.activeConnection.latestSample, {
                fieldWeakeningCurrentA: fw,
                batteryCurrentA: amps,
                motorCurrentA: 32.4,
                packVoltageV: 75,
                speedKph: 24.8,
                dutyCycle: 0.42,
                distanceM: 12400,
                footpadState: 'both',
              });
            socket.send(JSON.stringify(message));
          });
        });
        await page.goto(`${endpoint.origin}/#/live`);
        const field = page.getByRole('group', { name: 'Field weakening', exact: true });
        const power = page.getByRole('group', { name: 'Battery power', exact: true });
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('+900')
        );
        assert.match(await field.innerText(), /0\.7/);
        assert.equal(await page.getByText('DISTANCE', { exact: true }).count(), 1);
        await page.emulateMedia({ colorScheme: 'dark' });
        await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
        const drawColor = await power
          .locator('div')
          .last()
          .evaluate((el) => getComputedStyle(el).color);
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          await page.waitForTimeout(200);
          if (process.env.SCREENSHOT_DIR) {
            await mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/live-${name}-${theme}-draw.png` });
          }
        }
        // Observe intermediate rendered values, not just arrival at the target.
        await field.evaluate((el) => {
          (window as any).fwFrames = [];
          new MutationObserver(() => (window as any).fwFrames.push(el.textContent)).observe(el, {
            subtree: true,
            characterData: true,
            childList: true,
          });
        });
        fw = 8;
        amps = -4;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('-300')
        );
        assert.match(await power.innerText(), /Regen/i);
        assert(await page.evaluate(() => (window as any).fwFrames.some((text: string) => /[1-7]\.\d/.test(text))));
        assert.notEqual(
          await power
            .locator('div')
            .last()
            .evaluate((el) => getComputedStyle(el).color),
          drawColor
        );
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          await page.waitForTimeout(200);
          if (process.env.SCREENSHOT_DIR)
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/live-${name}-${theme}-regen.png` });
        }
        await page.emulateMedia({ reducedMotion: 'reduce' });
        fw = 0;
        amps = 0;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Field weakening"]')?.textContent?.includes('0.0')
        );
        assert.equal(await page.getByText('DISTANCE', { exact: true }).count(), 1);
        fw = null;
        amps = null;
        await page.waitForFunction(() =>
          document.querySelector('[aria-label="Battery power"]')?.textContent?.includes('—')
        );
        assert.match(await field.innerText(), /—/);
        for (const viewport of [
          { width: 320, height: 568 },
          { width: 1280, height: 900 },
        ]) {
          await page.setViewportSize(viewport);
          assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          const gauge = await page.getByRole('img', { name: 'Live speed and drive load gauge' }).boundingBox();
          assert(gauge && gauge.height > 150);
        }
        await page.setViewportSize({ width: 390, height: 844 });
        fw = 0.7;
        amps = 12;
        const firstBox = page.getByRole('button', { name: 'Choose value for box 1: Field weakening', exact: true });
        await firstBox.click({ button: 'right' });
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 99 });
        await firstBox.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 99 });
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.evaluate((el) =>
          el.addEventListener(
            'pointerdown',
            (event) => el.setAttribute('data-pointer-id', String((event as PointerEvent).pointerId)),
            { once: true }
          )
        );
        await firstBox.hover();
        await page.mouse.down();
        await firstBox.dispatchEvent('pointercancel', {
          pointerId: Number(await firstBox.getAttribute('data-pointer-id')),
        });
        await page.mouse.up();
        assert.equal(await page.getByRole('dialog').count(), 0);
        await firstBox.tap();
        const picker = page.getByRole('dialog', { name: 'Choose a value' });
        await picker.waitFor();
        assert.equal(
          await picker.getByRole('button', { name: 'Field weakening', exact: true }).getAttribute('aria-pressed'),
          'true'
        );
        assert.equal(await picker.getByRole('button', { name: 'Charging voltage', exact: true }).count(), 1);
        for (const theme of ['light', 'dark'] as const) {
          await page.emulateMedia({ colorScheme: theme });
          await page.waitForFunction((theme) => document.documentElement.dataset.theme === theme, theme);
          if (process.env.SCREENSHOT_DIR)
            await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/picker-${name}-${theme}.png` });
        }
        await page.keyboard.press('Escape');
        await picker.waitFor({ state: 'hidden' });
        assert.equal(await firstBox.evaluate((el) => el === document.activeElement), true);
        await page.keyboard.press('Enter');
        await picker.waitFor();
        rejectSelection = true;
        await picker.getByRole('button', { name: 'Pack voltage', exact: true }).tap();
        await picker.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
        assert.equal((await client.request('state.get', {})).settings.liveWidgetSlots[0], 'fieldWeakeningCurrentA');
        rejectSelection = false;
        await picker.getByRole('button', { name: 'Pack voltage', exact: true }).tap();
        await picker.waitFor({ state: 'hidden' });
        assert.equal(saves, 2, 'one request per activation, including the rejected save');
        assert.match(
          await page.getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true }).innerText(),
          /75.0/
        );
        await page.reload();
        const voltageBox = page.getByRole('button', { name: 'Choose value for box 1: Pack voltage', exact: true });
        await voltageBox.waitFor();
        assert.match(await voltageBox.innerText(), /75.0/);
        const secondBox = page.getByRole('button', {
          name: 'Choose value for box 2: Actual motor current',
          exact: true,
        });
        await secondBox.focus();
        await page.keyboard.press('Space');
        await picker.waitFor();
        await picker.getByRole('button', { name: 'Charging voltage', exact: true }).scrollIntoViewIfNeeded();
        await picker.getByRole('button', { name: 'Charging voltage', exact: true }).focus();
        await page.keyboard.press('Enter');
        await picker.waitFor({ state: 'hidden' });
        assert.equal(saves, 3);
        assert.equal((await client.request('state.get', {})).settings.liveWidgetSlots[1], 'chargingVoltageV');
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}

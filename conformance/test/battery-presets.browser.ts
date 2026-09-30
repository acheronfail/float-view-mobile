import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: named battery setup preserves drafts and atomic selections`, { timeout: 60_000 }, async () => {
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
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const id = ready.activeConnection!.savedDeviceId!;
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const patches: Record<string, unknown>[] = [];
      const errors: string[] = [];
      let reject = false;
      page.on('pageerror', (e) => errors.push(e.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const m = JSON.parse(String(raw));
          if (m.method === 'boardProfile.update') {
            patches.push(m.params.patch);
            if (reject) {
              socket.send(
                JSON.stringify({ id: m.id, error: { code: 'invalid_request', message: 'Preset save failed' } })
              );
              return;
            }
          }
          server.send(raw);
        });
        server.onMessage((raw) => socket.send(raw));
      });
      const saved = async (predicate: (v: Record<string, unknown>) => boolean) => {
        for (let i = 0; i < 100; i++) {
          const p = await client.request('boardProfile.get', { savedDeviceId: id });
          if (predicate(p.values)) return p;
          await page.waitForTimeout(30);
        }
        assert.fail('Profile was not saved');
      };
      await page.goto(`${endpoint.origin}/#/live`);
      await page.getByRole('button', { name: 'Battery setup required', exact: true }).tap();
      const dialog = page.getByRole('dialog');
      const basic = page.getByRole('button', { name: 'Basic', exact: true });
      const advanced = page.getByRole('button', { name: 'Advanced', exact: true });
      const select = page.getByRole('combobox', { name: 'Battery', exact: true });
      await select.waitFor();
      assert.equal(await basic.getAttribute('aria-pressed'), 'true');
      await advanced.click({ button: 'right' });
      assert.equal(await basic.getAttribute('aria-pressed'), 'true');
      await advanced.evaluate((el) =>
        el.addEventListener(
          'pointerdown',
          (e) => el.setAttribute('data-pointer', String((e as PointerEvent).pointerId)),
          { once: true }
        )
      );
      await advanced.hover();
      await page.mouse.down();
      await advanced.dispatchEvent('pointercancel', { pointerId: Number(await advanced.getAttribute('data-pointer')) });
      await page.mouse.up();
      assert.equal(await basic.getAttribute('aria-pressed'), 'true');
      await advanced.focus();
      await page.keyboard.press('Enter');
      const series = page.getByLabel('Battery series cells (S)', { exact: true });
      await series.fill('99');
      const before = patches.length;
      await basic.tap();
      await advanced.tap();
      await series.waitFor().catch(async (e) => {
        console.error(await page.locator('body').innerText(), errors);
        throw e;
      });
      assert.equal(await series.inputValue(), '99');
      assert.equal(patches.length, before);
      await series.fill('18');
      await basic.tap();
      assert.equal(patches.length, before);
      reject = true;
      await select.selectOption('floatwheel-adv2-samsung-50s');
      await dialog.getByRole('alert').filter({ hasText: 'Preset save failed' }).waitFor();
      assert.equal((await client.request('boardProfile.get', { savedDeviceId: id })).values.batteryPresetId, undefined);
      reject = false;
      // A known curve followed rapidly by a generic mapping; selection requests contain only identity.
      await select.selectOption('floatwheel-adv2-samsung-50s');
      await select.selectOption('chi-chi-ve-xr-18-2-molicel-p50b');
      await saved((v) => v.batteryPresetId === 'chi-chi-ve-xr-18-2-molicel-p50b');
      assert.ok(patches.every((p) => Object.keys(p).length === 1 && 'batteryPresetId' in p));
      assert.ok(!/generic|approximation/i.test(await dialog.innerText()));
      await client.waitForState((s) => s.activeConnection?.batteryEstimate.level != null);
      assert.equal(await page.getByRole('button', { name: 'Battery setup required', exact: true }).count(), 0);
      await advanced.tap();
      assert.equal(await page.getByLabel('Battery cell profile', { exact: true }).inputValue(), 'generic-linear');
      const capacity = page.getByLabel('Pack capacity (Ah, optional)', { exact: true });
      assert.equal(await capacity.inputValue(), '10');
      await capacity.fill('8.7');
      await capacity.press('Tab');
      await saved((v) => v.batteryCapacityAh === 8.7);
      await basic.tap();
      assert.match(await page.getByLabel('Battery configuration summary').innerText(), /8.7 Ah/);
      await advanced.tap();
      await capacity.fill('');
      await capacity.press('Tab');
      await saved((v) => v.batteryCapacityAh === null);
      await basic.tap();
      assert.match(await page.getByLabel('Battery configuration summary').innerText(), /Capacity not set/);
      await dialog.getByRole('button', { name: 'Done', exact: true }).click();
      await page.reload();
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Configure board', exact: true }).click();
      await select.waitFor();
      assert.equal(await select.inputValue(), 'chi-chi-ve-xr-18-2-molicel-p50b');
      await advanced.tap();
      assert.equal(await capacity.inputValue(), '');
      await page.getByRole('button', { name: 'Reset to battery defaults', exact: true }).tap();
      await saved((v) => v.batteryCapacityAh === 10);
      assert.equal(await capacity.inputValue(), '10');
      await series.fill('19');
      await series.press('Tab');
      await saved((v) => v.seriesCount === 19 && v.batteryCapacityAh === null && v.batteryPresetId === null);
      assert.equal(await capacity.inputValue(), '');
      await basic.tap();
      assert.equal(await select.inputValue(), 'custom');
      const customBefore = patches.length;
      await select.selectOption('custom');
      await series.waitFor();
      assert.equal(patches.length, customBefore);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}

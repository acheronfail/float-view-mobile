import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: Tune presets save reliably and switch immediately by touch or keyboard`,
    { timeout: 60000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
        await client.request('device.connect', {
          deviceId: scan.discoveredDevices[0]!.deviceId,
          trackingMode: 'saved',
        });
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        const context = await browser.newContext({ viewport: { width: 390, height: 700 }, hasTouch: true });
        if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        let failMethod: string | null = null;
        let creates = 0;
        let applies = 0;
        const reads: Array<{ source: string; refresh?: boolean }> = [];
        await context.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          server.onMessage((raw) => socket.send(raw));
          socket.onMessage((raw) => {
            const request = JSON.parse(String(raw));
            if (request.method === 'refloat.configuration.read') reads.push(request.params);
            if (request.method === 'refloat.tune.create') creates++;
            if (request.method === 'refloat.tune.apply') applies++;
            if (request.method === failMethod) {
              socket.send(
                JSON.stringify({
                  id: request.id,
                  error: { code: 'invalid_state', message: 'Preset test failure' },
                })
              );
              return;
            }
            server.send(raw);
          });
        });
        const page = await context.newPage();
        page.setDefaultTimeout(5000);
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        await page.goto(`${endpoint.origin}/`);
        await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
        await page.getByRole('button', { name: 'Tunes…', exact: true }).tap();
        await page.getByRole('button', { name: 'Save current tune…', exact: true }).tap();
        const dialog = page.getByRole('alertdialog', { name: 'Save current board Tune?' });
        await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Base');
        const save = dialog.getByRole('button', { name: 'Save preset', exact: true });
        await save.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await save.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(creates, 0);
        for (const method of ['refloat.configuration.read', 'refloat.tune.create']) {
          failMethod = method;
          await save.tap();
          await dialog.getByRole('alert').waitFor();
          assert.equal(await dialog.getByRole('alert').textContent(), 'Preset test failure');
          await page.waitForFunction(() =>
            Array.from(document.querySelectorAll('[role="alertdialog"] button')).some(
              (button) => button.textContent === 'Save preset' && !button.hasAttribute('disabled')
            )
          );
          assert.equal(await dialog.getByRole('textbox', { name: 'Name', exact: true }).inputValue(), 'Base');
        }
        failMethod = null;
        await save.focus();
        await page.keyboard.press('Enter');
        await dialog.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: /Base.*Active/ }).waitFor();
        assert.equal(creates, 2, 'keyboard/touch activation duplicated a save');
        assert.ok(reads.every((read) => read.source === 'current' && read.refresh === true));
        await page.reload();
        await page.getByRole('button', { name: /Base/ }).waitFor();

        const current = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
        const listed = await client.request('refloat.tune.list', { identity: current.identity, limit: 50 });
        const base = await client.request('refloat.tune.get', { tuneId: listed.items[0]!.tuneId });
        await client.request('refloat.tune.create', {
          name: 'Cruise',
          description: null,
          identity: current.identity,
          sourceRevision: current.revision,
          values: { ...base.values, 'leds.on': { type: 'boolean', value: false } },
        });
        await client.waitForState((state) => state.activeConnection?.latestSample?.rideState === 'riding');
        await page.reload();
        const cruise = page.getByRole('button', { name: /Cruise/ });
        await cruise.waitFor();
        await cruise.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await cruise.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(applies, 0);
        await cruise.scrollIntoViewIfNeeded();
        const bounds = await cruise.boundingBox();
        assert.ok(bounds);
        const x = bounds.x + bounds.width / 2;
        const y = bounds.y + bounds.height / 2;
        await cruise.evaluate((element) => {
          element.addEventListener(
            'pointerdown',
            (event) => {
              element.setAttribute('data-test-pointer-id', String((event as PointerEvent).pointerId));
            },
            { once: true }
          );
        });
        await page.mouse.move(x, y);
        await page.mouse.down();
        await cruise.dispatchEvent('pointercancel', {
          pointerId: Number(await cruise.getAttribute('data-test-pointer-id')),
        });
        await page.mouse.up();
        assert.equal(applies, 0);
        await page.mouse.move(x, y);
        await page.mouse.down();
        await page.mouse.move(x, y + 25);
        await page.mouse.up();
        assert.equal(applies, 0, 'scrolling must not apply a Tune');
        await cruise.focus();
        await page.keyboard.press('Enter');
        await page.getByRole('status').filter({ hasText: 'Cruise was applied and verified.' }).waitFor();
        assert.equal(applies, 1, 'keyboard activation must apply a Tune once while riding');
        assert.equal(await page.getByRole('alertdialog').count(), 0);
        assert.equal(await page.getByRole('button', { name: 'Apply preset', exact: true }).count(), 0);
        const applied = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
        assert.deepEqual(applied.values['leds.on'], { type: 'boolean', value: false });
        await cruise.tap();
        await page.getByRole('button', { name: /Cruise.*Active/ }).waitFor();
        assert.equal(applies, 1, 'tapping the active Tune must not rewrite it');
        await page.getByRole('button', { name: /Base/ }).tap();
        await page.getByRole('status').filter({ hasText: 'Base was applied and verified.' }).waitFor();
        assert.equal(applies, 2, 'one touch must switch back to the saved Tune');
        failMethod = 'refloat.tune.apply';
        await cruise.tap();
        await page.getByRole('dialog', { name: 'Tunes', exact: true }).getByRole('alert').waitFor();
        assert.equal(applies, 3);
        assert.equal(await page.getByRole('button', { name: /Cruise.*Active/ }).count(), 0);
        failMethod = null;
        await cruise.tap();
        await page.getByRole('status').filter({ hasText: 'Cruise was applied and verified.' }).waitFor();
        assert.equal(applies, 4, 'a failed switch must be retryable with one tap');
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: Tune save shows failures and persists a fresh board preset`, { timeout: 60000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', { deviceId: scan.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const context = await browser.newContext({ viewport: { width: 390, height: 700 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      let failMethod: string | null = null;
      let creates = 0;
      const reads: Array<{ source: string; refresh?: boolean }> = [];
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => socket.send(raw));
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'refloat.configuration.read') reads.push(request.params);
          if (request.method === 'refloat.tune.create') creates++;
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
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import {
  startMockServer,
  stopMockServer,
  getMockBootstrap,
  simulateMockForeground,
} from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: automatic connections preserve restored screens and the board toggle saves once`,
    { timeout: 60_000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        simulateMockForeground(false);
        const initial = await client.request('state.get', {});
        await client.request('settings.update', {
          expectedRevision: initial.settings.revision,
          patch: { autoConnectSavedDeviceId: 'saved-board' },
        });
        await client.request('uiState.set', {
          checkpoint: { version: 1, route: { kind: 'settings' }, live: null, surface: null },
        });
        const page = await browser.newPage({
          hasTouch: true,
          viewport: { width: 390, height: 844 },
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(5_000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        const methods: string[] = [];
        let rejectSave = false;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const request = JSON.parse(String(raw));
            methods.push(request.method);
            if (rejectSave && request.method === 'settings.update')
              socket.send(
                JSON.stringify({ id: request.id, error: { code: 'unavailable', message: 'Test save failure' } })
              );
            else server.send(raw);
          });
        });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        // Connection is admitted before checkpoint loading; restoration must still win.
        simulateMockForeground(true);
        await page.goto(endpoint.origin);
        await page.waitForURL('**/#/settings');
        assert.equal(methods.includes('device.connect'), false);
        assert.equal((await client.request('state.get', {})).activeConnection?.trackingMode, 'saved');
        await page.goto(`${endpoint.origin}/#/sessions/boards/saved-board`);
        await page.getByRole('button', { name: 'Board actions', exact: true }).click();
        await page.getByRole('button', { name: 'Configure board', exact: true }).click();
        const toggle = page.getByRole('switch', { name: 'Connect automatically', exact: true });
        await toggle.waitFor();
        assert.equal(await toggle.getAttribute('aria-checked'), 'true');
        const count = () => methods.filter((method) => method === 'settings.update').length;
        const before = count();
        await toggle.click({ button: 'right' });
        await toggle.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 9 });
        await toggle.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 9 });
        assert.equal(count(), before);
        await toggle.tap();
        await page.waitForFunction(
          () => document.querySelector('[role="switch"]')?.getAttribute('aria-checked') === 'false'
        );
        assert.equal(count(), before + 1);
        await toggle.focus();
        await page.keyboard.press('Space');
        await page.waitForFunction(
          () => document.querySelector('[role="switch"]')?.getAttribute('aria-checked') === 'true'
        );
        assert.equal(count(), before + 2);
        rejectSave = true;
        await toggle.tap();
        await page.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
        assert.equal(await toggle.getAttribute('aria-checked'), 'true');
        await page.close();
      } finally {
        client.close();
        await browser.close();
        await stopMockServer();
      }
    }
  );
}

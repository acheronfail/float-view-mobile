import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: current ride follows, freezes for inspection and finishes without disconnect navigation`,
    { timeout: 90_000 },
    async () => {
      const port = await startMockServer(11002);
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scanned = await client.waitForState((state) => state.discoveredDevices.length > 0);
        const deviceId = scanned.discoveredDevices[0]!.deviceId;
        await client.request('device.connect', { deviceId, trackingMode: 'saved' });
        const ready = await client.waitForState((state) => state.activeConnection?.state === 'ready');
        const sessionId = ready.activeConnection!.activeSession!.sessionId;
        // Retain even a short test ride.
        await client.request('session.marker.add', { sessionId, operationId: crypto.randomUUID() });
        const page = await browser.newPage({
          hasTouch: name === 'webkit',
          reducedMotion: 'reduce',
          viewport: name === 'webkit' ? { width: 390, height: 844 } : { width: 1000, height: 1000 },
        });
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        page.setDefaultTimeout(10000);
        const errors: string[] = [];
        page.on('pageerror', (error) => errors.push(error.message));
        let metadataRequests = 0;
        let disconnectRequests = 0;
        let reconnecting = false;
        let failRefresh = false;
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'session.get') {
              metadataRequests++;
              if (failRefresh) {
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'internal_error', message: 'Test refresh failure' } })
                );
                return;
              }
            }
            if (message.method === 'device.disconnect') disconnectRequests++;
            server.send(raw);
          });
          server.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            const state = message.data ?? message.result;
            if (reconnecting && state?.activeConnection) state.activeConnection.state = 'reconnecting';
            socket.send(JSON.stringify(message));
          });
        });
        const ends: number[] = [];
        await page.route('**/api/session-data', async (route) => {
          const request = route.request().postDataJSON();
          if (request.kind === 'series') ends.push(request.range.endElapsedMs);
          await route.continue();
        });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
        await page.goto(`${endpoint.origin}/#/live`);
        const menu = page.getByRole('button', { name: 'Board actions', exact: true });
        assert.equal(await page.getByRole('button', { name: /Current ride/ }).count(), 0);
        await menu.click();
        const current = page.getByRole('button', { name: 'Charts…', exact: true });
        await current.waitFor();
        await current.dispatchEvent('pointerdown', { pointerId: 11, isPrimary: false, button: 0 });
        await current.dispatchEvent('pointerup', { pointerId: 11, isPrimary: false, button: 0 });
        const currentBounds = (await current.boundingBox())!;
        await page.mouse.move(currentBounds.x + 20, currentBounds.y + 20);
        await page.mouse.down();
        await current.dispatchEvent('pointercancel', { pointerId: 1, isPrimary: true });
        await page.mouse.up();
        await page.mouse.move(0, 0);
        assert.match(page.url(), /#\/live$/);
        if (name === 'webkit') await current.tap();
        else {
          await current.focus();
          await page.keyboard.press('Enter');
        }
        await page.getByText('Recording · Following live', { exact: true }).waitFor();
        await page.waitForFunction(() => document.querySelectorAll('[data-history-group] svg').length > 0);
        const firstEnd = ends[0]!;
        await page.waitForTimeout(3400);
        assert(
          ends.some((end) => end > firstEnd),
          'growing snapshots should extend charts'
        );
        const legend = page.getByRole('button', { name: /^Hide .+ line$/ }).first();
        // Capture an element identity to prove refresh does not recreate the chart.
        await page
          .locator('[data-history-group] svg')
          .first()
          .evaluate((node) => node.setAttribute('data-preserved', 'yes'));
        const chart = page.locator('[data-history-group] svg').first();
        const box = (await chart.boundingBox())!;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.down();
        await page.mouse.up();
        await page.getByRole('button', { name: 'Jump to live', exact: true }).waitFor();
        const pausedRequests = metadataRequests;
        await page.waitForTimeout(3400);
        assert.equal(metadataRequests, pausedRequests);
        assert.equal(await chart.getAttribute('data-preserved'), 'yes');
        const hiddenLabel = (await legend.getAttribute('aria-label'))!.replace(/^Hide /, 'Show ');
        await legend.click();
        const showLegend = page.getByRole('button', { name: hiddenLabel, exact: true });
        await showLegend.waitFor();
        await page.getByRole('button', { name: 'Jump to live', exact: true }).click();
        await page.getByText('Recording · Following live', { exact: true }).waitFor();
        await page.waitForTimeout(500);
        assert(metadataRequests > pausedRequests);
        assert.equal(await chart.getAttribute('data-preserved'), 'yes');
        reconnecting = true;
        await page.getByText('Reconnecting · Following live', { exact: true }).waitFor();
        reconnecting = false;
        await page.getByText('Recording · Following live', { exact: true }).waitFor();
        await showLegend.waitFor();
        failRefresh = true;
        await page.getByText(/Could not refresh this ride/).waitFor();
        assert(await chart.isVisible());
        failRefresh = false;
        await page.getByText(/Could not refresh this ride/).waitFor({ state: 'hidden' });
        await page.getByRole('link', { name: 'Back to Live', exact: true }).click();
        await menu.click();
        await current.waitFor();
        const afterReturn = metadataRequests;
        await page.waitForTimeout(3400);
        assert.equal(metadataRequests, afterReturn);
        assert.equal(disconnectRequests, 0);
        assert.equal((await client.request('state.get', {})).activeConnection?.activeSession?.sessionId, sessionId);
        await page.evaluate(() => {
          location.hash = '#/sessions';
        });
        await page.getByRole('button', { name: /Current ride/ }).click();
        await page.getByText('Recording · Following live', { exact: true }).waitFor();
        await client.request('device.disconnect', {});
        await page.getByText('Recording · Following live', { exact: true }).waitFor({ state: 'hidden' });
        await page.waitForTimeout(3500);
        assert.match(page.url(), new RegExp(sessionId));
        assert.equal((await client.request('session.get', { sessionId })).summary.state, 'completed');
        assert.equal(await page.getByRole('heading', { name: 'Current ride', exact: true }).count(), 0);
        const completedRequests = metadataRequests;
        await page.waitForTimeout(3400);
        assert.equal(metadataRequests, completedRequests);
        await client.request('device.connect', { deviceId, trackingMode: 'guest' });
        await client.waitForState((state) => state.activeConnection?.state === 'ready');
        await page.goto(`${endpoint.origin}/#/live`);
        await page.getByText('Guest connection — live data will not be saved or tracked.').waitFor();
        await menu.click();
        assert.equal(await current.count(), 0);
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}

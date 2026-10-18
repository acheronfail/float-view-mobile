import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: recorded firmware, paginated reconnects and older sessions`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
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
      const sessionId = ready.activeConnection!.activeSession!.sessionId;
      const page = await browser.newPage({ hasTouch: true, viewport: { width: 390, height: 844 } });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.setDefaultTimeout(10_000);
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.summary?.sessionId === sessionId && message.result.snapshot) {
            message.result.connectionContextCount = 65;
            message.result.snapshot.committedEventSequence = 1000;
          }
          socket.send(JSON.stringify(message));
        });
      });
      let contextRequests = 0;
      await page.route('**/api/session-data', async (route) => {
        const request = route.request().postDataJSON();
        if (request.kind !== 'connectionContexts') return route.continue();
        contextRequests++;
        const start = request.cursor ? 64 : 0;
        const firmware = { major: 6, minor: 6, testVersion: 2, hardwareName: 'Thor300', firmwareName: null };
        await route.fulfill({
          json: {
            kind: request.kind,
            sessionId: request.sessionId,
            snapshot: request.snapshot,
            range: request.range,
            contexts: Array.from({ length: start ? 1 : 64 }, (_, i) => ({
              sequence: start + i + 1,
              elapsedMs: 0,
              observedAtMs: 1000,
              context: {
                version: 1,
                refloatVersion: '1.3.0',
                controller: firmware,
                transport: { ...firmware, major: 6, minor: 5, firmwareName: 'VESC Express' },
                connection: { kind: 'can', canId: start ? 43 : 42 },
              },
            })),
            nextCursor: start ? null : 'page-2',
          },
        });
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/sessions/${sessionId}`);
      const section = page.getByRole('region', { name: 'Connection history' });
      await section
        .getByText('Refloat 1.3.0 · Controller VESC 6.06 (test 2)', { exact: true })
        .first()
        .waitFor()
        .catch(async (error) => {
          console.error(await page.locator('body').innerText(), errors);
          throw error;
        });
      assert.equal(await section.getByText('Via VESC Express → CAN 42', { exact: true }).count(), 64);
      const next = section.getByRole('button', { name: 'Next connections', exact: true });
      const before = contextRequests;
      await next.click({ button: 'right' });
      await page.waitForTimeout(100);
      assert.equal(contextRequests, before);
      await next.focus();
      await page.keyboard.press('Enter');
      await section.getByText('Via VESC Express → CAN 43', { exact: true }).waitFor();
      assert.equal(await section.locator('li').count(), 1);
      const first = section.getByRole('button', { name: 'First connections', exact: true });
      if (name === 'webkit') await first.tap();
      else await first.click();
      await section.getByText('First recorded connection · 0:00', { exact: true }).waitFor();
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      await page
        .getByRole('region', { name: 'Connection history' })
        .getByText('Not recorded', { exact: true })
        .waitFor();
      assert.equal(await page.getByText('Via VESC Express → CAN 42', { exact: true }).count(), 0);
      assert.deepEqual(errors, []);
    } finally {
      client.close();
      await browser.close();
      await stopMockServer();
    }
  });
}
